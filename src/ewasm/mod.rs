mod abi;
mod codegen;
mod contract;
mod expressions;
mod function;
mod function_context;
pub mod preprocessor;
mod statements;
mod structs;
mod types;
mod utils;

use inkwell::context::Context as LLVMContext;
use inkwell::passes::PassManager;

use crate::ast::{
    AssetDeclaration, ContractBehaviourDeclaration, Module, StructDeclaration, TopLevelDeclaration,
    TraitDeclaration,
};
use crate::context::Context;
use crate::ewasm::abi::generate_abi;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::contract::LLVMContract;
use itertools::Itertools;
use nom::lib::std::collections::HashMap;
use process::Command;
use regex::Regex;
use std::error::Error;
use std::io::Write;
use std::{fs, path::Path, process};
use wabt::wasm2wat;
use wabt::wat2wasm;

const JSON_LCC_KEY: &str = "llcPath";
const JSON_WASM_LD_KEY: &str = "wasm-ldPath";
const TEMPORARY_DIRECTORY_NAME: &str = "tmp";
const OUTPUT_DIRECTORY_NAME: &str = "output";

pub fn generate(module: &Module, context: &mut Context) {
    let external_traits = module
        .declarations
        .iter()
        .filter_map(|d| {
            if let TopLevelDeclaration::TraitDeclaration(t) = d {
                Some(t)
            } else {
                None
            }
        })
        .collect::<Vec<&TraitDeclaration>>();

    let ewasm_contracts = module
        .declarations
        .iter()
        .filter_map(|dec| {
            if let TopLevelDeclaration::ContractDeclaration(contract_declaration) = dec {
                let contract_behaviour_declarations = module
                    .declarations
                    .iter()
                    .filter_map(|dec| {
                        if let TopLevelDeclaration::ContractBehaviourDeclaration(cbd) = dec {
                            if cbd.identifier.token == contract_declaration.identifier.token {
                                return Some(cbd);
                            }
                        }
                        None
                    })
                    .collect::<Vec<&ContractBehaviourDeclaration>>();

                let struct_declarations = module
                    .declarations
                    .iter()
                    .filter_map(|d| {
                        if let TopLevelDeclaration::StructDeclaration(s) = d {
                            Some(s)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<&StructDeclaration>>();

                // Are assets applicable to eWASM?
                let asset_declarations = module
                    .declarations
                    .iter()
                    .filter_map(|d| {
                        if let TopLevelDeclaration::AssetDeclaration(a) = d {
                            Some(a)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<&AssetDeclaration>>();

                let ewasm_contract = LLVMContract {
                    contract_declaration,
                    contract_behaviour_declarations,
                    struct_declarations,
                    asset_declarations,
                    external_traits: external_traits.clone(), // Only cloning the references but is there a better way?
                    environment: &context.environment,
                };

                Some(ewasm_contract)
            } else {
                None
            }
        })
        .collect::<Vec<LLVMContract>>();

    assert!(!ewasm_contracts.is_empty());

    // It seems from the move target that we are allowed to create multiple contracts, and each of these
    // gets put into a separate file, so we will do the same here
    let output_path = Path::new(OUTPUT_DIRECTORY_NAME);
    if !output_path.exists() {
        fs::create_dir(output_path).expect("Could not create output directory");
    }

    let flint_config =
        fs::read_to_string(Path::new("flint_config.json")).expect("Could not find flint config");
    let flint_config = json::parse(flint_config.as_str()).expect("Could not parse flint config");
    assert!(flint_config.is_object());
    let llc_path = flint_config
        .entries()
        .find(|(key, _)| *key == JSON_LCC_KEY)
        .expect("No llc in flint config")
        .1
        .as_str()
        .expect("Could not find llc in flint config");
    assert!(!llc_path.is_empty());

    let wasm_ld_path = flint_config
        .entries()
        .find(|(key, _)| *key == JSON_WASM_LD_KEY)
        .expect("No wasm-ld in flint config")
        .1
        .as_str()
        .expect("Could not find wasm-ld path in flint config");
    assert!(!wasm_ld_path.is_empty());

    for contract in ewasm_contracts.iter() {
        let tmp_path = Path::new(TEMPORARY_DIRECTORY_NAME);
        if !tmp_path.exists() {
            fs::create_dir(tmp_path).expect("Could not create temporary directory");
        }

        let file_name = contract.contract_declaration.identifier.token.as_str();
        let get_path = |folder: &str, ext: &str| format!("{}/{}.{}", folder, file_name, ext);

        // Create LLVM file
        create_and_write_to_file(
            Path::new(get_path(TEMPORARY_DIRECTORY_NAME, "ll").as_str()),
            &*generate_llvm(contract).as_bytes(),
        )
        .expect("Could not create file");

        // Convert LLVM to wasm32:
        Command::new(llc_path)
            .arg("-O3")
            .arg("-march=wasm32")
            .arg("-filetype=obj")
            .arg(get_path(TEMPORARY_DIRECTORY_NAME, "ll"))
            .status()
            .expect("Could not compile to WASM");

        // Link externally defined functions
        Command::new(wasm_ld_path)
            .arg("--no-entry")
            .arg("--export-all")
            .arg("--allow-undefined")
            .arg("-o")
            .arg(get_path(TEMPORARY_DIRECTORY_NAME, "wasm"))
            .arg(get_path(TEMPORARY_DIRECTORY_NAME, "o"))
            .status()
            .expect("Could not link externally defined methods");

        // The following only exists so that we can inspect LLVM output and wasm files as wat files
        // while developing, and should be removed. TODO
        let wasm = fs::read(Path::new(
            get_path(TEMPORARY_DIRECTORY_NAME, "wasm").as_str(),
        ))
        .expect("Could not read wasm");
        let mut as_wat = wasm2wat(wasm).expect("Could not convert wasm to wat");

        // Shift final module closing curly brace onto its own line so it is not removed when trimming exports
        as_wat.insert(as_wat.len() - 2, '\n');
        // Remove exports except memory and main
        let export_regex = Regex::new("export \"((main)|(memory))\"").unwrap();

        let as_wat = as_wat
            .lines()
            .filter(|line| !line.contains("export") || export_regex.is_match(line))
            .intersperse("\n")
            .collect::<String>();

        create_and_write_to_file(
            Path::new(get_path(TEMPORARY_DIRECTORY_NAME, "wat").as_str()),
            &as_wat.as_bytes(),
        )
        .expect("Could not create temporary wat file");

        let post_processed_wasm =
            wat2wasm(as_wat.as_bytes()).expect("Could not convert wat to wasm");
        create_and_write_to_file(
            Path::new(get_path(OUTPUT_DIRECTORY_NAME, "wasm").as_str()),
            &post_processed_wasm,
        )
        .expect("Could not write to output wasm file");

        // TODO remove this for the final release, as we do not need to give wat and wasm files
        fs::copy(
            Path::new(get_path(TEMPORARY_DIRECTORY_NAME, "wat").as_str()),
            Path::new(get_path(OUTPUT_DIRECTORY_NAME, "wat").as_str()),
        )
        .expect("Could not copy wat file from temporary dir to output dir");

        // TODO remove this for the final release, but for testing the LLVM, we need to be able to read the LLVM files
        fs::copy(
            Path::new(get_path(TEMPORARY_DIRECTORY_NAME, "ll").as_str()),
            Path::new(get_path(OUTPUT_DIRECTORY_NAME, "ll").as_str()),
        )
        .expect("Could not copy ll file from temporary dir to output dir");

        // Delete all tmp files
        fs::remove_dir_all(tmp_path).expect("Could not remove temporary directory");

        // Generate the ABI
        create_and_write_to_file(
            Path::new(get_path(OUTPUT_DIRECTORY_NAME, "json").as_str()),
            &*generate_abi(&contract.contract_behaviour_declarations).as_bytes(),
        )
        .expect("Could not generate abi file");
    }
}

fn create_and_write_to_file(path: &Path, data: &[u8]) -> Result<(), Box<dyn Error + 'static>> {
    Ok(fs::File::create(path)?.write_all(data)?)
}

fn generate_llvm(contract: &LLVMContract) -> String {
    // The following is a little confusing from a Rust perspective, because all of these things have
    // references to each other, so changing one changes all the others. Not only this, but they need
    // not be declared mutable either. The reason is that all these things are wrappers around C++
    // objects, and so rust does not understand that they interact, nor that we are mutating them
    let llvm_context = LLVMContext::create();
    let llvm_module = llvm_context.create_module("contract");
    let builder = llvm_context.create_builder();

    // The fpm will optimise our functions using the LLVM optimisations that we choose to add
    let fpm = PassManager::create(&llvm_module);
    // Add more of the available optimisations.
    // These are commented out while developing, since it is easier to see exactly what is generated
    // without optimisations

    /*
    fpm.add_verifier_pass();
    fpm.add_instruction_combining_pass();
    fpm.add_reassociate_pass();
    fpm.add_gvn_pass();
    fpm.add_promote_memory_to_register_pass();
    fpm.add_cfg_simplification_pass();
    */

    fpm.initialize();

    let mut codegen = Codegen {
        contract_name: contract.contract_declaration.identifier.token.as_str(),
        context: &llvm_context,
        module: &llvm_module,
        builder: &builder,
        fpm: &fpm,
        types: HashMap::new(),
    };

    build_empty_main_function(&codegen);

    contract.generate(&mut codegen);
    llvm_module.print_to_string().to_string()
}

// This simply creates an empty main method, since eWASM requires a main that does not have
// inputs or outputs
fn build_empty_main_function(codegen: &Codegen) {
    let void_type = codegen.context.void_type().fn_type(&[], false);
    let main = codegen.module.add_function("main", void_type, None);
    let block = codegen.context.append_basic_block(main, "entry");
    codegen.builder.position_at_end(block);
    codegen.builder.build_return(None);
    codegen.verify_and_optimise(&main);
}

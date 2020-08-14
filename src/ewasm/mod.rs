mod call;
mod codegen;
mod contract;
mod declaration;
mod expressions;
mod function;
mod function_context;
mod identifier;
pub mod preprocessor;
mod statements;
mod structs;
mod types;
mod literal;

extern crate inkwell;

use self::inkwell::context::Context as LLVMContext;

use self::inkwell::passes::PassManager;

use crate::ast::{
    AssetDeclaration, ContractBehaviourDeclaration, Module, StructDeclaration, TopLevelDeclaration,
    TraitDeclaration,
};
use crate::context::Context;

use crate::ewasm::codegen::Codegen;
use crate::ewasm::contract::LLVMContract;
use nom::lib::std::collections::HashMap;
use std::io::Write;
use std::{fs, path, process};

// TODO create ABI JSON struct? (remember we also need to generate the ABI)

pub fn generate(module: &Module, context: &Context) {
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
    for contract in ewasm_contracts.iter() {
        let _contract_file = create_llvm_file(contract);

        // TODO use llvm tools to compile _contract_file to WASM, then remove exports etc and
        // probably use WABT tools to verify it etc.
        // Also create the ABI file
        // remove the temporary ewasm file
    }
}

fn create_llvm_file(contract: &LLVMContract) -> fs::File {
    let path = path::Path::new("tmp/llvm_ir_contract.ll");
    let mut file = fs::File::create(path).unwrap_or_else(|err| {
        println!(
            "Could not create file {}: {}",
            path.display(),
            err.to_string()
        );
        process::exit(1);
    });

    let llvm_module = generate_llvm(contract);

    file.write_all(llvm_module.as_bytes())
        .unwrap_or_else(|err| {
            exit_on_failure(
                format!(
                    "Could not write to file {}: {}",
                    path.display(),
                    err.to_string()
                )
                .as_str(),
            )
        });

    file
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
    // TODO add more of the available optimisations
    fpm.add_verifier_pass();
    fpm.initialize();

    let mut codegen = Codegen {
        context: &llvm_context,
        module: &llvm_module,
        builder: &builder,
        fpm: &fpm,
        types: HashMap::new(),
    };

    // Since all mutation happens in C++, (below Rust) we need not mark codegen as mutable
    contract.generate(&mut codegen);
    llvm_module.print_to_string().to_string()
}

fn exit_on_failure(msg: &str) -> ! {
    println!("{}", msg);
    process::exit(1)
}

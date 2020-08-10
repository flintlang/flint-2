mod codegen;

extern crate inkwell;

use self::inkwell::builder::Builder;
use self::inkwell::context::Context as LLVMContext;
use self::inkwell::module::Module as LLVMModule;
use self::inkwell::passes::PassManager;
use self::inkwell::values::FunctionValue;
use crate::ast::Module;
use crate::context::Context;

use std::io::Write;
use std::{fs, path, process};

struct Codegen<'a, 'ctx> {
    pub llvm_context: &'ctx LLVMContext,
    pub llvm_module: &'a LLVMModule<'ctx>,
    pub builder: &'a Builder<'ctx>,
    pub fpm: &'a PassManager<FunctionValue<'ctx>>,
    pub flint_context: &'a Context,
    pub ast: &'a Module,
}

// TODO create ABI JSON struct?

fn generate_llvm(module: &Module, context: &mut Context) -> String {
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
        llvm_context: &llvm_context,
        llvm_module: &llvm_module,
        builder: &builder,
        fpm: &fpm,
        flint_context: context,
        ast: &module,
    };

    codegen.generate();
    llvm_module
        .print_to_string()
        .to_string()
}

fn create_llvm_file(module: &Module, context: &mut Context) -> fs::File {
    let path = path::Path::new("tmp/llvm_ir_contract.ll");
    let mut file = fs::File::create(path).unwrap_or_else(|err| {
        println!(
            "Could not create file {}: {}",
            path.display(),
            err.to_string()
        );
        process::exit(1);
    });

    let llvm_module = generate_llvm(module, context);

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

fn exit_on_failure(msg: &str) -> ! {
    println!("{}", msg);
    process::exit(1)
}

pub fn generate(module: &Module, context: &mut Context) {
    let _file = create_llvm_file(module, context);
    // TODO use llvm tools to compile _file to WASM, then remove exports etc and
    // TODO probably use WABT tools to verify it etc.
    // TODO Also create the ABI file
    // TODO remove the temporary llvm file
}

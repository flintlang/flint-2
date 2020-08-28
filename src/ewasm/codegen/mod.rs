extern crate inkwell;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::passes::PassManager;
use inkwell::types::StructType;
use inkwell::values::FunctionValue;
use std::collections::HashMap;

pub mod imports;
pub mod runtime_functions;

pub struct Codegen<'a, 'ctx> {
    pub contract_name: &'a str,
    pub context: &'ctx Context,
    pub module: &'a Module<'ctx>,
    pub builder: &'a Builder<'ctx>,
    pub fpm: &'a PassManager<FunctionValue<'ctx>>,
    pub types: HashMap<String, (Vec<String>, StructType<'ctx>)>,
}

impl<'a, 'ctx> Codegen<'a, 'ctx> {
    pub fn verify_and_optimise(&self, func: &FunctionValue) {
        // False means it does not print to stdoutput why the function is invalid
        if func.verify(true) {
            self.fpm.run_on(func);
        } else {
            self.module.print_to_stderr();
            panic!(
                "Invalid function `{}`",
                func.get_name()
                    .to_str()
                    .unwrap_or("<could not convert func name to str>")
            );
        }
    }
}

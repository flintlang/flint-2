use super::inkwell::attributes::AttributeLoc;
use super::inkwell::module::Linkage;
use super::inkwell::types::BasicType;
use super::inkwell::values::{BasicValue, FunctionValue};
use crate::ewasm::Codegen;

// This file will likely serve as a 'utils' file, where things generic to code generation can go

impl<'a, 'ctx> Codegen<'a, 'ctx> {
    // For now this will just set up what will often likely need, i.e. getCaller
    // TODO have other std library methods linked as they are used
    // TODO alternatively, we could link everything, and unused things will be optimised out?
    pub fn ether_imports(&self) {
        let import_linkage_attribute = self
            .context
            .create_string_attribute("wasm-import-module", "ethereum");
        // Declare getCaller:
        // Takes an i32 input param pointing to where in memory to load the address of the caller
        // Returns nothing
        let param_type = self.context.i32_type().as_basic_type_enum();
        let return_type = self.context.void_type().fn_type(&[param_type], false);
        let get_caller_extern =
            self.module
                .add_function("getCaller", return_type, Some(Linkage::External));
        get_caller_extern
            .get_first_param()
            .as_mut()
            .map(|param| param.set_name("resultOffset"));

        // The following attributes tell LLVM how to link to ethereum
        get_caller_extern.add_attribute(AttributeLoc::Function, import_linkage_attribute);
        let function_import_name = self
            .context
            .create_string_attribute("wasm-import-name", "getCaller");
        get_caller_extern.add_attribute(AttributeLoc::Function, function_import_name);

        self.verify_and_optimise(&get_caller_extern);
    }

    pub fn verify_and_optimise(&self, func: &FunctionValue) {
        // False means it does not print to stdoutput why the function is invalid
        if func.verify(false) {
            self.fpm.run_on(func);
        } else {
            panic!(
                "Invalid function `{}`",
                func.get_name()
                    .to_str()
                    .unwrap_or("<could not convert func name to str>")
            );
        }
    }
}

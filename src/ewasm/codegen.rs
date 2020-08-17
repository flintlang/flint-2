use super::inkwell::attributes::AttributeLoc;
use super::inkwell::builder::Builder;
use super::inkwell::context::Context as LLVMContext;
use super::inkwell::module::Linkage;
use super::inkwell::module::Module as LLVMModule;
use super::inkwell::passes::PassManager;
use super::inkwell::types::{BasicType, StructType};
use super::inkwell::values::{BasicValue, FunctionValue, PointerValue};
use std::collections::HashMap;

pub struct Codegen<'a, 'ctx> {
    pub context: &'ctx LLVMContext,
    pub module: &'a LLVMModule<'ctx>,
    pub builder: &'a Builder<'ctx>,
    pub fpm: &'a PassManager<FunctionValue<'ctx>>,
    // I THINK we need this to keep track of user defined types.
    pub types: HashMap<String, (Vec<String>, StructType<'ctx>)>,
}

impl<'a, 'ctx> Codegen<'a, 'ctx> {
    // For now this will just set up what will often likely need, i.e. getCaller
    // TODO have other std library methods linked as they are used
    // alternatively, we could link everything, and unused things will be optimised out?
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

        get_caller_extern.get_first_param().as_mut().map(|param| {
            param.set_name("resultOffset");
            param
        });

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
            self.module.print_to_stderr();
            panic!(
                "Invalid function `{}`",
                func.get_name()
                    .to_str()
                    .unwrap_or("<could not convert func name to str>")
            );
        }
    }

    pub fn build_struct_member_getter(
        &self,
        struct_type_name: &str,
        field_name: &str,
        the_struct: PointerValue<'ctx>,
        name_to_assign: &str,
    ) -> PointerValue<'ctx> {
        // PRE: the struct has been defined, and the_struct is a pointer to an instance of it
        let (field_names, _) = self.types.get(struct_type_name).unwrap();
        self.builder
            .build_struct_gep(
                the_struct,
                field_names
                    .iter()
                    .position(|name| name == field_name)
                    .unwrap() as u32,
                name_to_assign,
            )
            .unwrap()
    }
}

extern crate inkwell;

use crate::ewasm::codegen::Codegen;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::module::Linkage;
use inkwell::types::{BasicType, FunctionType};
use inkwell::values::BasicValue;
use inkwell::AddressSpace;

// For now this will just set up what will often likely need, i.e. getCaller and revert
// TODO have other std library methods linked as they are used
// alternatively, we could link everything, and unused things will be optimised out?
impl<'a, 'ctx> Codegen<'a, 'ctx> {
    pub fn ether_imports(&self) {
        // self.import_get_caller(); // Using a dummy getCaller while we are testing TODO remove for release
        self.import_revert();
        self.import_get_external_balance();
    }

    #[allow(dead_code)]
    fn import_get_caller(&self) {
        // Takes an i32 input param pointing to where in memory to load the address of the caller
        // Returns nothing
        let param_type = self
            .context
            .custom_width_int_type(160)
            .as_basic_type_enum()
            .ptr_type(AddressSpace::Generic)
            .as_basic_type_enum();

        let func_type = self.context.void_type().fn_type(&[param_type], false);
        self.generate_import_and_extern("getCaller", func_type, Some(vec!["resultOffset"]));
    }

    fn import_get_external_balance(&self) {
        // Takes an i32 input param pointing to where in memory to get the address of the caller, and an i32 input param pointing to where in memory to store the balance
        // Returns nothing
        let address_ptr = self
            .context
            .custom_width_int_type(160)
            .as_basic_type_enum()
            .ptr_type(AddressSpace::Generic)
            .as_basic_type_enum();

        let int_ptr = self
            .context
            .i128_type()
            .ptr_type(AddressSpace::Generic)
            .as_basic_type_enum();

        let func_type = self.context.void_type().fn_type(&[address_ptr, int_ptr], false);
        self.generate_import_and_extern("getExternalBalance", func_type, Some(vec!["addressOffset", "resultOffset"]));
    }

    fn import_revert(&self) {
        // Takes memory pointer for where output data is stored, and an int, saying how long the data is
        // Returns nothing
        let first_param_type = self
            .context
            .i32_type() // This may change since at the moment it means we can only expect to store an int as returning data
            .ptr_type(AddressSpace::Generic)
            .as_basic_type_enum();

        let second_param_type = self.context.i32_type().as_basic_type_enum();

        let param_names = vec!["dataOffset", "length"];

        let func_type = self
            .context
            .void_type()
            .fn_type(&[first_param_type, second_param_type], false);
        self.generate_import_and_extern("revert", func_type, Some(param_names));
    }

    fn generate_import_and_extern(
        &self,
        name: &str,
        func_type: FunctionType<'ctx>,
        param_names: Option<Vec<&str>>,
    ) {
        // Declare function
        let extern_declaration = self
            .module
            .add_function(name, func_type, Some(Linkage::External));

        if let Some(param_names) = param_names {
            for (index, param) in extern_declaration.get_param_iter().enumerate() {
                param.set_name(param_names[index]);
            }
        }

        // Add attributes to say where to link from
        let name_attribute = self
            .context
            .create_string_attribute("wasm-import-name", name);

        extern_declaration.add_attribute(AttributeLoc::Function, self.get_import_attribute());
        extern_declaration.add_attribute(AttributeLoc::Function, name_attribute);
        self.verify_and_optimise(&extern_declaration);
    }

    fn get_import_attribute(&self) -> Attribute {
        self.context
            .create_string_attribute("wasm-import-module", "ethereum")
    }
}

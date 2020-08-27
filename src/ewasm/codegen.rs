use super::inkwell::attributes::AttributeLoc;
use super::inkwell::builder::Builder;
use super::inkwell::context::Context as LLVMContext;
use super::inkwell::module::Linkage;
use super::inkwell::module::Module as LLVMModule;
use super::inkwell::passes::PassManager;
use super::inkwell::types::{BasicType, StructType};
use super::inkwell::values::{BasicValue, FunctionValue};
use super::inkwell::{AddressSpace, IntPredicate};
use std::collections::HashMap;

pub struct Codegen<'a, 'ctx> {
    pub contract_name: &'a str,
    pub context: &'ctx LLVMContext,
    pub module: &'a LLVMModule<'ctx>,
    pub builder: &'a Builder<'ctx>,
    pub fpm: &'a PassManager<FunctionValue<'ctx>>,
    pub types: HashMap<String, (Vec<String>, StructType<'ctx>)>,
}

impl<'a, 'ctx> Codegen<'a, 'ctx> {
    // For now this will just set up what will often likely need, i.e. getCaller
    // TODO have other std library methods linked as they are used
    // alternatively, we could link everything, and unused things will be optimised out?
    #[allow(dead_code)]
    pub fn ether_imports(&self) {
        let import_linkage_attribute = self
            .context
            .create_string_attribute("wasm-import-module", "ethereum");
        // Declare getCaller:
        // Takes an i32 input param pointing to where in memory to load the address of the caller
        // Returns nothing
        let param_type = self
            .context
            .custom_width_int_type(160)
            .as_basic_type_enum()
            .ptr_type(AddressSpace::Generic)
            .as_basic_type_enum();
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

    pub fn runtime_functions(&self) {
        self.get_caller();
        self.power();
    }

    pub fn get_caller(&self) {
        // Dummy implementation of the eWASM getCaller function
        let param_type = self
            .context
            .custom_width_int_type(160)
            .ptr_type(AddressSpace::Generic)
            .as_basic_type_enum();
        
        let func_type = self
            .context
            .void_type()
            .fn_type(&[param_type], false);

        let func_val = self.module.add_function("getCaller", func_type, None);
        let bb = self.context.append_basic_block(func_val, "entry");
        self.builder.position_at_end(bb);

        let memory_offset = func_val.get_params()[0].into_pointer_value();
        let address = self.context.custom_width_int_type(160).const_int(1, false);

        self.builder.build_store(memory_offset, address);
        self.builder.build_return(None);
    }

    pub fn power(&self) {
        // Integer exponent method. Naive implementation
        // PRE: a >= 0, b > 0 and a and b are integers
        let param_type = self.context.i64_type().as_basic_type_enum();
        let int_type = self
            .context
            .i64_type()
            .fn_type(&[param_type, param_type], false);
        let exp = self.module.add_function("_exp", int_type, None);
        let bb = self.context.append_basic_block(exp, "entry");
        self.builder.position_at_end(bb);

        let a = exp.get_params()[0].into_int_value();
        let b = exp.get_params()[1].into_int_value();

        let acc = self
            .builder
            .build_alloca(self.context.i64_type().as_basic_type_enum(), "acc");
        self.builder
            .build_store(acc, self.context.i64_type().const_int(1, false));

        let count = self
            .builder
            .build_alloca(self.context.i64_type().as_basic_type_enum(), "i");
        self.builder
            .build_store(count, self.context.i64_type().const_int(0, false));

        let loop_bb = self.context.append_basic_block(exp, "loop");
        let check_cond_bb = self.context.append_basic_block(exp, "check_cond");
        let end_bb = self.context.append_basic_block(exp, "end");

        self.builder.build_unconditional_branch(check_cond_bb);
        self.builder.position_at_end(loop_bb);

        let acc_loaded = self.builder.build_load(acc, "acc_load");
        let mul = self
            .builder
            .build_int_mul(acc_loaded.into_int_value(), a, "new_acc");
        self.builder.build_store(acc, mul.as_basic_value_enum());

        let count_loaded = self.builder.build_load(count, "count_load");
        let add = self.builder.build_int_add(
            count_loaded.into_int_value(),
            self.context.i64_type().const_int(1, false),
            "new_count",
        );
        self.builder.build_store(count, add.as_basic_value_enum());

        self.builder.build_unconditional_branch(check_cond_bb);

        self.builder.position_at_end(check_cond_bb);

        let count_loaded = self.builder.build_load(count, "count_load");
        let cond = self.builder.build_int_compare(
            IntPredicate::SLT,
            count_loaded.into_int_value(),
            b,
            "cond",
        );

        self.builder.build_conditional_branch(cond, loop_bb, end_bb);

        self.builder.position_at_end(end_bb);
        let loaded_acc = self.builder.build_load(acc, "result");
        self.builder.build_return(Some(&loaded_acc));

        self.verify_and_optimise(&exp);
    }

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

#[cfg(test)]
mod runtime_tests {
    use super::super::inkwell::context::Context;
    use super::super::inkwell::execution_engine::JitFunction;
    use super::super::inkwell::passes::PassManager;
    use super::super::inkwell::OptimizationLevel;
    use crate::ewasm::codegen::Codegen;
    use std::collections::HashMap;

    #[test]
    fn test_power() {
        let llvm_context = Context::create();
        let llvm_module = llvm_context.create_module("runtime_tests");
        let builder = llvm_context.create_builder();
        let fpm = PassManager::create(&llvm_module);

        fpm.initialize();

        let codegen = Codegen {
            contract_name: "runtime_tests",
            context: &llvm_context,
            module: &llvm_module,
            builder: &builder,
            fpm: &fpm,
            types: HashMap::new(),
        };

        codegen.power();

        let engine = codegen
            .module
            .create_jit_execution_engine(OptimizationLevel::None)
            .expect("Could not create execution engine");

        unsafe {
            let power_func: JitFunction<unsafe extern "C" fn(i64, i64) -> i64> = engine
                .get_function("_exp")
                .expect("Could not find function exp");

            assert_eq!(power_func.call(10, 0), 1);
            assert_eq!(power_func.call(10, 1), 10);
            assert_eq!(power_func.call(2, 3), 8);
            assert_eq!(power_func.call(0, 5), 0);
        }
    }
}

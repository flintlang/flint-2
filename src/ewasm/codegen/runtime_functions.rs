use crate::ewasm::codegen::Codegen;
use crate::ewasm::preprocessor::LLVMPreProcessor;
use inkwell::types::BasicType;
use inkwell::values::BasicValue;
use inkwell::values::BasicValueEnum;
use inkwell::values::InstructionOpcode;
use inkwell::{AddressSpace, IntPredicate};

impl<'a, 'ctx> Codegen<'a, 'ctx> {
    pub fn runtime_functions(&self) {
        self.get_caller();
        self.get_caller_wrapper();
        self.power();
        self.get_ethereum_internal();
    }

    fn get_caller_wrapper(&self) {
        let address_type = self.context.custom_width_int_type(160).as_basic_type_enum();

        let func_type = address_type.fn_type(&[], false);

        let func_val = self.module.add_function("_getCaller", func_type, None);

        let bb = self.context.append_basic_block(func_val, "entry");

        self.builder.position_at_end(bb);

        let address_type = self.context.custom_width_int_type(160).as_basic_type_enum();

        let memory_offset = self.builder.build_alloca(address_type, "memory_offset");
        let get_caller = self.module.get_function("getCaller").unwrap();

        self.builder.build_call(
            get_caller,
            &[BasicValueEnum::PointerValue(memory_offset)],
            "tmp_call",
        );

        let caller_address = self
            .builder
            .build_load(memory_offset, LLVMPreProcessor::CALLER_PROTECTIONS_PARAM);

        self.builder.build_return(Some(&caller_address));
    }

    fn get_caller(&self) {
        // Dummy implementation of the eWASM getCaller function TODO remove for release
        let param_type = self
            .context
            .custom_width_int_type(160)
            .ptr_type(AddressSpace::Generic)
            .as_basic_type_enum();

        let func_type = self.context.void_type().fn_type(&[param_type], false);

        let func_val = self.module.add_function("getCaller", func_type, None);
        let bb = self.context.append_basic_block(func_val, "entry");
        self.builder.position_at_end(bb);

        let memory_offset = func_val.get_params()[0].into_pointer_value();
        let address = self.context.custom_width_int_type(160).const_int(1, false);

        self.builder.build_store(memory_offset, address);
        self.builder.build_return(None);
    }

    /// Integer exponent method. Naive implementation
    /// PRE: a >= 0, b > 0 and a and b are integers
    fn power(&self) {
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

    fn get_ethereum_internal(&self) {
        self.get_balance();
        self.transfer();
    }

    fn get_balance(&self) {
        // wrapper for the eWASM getExternalBalance function
        let address_type = self.context.custom_width_int_type(160).as_basic_type_enum();

        let func_type = self.context.i64_type().fn_type(&[address_type], false);
        let func_val = self
            .module
            .add_function("Flint_balanceOf_Inner", func_type, None);
        let bb = self.context.append_basic_block(func_val, "entry");

        self.builder.position_at_end(bb);

        let address = func_val.get_params()[0];
        let memory_offset = self.builder.build_alloca(address_type, "memory_offset");

        self.builder.build_store(memory_offset, address);

        let int_type = self.context.i128_type();
        let result_offset = self.builder.build_alloca(int_type, "result_offset");
        let get_balance = self.module.get_function("getExternalBalance").unwrap();

        self.builder.build_call(
            get_balance,
            &[
                memory_offset.as_basic_value_enum(),
                result_offset.as_basic_value_enum(),
            ],
            "get_balance",
        );

        let balance = self.builder.build_load(result_offset, "balance");
        let balance = self.builder.build_cast(
            InstructionOpcode::Trunc,
            balance,
            self.context.i64_type(),
            "tmpcast",
        );

        self.builder.build_return(Some(&balance));
    }

    // TODO: Due to a lack of eWASM documentation, we aren't sure how money should be transferred in eWASM (see our question posted in ewasm/design here: https://github.com/ewasm/design/pull/113).
    // The below implementation is based on the Flint 1 send function, however because we were unable to use the eWASM testnet (it is currently down), we were unable to validate
    // if this function was correct.

    fn transfer(&self) {
        // wrapper for the eWASM call function
        let address_type = self.context.custom_width_int_type(160).as_basic_type_enum();

        let value_type = self.context.i64_type().as_basic_type_enum();

        let func_type = self
            .context
            .void_type()
            .fn_type(&[address_type, address_type, value_type], false);
        let func_val = self
            .module
            .add_function("Flint_transfer_Inner", func_type, None);
        let bb = self.context.append_basic_block(func_val, "entry");

        self.builder.position_at_end(bb);

        let to = func_val.get_params()[1];
        let value = func_val.get_params()[2];
        let value_type = self.context.i128_type();
        let value = self
            .builder
            .build_cast(InstructionOpcode::SExt, value, value_type, "cast");

        let get_gas = self.module.get_function("getGasLeft").unwrap();
        let gas = self
            .builder
            .build_call(get_gas, &[], "get_gas")
            .try_as_basic_value()
            .left()
            .unwrap();
        let memory_offset = self.builder.build_alloca(address_type, "memory_offset");
        let value_offset = self.builder.build_alloca(value_type, "value_offset");
        let data_offset = self
            .builder
            .build_alloca(self.context.i64_type(), "data_offset");
        let data_length = self
            .context
            .i32_type()
            .const_int(0, false)
            .as_basic_value_enum();

        self.builder.build_store(memory_offset, to);
        self.builder.build_store(value_offset, value);
        self.builder
            .build_store(data_offset, self.context.i64_type().const_int(0, false));

        let call = self.module.get_function("call").unwrap();

        let result = self
            .builder
            .build_call(
                call,
                &[
                    gas,
                    memory_offset.as_basic_value_enum(),
                    value_offset.as_basic_value_enum(),
                    data_offset.as_basic_value_enum(),
                    data_length,
                ],
                "call",
            )
            .try_as_basic_value()
            .left()
            .unwrap();
        let is_equal = self.builder.build_int_compare(
            IntPredicate::EQ,
            result.into_int_value(),
            self.context.i32_type().const_int(0, false),
            "is_zero",
        );
        let then_bb = self.context.append_basic_block(func_val, "then");
        let else_bb = self.context.append_basic_block(func_val, "else");

        self.builder
            .build_conditional_branch(is_equal, then_bb, else_bb);
        self.builder.position_at_end(then_bb);
        self.builder.build_return(None);
        self.builder.position_at_end(else_bb);

        let revert_function = self
            .module
            .get_function("revert")
            .expect("Could not find revert function");

        // TODO fill program return info with something meaningful
        let zero = self.context.i32_type().const_int(0, false);
        let ptr = self.builder.build_alloca(zero.get_type(), "mem_ptr");
        self.builder.build_store(ptr, zero);

        self.builder.build_call(
            revert_function,
            &[ptr.as_basic_value_enum(), zero.as_basic_value_enum()],
            "halt",
        );

        self.builder.build_unreachable();
    }
}

#[cfg(test)]
mod runtime_tests {
    use crate::ewasm::codegen::Codegen;
    use inkwell::context::Context;
    use inkwell::execution_engine::JitFunction;
    use inkwell::passes::PassManager;
    use inkwell::OptimizationLevel;
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

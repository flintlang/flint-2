use crate::ast::literals::Literal;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::inkwell::values::BasicValueEnum;

pub struct LLVMLiteral<'a> {
    pub literal: &'a Literal,
}

impl<'a> LLVMLiteral<'a> {
    pub fn generate<'ctx>(
        &self,
        codegen: &Codegen<'_, 'ctx>,
        _function_context: &FunctionContext,
    ) -> Option<BasicValueEnum<'ctx>> {
        Some(match self.literal {
            Literal::BooleanLiteral(b) => {
                let bool_type = codegen.context.bool_type();

                if *b {
                    BasicValueEnum::IntValue(bool_type.const_all_ones())
                } else {
                    BasicValueEnum::IntValue(bool_type.const_zero())
                }
            }
            Literal::AddressLiteral(a) => {
                let address_type = codegen.context.custom_width_int_type(160);
                let address = a.trim_start_matches("0x");
                if let Ok(address) = u64::from_str_radix(address, 16) {
                    BasicValueEnum::IntValue(address_type.const_int(address, false))
                } else {
                    panic!("Invalid address literal")
                }
            }
            Literal::StringLiteral(_) => panic!("Strings not currently supported"),
            Literal::U8Literal(u) => {
                BasicValueEnum::IntValue(codegen.context.i8_type().const_int((*u).into(), false))
            }
            Literal::IntLiteral(i) => {
                BasicValueEnum::IntValue(codegen.context.i64_type().const_int(*i, false))
            }
            Literal::FloatLiteral(f) => {
                BasicValueEnum::FloatValue(codegen.context.f64_type().const_float(*f))
            }
        })
    }
}

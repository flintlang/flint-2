use super::call::LLVMFunctionCall;
use crate::ast::{BinOp, BinaryExpression, Expression, FunctionCall, Identifier};
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;
use inkwell::types::StructType;
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};

pub struct LLVMStructAccess<'a> {
    pub expr: &'a Expression,
}

enum FieldOrFunction<'a> {
    StructFunctionCall(&'a FunctionCall),
    StructField(&'a str),
}

impl<'a> FieldOrFunction<'a> {
    #[allow(dead_code)]
    fn as_function_call(&self) -> &FunctionCall {
        if let FieldOrFunction::StructFunctionCall(call) = self {
            call
        } else {
            panic!("Not a function call")
        }
    }

    fn as_field(&self) -> &str {
        if let FieldOrFunction::StructField(field) = self {
            field
        } else {
            panic!("Not a field")
        }
    }
}

impl<'a> LLVMStructAccess<'a> {
    pub fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        if let [first, accesses @ ..] = self.flatten_expr(self.expr).as_slice() {
            let the_struct = function_context.get_declaration(first.as_field()).unwrap();
            let the_struct = the_struct.into_pointer_value();

            let access = accesses.iter().fold(Some(the_struct), |ptr, name| {
                if let Some(ptr) = ptr {
                    self.access(codegen, ptr, name, function_context)
                } else {
                    None
                }
            });

            if function_context.requires_pointer {
                access.map(|ptr| ptr.as_basic_value_enum())
            } else if let Some(ptr) = access {
                Some(codegen.builder.build_load(ptr, "loaded"))
            } else {
                None
            }
        } else {
            panic!("Malformed access")
        }
    }

    fn access<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        ptr: PointerValue<'ctx>,
        rhs: &FieldOrFunction,
        function_context: &mut FunctionContext<'ctx>,
    ) -> Option<PointerValue<'ctx>> {
        let struct_type_name =
            self.get_name_from_struct_type(ptr.get_type().get_element_type().into_struct_type());
        match rhs {
            FieldOrFunction::StructField(field_name) => {
                let (field_names, _) = codegen.types.get(struct_type_name.as_str()).unwrap();
                let index = field_names
                    .iter()
                    .position(|name| name == field_name)
                    .unwrap();

                Some(
                    codegen
                        .builder
                        .build_struct_gep(ptr, index as u32, "tmp_ptr")
                        .expect("Bad access"),
                )
            }
            FieldOrFunction::StructFunctionCall(call) => {
                let val = LLVMFunctionCall {
                    function_call: call,
                }
                .generate(codegen, function_context);

                if let Some(returned) = val {
                    let ret_ptr = codegen.builder.build_alloca(returned.get_type(), "tmp");
                    codegen.builder.build_store(ret_ptr, returned);
                    Some(ret_ptr)
                } else {
                    None
                }
            }
        }
    }

    fn flatten_expr(&self, expr: &'a Expression) -> Vec<FieldOrFunction<'a>> {
        match expr {
            Expression::SelfExpression => vec![FieldOrFunction::StructField(Identifier::SELF)],
            Expression::Identifier(id) => vec![FieldOrFunction::StructField(id.token.as_str())],
            Expression::BinaryExpression(BinaryExpression {
                lhs_expression,
                rhs_expression,
                op: BinOp::Dot,
                ..
            }) => {
                let mut flattened = self.flatten_expr(lhs_expression);
                flattened.extend(self.flatten_expr(rhs_expression));
                flattened
            }
            Expression::FunctionCall(call) => vec![FieldOrFunction::StructFunctionCall(call)],
            _ => panic!("Malformed access"),
        }
    }

    fn get_name_from_struct_type(&self, struct_type: StructType<'a>) -> String {
        struct_type
            .get_name()
            .unwrap()
            .to_str()
            .expect("Could not convert cstr to str")
            .to_string()
    }
}

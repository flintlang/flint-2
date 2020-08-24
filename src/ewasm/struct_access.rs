use super::inkwell::types::StructType;
use super::inkwell::values::PointerValue;
use crate::ast::{BinOp, BinaryExpression, Expression, FunctionCall};
use crate::ewasm::call::LLVMFunctionCall;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;

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
    ) -> PointerValue<'ctx> {
        if let [first, accesses @ ..] = self.flatten_expr(self.expr).as_slice() {
            // TODO account for the fact that we might have something like foo().bar() if foo returns a struct
            let the_struct = function_context.get_declaration(first.as_field()).unwrap();
            let the_struct = the_struct.into_pointer_value();

            accesses.iter().fold(the_struct, |ptr, name| {
                self.access(codegen, ptr, name, function_context)
            })
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
    ) -> PointerValue<'ctx> {
        let struct_type_name =
            self.get_name_from_struct_type(ptr.get_type().get_element_type().into_struct_type());
        match rhs {
            FieldOrFunction::StructField(field_name) => {
                let (field_names, _) = codegen.types.get(struct_type_name.as_str()).unwrap();
                let index = field_names
                    .iter()
                    .position(|name| name == field_name)
                    .unwrap();

                codegen
                    .builder
                    .build_struct_gep(ptr, index as u32, "tmp_ptr")
                    .expect("Bad access")
            }
            FieldOrFunction::StructFunctionCall(call) => {
                let mangled_name =
                    format!("{}_{}", struct_type_name, call.identifier.token.as_str());
                // TODO account for void functions - return dummy pointer? or return Option<Pointer>? If the latter, needs to be
                // project wide probably, a bit like assignments will be
                let return_type = codegen
                    .module
                    .get_function(mangled_name.as_str())
                    .unwrap()
                    .get_type()
                    .get_return_type();
                if let Some(return_type) = return_type {
                    let ret_ptr = codegen.builder.build_alloca(return_type, "tmp");
                    let val = LLVMFunctionCall {
                        function_call: call,
                    }
                        .generate(codegen, function_context);
                    codegen.builder.build_store(ret_ptr, val);
                    ret_ptr
                } else {
                    // TODO switch to returning None
                    codegen
                        .builder
                        .build_alloca(codegen.context.i32_type(), "junk-remove-me")
                }
            }
        }
    }

    fn flatten_expr(&self, expr: &'a Expression) -> Vec<FieldOrFunction<'a>> {
        match expr {
            Expression::SelfExpression => vec![FieldOrFunction::StructField("this")],
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

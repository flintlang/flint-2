use super::inkwell::types::{StructType, VectorType};
use super::inkwell::values::{BasicValue, BasicValueEnum, InstructionOpcode, PointerValue};
use super::inkwell::{FloatPredicate, IntPredicate};
use crate::ast::expressions::{
    BinaryExpression, CastExpression, InoutExpression, SubscriptExpression,
};
use crate::ast::operators::BinOp;
use crate::ast::{Expression, Identifier};
use crate::ewasm::assignment::LLVMAssignment;
use crate::ewasm::call::{LLVMExternalCall, LLVMFunctionCall};
use crate::ewasm::codegen::Codegen;
use crate::ewasm::declaration::LLVMVariableDeclaration;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::literal::LLVMLiteral;
use crate::ewasm::types::LLVMType;

pub struct LLVMExpression<'a> {
    pub expression: &'a Expression,
}

impl<'a> LLVMExpression<'a> {
    // We want to take an expression, create any of the intermediary steps to evaluate it,
    // and then return the variable that stores the evaluated result
    pub fn generate<'ctx>(
        &self,
        codegen: &Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        match self.expression {
            Expression::Identifier(i) => {
                LLVMIdentifier { identifier: i }.generate(codegen, function_context)
            }
            Expression::BinaryExpression(b) => {
                LLVMBinaryExpression { expression: b }.generate(codegen, function_context)
            }
            Expression::InoutExpression(i) => {
                LLVMInoutExpression { expression: i }.generate(codegen, function_context)
            }
            Expression::ExternalCall(f) => {
                LLVMExternalCall { external_call: f }.generate(codegen, function_context)
            }
            Expression::FunctionCall(f) => {
                LLVMFunctionCall {
                    function_call: f,
                    module_name: &"Self".to_string(),
                }
            }
            .generate(codegen, function_context),
            Expression::VariableDeclaration(v) => {
                LLVMVariableDeclaration { declaration: v }.generate(codegen, function_context)
            }
            Expression::BracketedExpression(b) => LLVMExpression {
                expression: &*b.expression,
            }
            .generate(codegen, function_context),
            Expression::AttemptExpression(_) => panic!("Should be removed in the preprocessor"),
            Expression::Literal(l) => {
                LLVMLiteral { literal: l }.generate(codegen, function_context)
            }

            Expression::ArrayLiteral(a) => {
                let elements = a
                    .elements
                    .iter()
                    .map(|e| LLVMExpression { expression: e }.generate(codegen, function_context))
                    .collect::<Vec<BasicValueEnum>>();

                BasicValueEnum::VectorValue(VectorType::const_vector(&elements))
            }
            Expression::DictionaryLiteral(_) => unimplemented!(),
            Expression::SelfExpression => LLVMSelfExpression {}.generate(codegen, function_context),
            Expression::SubscriptExpression(s) => LLVMSubscriptExpression {
                expression: s,
                rhs: None,
            }
            .generate(codegen, function_context),
            Expression::RangeExpression(_) => unimplemented!(),
            Expression::RawAssembly(_, _) => unimplemented!(),
            Expression::CastExpression(c) => {
                LLVMCastExpression { expression: c }.generate(codegen, function_context)
            }
            Expression::Sequence(_) => unimplemented!(),
        }
    }
}

pub struct LLVMIdentifier<'a> {
    pub identifier: &'a Identifier,
}

impl<'a> LLVMIdentifier<'a> {
    pub fn generate<'ctx>(
        &self,
        codegen: &Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        // Move add this check to the preprocessor
        if self.identifier.enclosing_type.is_some() {
            let pointer_to_value = LLVMStructAccess {
                expr: &Expression::BinaryExpression(BinaryExpression {
                    lhs_expression: Box::new(Expression::Identifier(Identifier::generated("this"))),
                    rhs_expression: Box::new(Expression::Identifier(self.identifier.clone())),
                    op: BinOp::Dot,
                    line_info: Default::default(),
                }),
            }
                .generate(codegen, function_context);
            if function_context.assigning {
                pointer_to_value.as_basic_value_enum()
            } else {
                codegen.builder.build_load(pointer_to_value, "val")
            }
        } else if self.identifier.token.as_str().eq(codegen.contract_name) {
            let contract_var = codegen
                .module
                .get_global(codegen.contract_name)
                .unwrap()
                .as_pointer_value();
            if function_context.assigning {
                contract_var.as_basic_value_enum()
            } else {
                codegen.builder.build_load(contract_var, "contract")
            }
        } else {
            let variable = function_context
                .get_declaration(self.identifier.token.as_str())
                .unwrap();

            if function_context.assigning {
                let var_ptr = codegen.builder.build_alloca(variable.get_type(), "tmp");
                codegen.builder.build_store(var_ptr, *variable);
                var_ptr.as_basic_value_enum()
            } else {
                *variable
            }
        }
    }
}

struct LLVMBinaryExpression<'a> {
    expression: &'a BinaryExpression,
}

impl<'a> LLVMBinaryExpression<'a> {
    fn generate<'ctx>(
        &self,
        codegen: &Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        let lhs = LLVMExpression {
            expression: &*self.expression.lhs_expression,
        }
        .generate(codegen, function_context);
        let rhs = LLVMExpression {
            expression: &*self.expression.rhs_expression,
        }
        .generate(codegen, function_context);

        match self.expression.op {
            BinOp::Dot => {
                if let Expression::FunctionCall(f) = &*self.expression.rhs_expression {
                    return LLVMFunctionCall {
                        function_call: f,
                        module_name: "Self",
                    }
                        .generate(codegen, function_context);
                }

                // TODO lhs should always be a pointer if it is not a function call
                // Do a fold of pointer accesses?

                // else if let Expression::Identifier(Identifier {
                //     token: struct_name, ..
                // }) = &*self.expression.lhs_expression
                // {
                //     if let Expression::Identifier(Identifier {
                //         token: field_name, ..
                //     }) = &*self.expression.rhs_expression
                //     {
                //         return LLVMStructAccess {
                //             struct_name,
                //             field_name,
                //         }
                //             .generate(codegen, function_context)
                //             .as_basic_value_enum();
                //     }
                // }
                panic!("Malformed property access")
            }
            BinOp::Equal => LLVMAssignment {
                lhs: &*self.expression.lhs_expression,
                rhs: &*self.expression.rhs_expression,
            }
            .generate(codegen, function_context),
            BinOp::Plus | BinOp::OverflowingPlus => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(
                            codegen.builder.build_int_add(lhs, rhs, "tmpadd"),
                        );
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::FloatValue(codegen.builder.build_float_add(
                            lhs.const_signed_to_float(float_type),
                            rhs,
                            "tmpadd",
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::FloatValue(codegen.builder.build_float_add(
                            lhs,
                            rhs.const_signed_to_float(float_type),
                            "tmpadd",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return BasicValueEnum::FloatValue(
                            codegen.builder.build_float_add(lhs, rhs, "tmpadd"),
                        );
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::OverflowingMinus | BinOp::Minus => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(
                            codegen.builder.build_int_sub(lhs, rhs, "tmpsub"),
                        );
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::FloatValue(codegen.builder.build_float_sub(
                            lhs.const_signed_to_float(float_type),
                            rhs,
                            "tmpsub",
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::FloatValue(codegen.builder.build_float_sub(
                            lhs,
                            rhs.const_signed_to_float(float_type),
                            "tmpsub",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return BasicValueEnum::FloatValue(
                            codegen.builder.build_float_sub(lhs, rhs, "tmpsub"),
                        );
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::OverflowingTimes | BinOp::Times => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(
                            codegen.builder.build_int_mul(lhs, rhs, "tmpmul"),
                        );
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::FloatValue(codegen.builder.build_float_mul(
                            lhs.const_signed_to_float(float_type),
                            rhs,
                            "tmpmul",
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::FloatValue(codegen.builder.build_float_mul(
                            lhs,
                            rhs.const_signed_to_float(float_type),
                            "tmpmul",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return BasicValueEnum::FloatValue(
                            codegen.builder.build_float_mul(lhs, rhs, "tmpmul"),
                        );
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::Power => panic!("operator not supported"),
            BinOp::Divide => {
                if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return BasicValueEnum::FloatValue(
                            codegen.builder.build_float_div(lhs, rhs, "tmpdiv"),
                        );
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::Percent => panic!("operator not supported"),
            BinOp::PlusEqual => panic!("should have been preprocessed"),
            BinOp::MinusEqual => panic!("should have been preprocessed"),
            BinOp::TimesEqual => panic!("should have been preprocessed"),
            BinOp::DivideEqual => panic!("should have been preprocessed"),
            BinOp::DoubleEqual => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_int_compare(
                            IntPredicate::EQ,
                            lhs,
                            rhs,
                            "tmpeq",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OEQ,
                            lhs.const_signed_to_float(float_type),
                            rhs,
                            "tmpeq",
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OEQ,
                            lhs,
                            rhs.const_signed_to_float(float_type),
                            "tmpeq",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OEQ,
                            lhs,
                            rhs,
                            "tmpeq",
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::NotEqual => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_int_compare(
                            IntPredicate::NE,
                            lhs,
                            rhs,
                            "tmpne",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::ONE,
                            lhs.const_signed_to_float(float_type),
                            rhs,
                            "tmpne",
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::ONE,
                            lhs,
                            rhs.const_signed_to_float(float_type),
                            "tmpne",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::ONE,
                            lhs,
                            rhs,
                            "tmpne",
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::LessThan => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_int_compare(
                            IntPredicate::SLT,
                            lhs,
                            rhs,
                            "tmplt",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OLT,
                            lhs.const_signed_to_float(float_type),
                            rhs,
                            "tmplt",
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OLT,
                            lhs,
                            rhs.const_signed_to_float(float_type),
                            "tmplt",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OLT,
                            lhs,
                            rhs,
                            "tmplt",
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::LessThanOrEqual => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_int_compare(
                            IntPredicate::SLE,
                            lhs,
                            rhs,
                            "tmple",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OLE,
                            lhs.const_signed_to_float(float_type),
                            rhs,
                            "tmple",
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OLE,
                            lhs,
                            rhs.const_signed_to_float(float_type),
                            "tmple",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OLE,
                            lhs,
                            rhs,
                            "tmple",
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::GreaterThan => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_int_compare(
                            IntPredicate::SGT,
                            lhs,
                            rhs,
                            "tmpgt",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OGT,
                            lhs.const_signed_to_float(float_type),
                            rhs,
                            "tmpgt",
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OGT,
                            lhs,
                            rhs.const_signed_to_float(float_type),
                            "tmpgt",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OGT,
                            lhs,
                            rhs,
                            "tmpgt",
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::GreaterThanOrEqual => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_int_compare(
                            IntPredicate::SGE,
                            lhs,
                            rhs,
                            "tmpge",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OGE,
                            lhs.const_signed_to_float(float_type),
                            rhs,
                            "tmpge",
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OGE,
                            lhs,
                            rhs.const_signed_to_float(float_type),
                            "tmpge",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_float_compare(
                            FloatPredicate::OGE,
                            lhs,
                            rhs,
                            "tmpge",
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::Or => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(
                            codegen.builder.build_or(lhs, rhs, "tmpor"),
                        );
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::And => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(
                            codegen.builder.build_and(lhs, rhs, "tmpand"),
                        );
                    }
                }

                panic!("Invalid operation supplied")
            }
        }
    }
}

struct LLVMInoutExpression<'a> {
    expression: &'a InoutExpression,
}

impl<'a> LLVMInoutExpression<'a> {
    fn generate<'ctx>(
        &self,
        codegen: &Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        let expr = LLVMExpression {
            expression: &self.expression.expression,
        }
            .generate(codegen, function_context);
        // FIX: into_pointer_value() is wrong
        let ptr = codegen.builder.build_alloca(expr.get_type(), "tmpptr");
        codegen.builder.build_store(ptr, expr);

        BasicValueEnum::PointerValue(ptr)
    }
}

struct LLVMSelfExpression {}

impl<'a> LLVMSelfExpression {
    pub fn generate<'ctx>(
        &self,
        codegen: &Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        LLVMIdentifier {
            identifier: &Identifier::generated(self.name()),
        }
        .generate(codegen, function_context)
    }

    fn name(&self) -> &'static str {
        "this"
    }
}

pub struct LLVMSubscriptExpression<'a> {
    #[allow(dead_code)]
    expression: &'a SubscriptExpression,
    #[allow(dead_code)]
    rhs: Option<LLVMExpression<'a>>,
}

impl<'a> LLVMSubscriptExpression<'a> {
    fn generate<'ctx>(
        &self,
        _codegen: &Codegen<'_, 'ctx>,
        _function_context: &FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        // TODO: implement once arrays are implemented
        unimplemented!();
    }
}

struct LLVMCastExpression<'a> {
    expression: &'a CastExpression,
}

impl<'a> LLVMCastExpression<'a> {
    fn generate<'ctx>(
        &self,
        codegen: &Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        let cast_from_val = LLVMExpression {
            expression: &self.expression.expression,
        }
            .generate(codegen, function_context);
        let cast_to_type = LLVMType {
            ast_type: &self.expression.cast_type,
        }
            .generate(codegen);

        // TODO: which opcode should we pick here?
        codegen.builder.build_cast(
            InstructionOpcode::Load,
            cast_from_val,
            cast_to_type,
            "tmpcast",
        )
    }
}

struct LLVMStructAccess<'a> {
    expr: &'a Expression,
}

impl<'a> LLVMStructAccess<'a> {
    fn generate<'ctx>(
        &self,
        codegen: &Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> PointerValue<'ctx> {
        if let [first, accesses @ ..] = self.flatten_expr(self.expr).as_slice() {
            let the_struct = function_context.get_declaration(first).unwrap();
            let the_struct = the_struct.into_struct_value();

            // Is there a better way to get a pointer to a struct value?
            let struct_ptr = codegen
                .builder
                .build_alloca(the_struct.get_type(), "struct_ptr");
            codegen.builder.build_store(struct_ptr, the_struct);

            accesses
                .iter()
                .fold(struct_ptr, |ptr, name| self.access(codegen, ptr, name))
        } else {
            panic!("Malformed access")
        }
    }

    fn access<'ctx>(
        &self,
        codegen: &Codegen<'_, 'ctx>,
        ptr: PointerValue<'ctx>,
        rhs_name: &str,
    ) -> PointerValue<'ctx> {
        codegen.module.print_to_stderr();
        let struct_type_name =
            self.get_name_from_struct_type(ptr.get_type().get_element_type().into_struct_type());
        let (field_names, _) = codegen.types.get(struct_type_name.as_str()).unwrap();
        let index = field_names
            .iter()
            .position(|name| name == rhs_name)
            .unwrap();

        codegen
            .builder
            .build_struct_gep(ptr, index as u32, "tmp_ptr")
            .expect("Bad access")
    }

    fn flatten_expr(&self, expr: &'a Expression) -> Vec<&'a str> {
        match expr {
            Expression::Identifier(id) => vec![id.token.as_str()],
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
            // TODO: a.b.foo() is unimplemented for now. Associate struct functions with structs?
            Expression::FunctionCall(_) => unimplemented!(),
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

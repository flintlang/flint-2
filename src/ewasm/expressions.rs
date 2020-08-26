use super::inkwell::types::{AnyType, VectorType};
use super::inkwell::values::{BasicValue, BasicValueEnum, InstructionOpcode};
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
use crate::ewasm::struct_access::LLVMStructAccess;
use crate::ewasm::types::LLVMType;
use crate::ewasm::utils::*;

pub struct LLVMExpression<'a> {
    pub expression: &'a Expression,
}

impl<'a> LLVMExpression<'a> {
    // We want to take an expression, create any of the intermediary steps to evaluate it,
    // and then return the variable that stores the evaluated result
    pub fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
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
                { LLVMFunctionCall { function_call: f } }.generate(codegen, function_context)
            }
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
                    .map(|e| {
                        LLVMExpression { expression: e }
                            .generate(codegen, function_context)
                            .unwrap()
                    })
                    .collect::<Vec<BasicValueEnum>>();

                Some(BasicValueEnum::VectorValue(VectorType::const_vector(
                    &elements,
                )))
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
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        if self.identifier.enclosing_type.is_some() {
            LLVMStructAccess {
                expr: &Expression::BinaryExpression(BinaryExpression {
                    lhs_expression: Box::new(Expression::Identifier(Identifier::generated("this"))),
                    rhs_expression: Box::new(Expression::Identifier(self.identifier.clone())),
                    op: BinOp::Dot,
                    line_info: Default::default(),
                }),
            }
                .generate(codegen, function_context)
        } else if !self.identifier.token.as_str().eq(codegen.contract_name) {
            let variable = function_context
                .get_declaration(self.identifier.token.as_str())
                .unwrap();

            Some(*variable)
        } else {
            Some(
                codegen
                    .module
                    .get_global(codegen.contract_name)
                    .unwrap()
                    .as_pointer_value()
                    .as_basic_value_enum(),
            )
        }
    }
}

struct LLVMBinaryExpression<'a> {
    expression: &'a BinaryExpression,
}

impl<'a> LLVMBinaryExpression<'a> {
    fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        match &self.expression.op {
            BinOp::Dot => {
                if let Expression::FunctionCall(f) = &*self.expression.rhs_expression {
                    return LLVMFunctionCall { function_call: f }
                        .generate(codegen, function_context);
                } else {
                    return LLVMStructAccess {
                        expr: &Expression::BinaryExpression(BinaryExpression {
                            lhs_expression: self.expression.lhs_expression.clone(),
                            rhs_expression: self.expression.rhs_expression.clone(),
                            op: BinOp::Dot,
                            line_info: Default::default(),
                        }),
                    }
                    .generate(codegen, function_context)
                    .map(|opt| opt.as_basic_value_enum());
                }
            }
            BinOp::Equal => {
                return LLVMAssignment {
                    lhs: &*self.expression.lhs_expression,
                    rhs: &*self.expression.rhs_expression,
                }
                .generate(codegen, function_context);
            }
            _ => (),
        }

        let lhs = LLVMExpression {
            expression: &*self.expression.lhs_expression,
        }
        .generate(codegen, function_context);
        let rhs = LLVMExpression {
            expression: &*self.expression.rhs_expression,
        }
        .generate(codegen, function_context);

        if lhs.is_none() || rhs.is_none() {
            return None;
        }

        // Beyond here, lhs and rhs must evaluate to something since we are doing binary operations
        // on them.

        let mut lhs = lhs.unwrap();
        let mut rhs = rhs.unwrap();

        if lhs.get_type().is_pointer_type() {
            let lhs_val = codegen
                .builder
                .build_load(lhs.into_pointer_value(), "tmp_l");
            function_context.add_local("tmp_l", lhs_val);
            lhs = lhs_val;
        }

        if rhs.get_type().is_pointer_type() {
            let rhs_val = codegen
                .builder
                .build_load(rhs.into_pointer_value(), "tmp_r");
            function_context.add_local("tmp_r", rhs_val);
            rhs = rhs_val;
        }

        match self.expression.op {
            BinOp::Dot => panic!("Expression should already be evaluated"),
            BinOp::Equal => panic!("Expression should already be evaluated"),
            BinOp::Plus | BinOp::OverflowingPlus => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_int_add(lhs, rhs, "tmpadd"),
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let first_type = codegen.context.f64_type();
                        let first_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            lhs,
                            first_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::FloatValue(codegen.builder.build_float_add(
                            first_val.into_float_value(),
                            rhs,
                            "tmpadd",
                        )));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let second_type = codegen.context.f64_type();
                        let second_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            rhs,
                            second_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::FloatValue(codegen.builder.build_float_add(
                            lhs,
                            second_val.into_float_value(),
                            "tmpadd",
                        )));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return Some(BasicValueEnum::FloatValue(
                            codegen.builder.build_float_add(lhs, rhs, "tmpadd"),
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::OverflowingMinus | BinOp::Minus => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_int_sub(lhs, rhs, "tmpsub"),
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let first_type = codegen.context.f64_type();
                        let first_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            lhs,
                            first_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::FloatValue(codegen.builder.build_float_sub(
                            first_val.into_float_value(),
                            rhs,
                            "tmpsub",
                        )));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let second_type = codegen.context.f64_type();
                        let second_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            rhs,
                            second_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::FloatValue(codegen.builder.build_float_sub(
                            lhs,
                            second_val.into_float_value(),
                            "tmpsub",
                        )));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return Some(BasicValueEnum::FloatValue(
                            codegen.builder.build_float_sub(lhs, rhs, "tmpsub"),
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::OverflowingTimes | BinOp::Times => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_int_mul(lhs, rhs, "tmpmul"),
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let first_type = codegen.context.f64_type();
                        let first_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            lhs,
                            first_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::FloatValue(codegen.builder.build_float_mul(
                            first_val.into_float_value(),
                            rhs,
                            "tmpmul",
                        )));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let second_type = codegen.context.f64_type();
                        let second_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            rhs,
                            second_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::FloatValue(codegen.builder.build_float_mul(
                            lhs,
                            second_val.into_float_value(),
                            "tmpmul",
                        )));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return Some(BasicValueEnum::FloatValue(
                            codegen.builder.build_float_mul(lhs, rhs, "tmpmul"),
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::Power => panic!("operator not supported"),
            BinOp::Divide => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let first_type = codegen.context.f64_type();
                        let second_type = codegen.context.f64_type();
                        let first = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            lhs,
                            first_type,
                            "tmp_cast",
                        );
                        let second = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            rhs,
                            second_type,
                            "tmp_cast",
                        );
                        let result_type = codegen.context.i64_type();
                        let result = codegen.builder.build_float_div(
                            first.into_float_value(),
                            second.into_float_value(),
                            "tmpdiv",
                        );
                        return Some(codegen.builder.build_cast(
                            InstructionOpcode::FPToSI,
                            result,
                            result_type,
                            "tmp_cast",
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let first_type = codegen.context.f64_type();
                        let first_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            lhs,
                            first_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::FloatValue(codegen.builder.build_float_div(
                            first_val.into_float_value(),
                            rhs,
                            "tmpdiv",
                        )));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let second_type = codegen.context.f64_type();
                        let second_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            rhs,
                            second_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::FloatValue(codegen.builder.build_float_div(
                            lhs,
                            second_val.into_float_value(),
                            "tmpdiv",
                        )));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return Some(BasicValueEnum::FloatValue(
                            codegen.builder.build_float_div(lhs, rhs, "tmpdiv"),
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::Percent => {
                // Assume mod can only be used on ints:
                assert!(lhs.is_int_value() && rhs.is_int_value());
                Some(
                    codegen
                        .builder
                        .build_int_signed_rem(lhs.into_int_value(), rhs.into_int_value(), "modulo")
                        .as_basic_value_enum(),
                )
            }
            BinOp::PlusEqual => panic!("should have been preprocessed"),
            BinOp::MinusEqual => panic!("should have been preprocessed"),
            BinOp::TimesEqual => panic!("should have been preprocessed"),
            BinOp::DivideEqual => panic!("should have been preprocessed"),
            BinOp::DoubleEqual => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(codegen.builder.build_int_compare(
                            IntPredicate::EQ,
                            lhs,
                            rhs,
                            "tmpeq",
                        )));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let first_type = codegen.context.f64_type();
                        let first_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            lhs,
                            first_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OEQ,
                                first_val.into_float_value(),
                                rhs,
                                "tmpeq",
                            ),
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let second_type = codegen.context.f64_type();
                        let second_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            rhs,
                            second_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OEQ,
                                lhs,
                                second_val.into_float_value(),
                                "tmpeq",
                            ),
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OEQ,
                                lhs,
                                rhs,
                                "tmpeq",
                            ),
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::NotEqual => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(codegen.builder.build_int_compare(
                            IntPredicate::NE,
                            lhs,
                            rhs,
                            "tmpne",
                        )));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let first_type = codegen.context.f64_type();
                        let first_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            lhs,
                            first_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::ONE,
                                first_val.into_float_value(),
                                rhs,
                                "tmpne",
                            ),
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let second_type = codegen.context.f64_type();
                        let second_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            rhs,
                            second_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::ONE,
                                lhs,
                                second_val.into_float_value(),
                                "tmpne",
                            ),
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::ONE,
                                lhs,
                                rhs,
                                "tmpne",
                            ),
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::LessThan => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(codegen.builder.build_int_compare(
                            IntPredicate::SLT,
                            lhs,
                            rhs,
                            "tmplt",
                        )));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let first_type = codegen.context.f64_type();
                        let first_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            lhs,
                            first_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OLT,
                                first_val.into_float_value(),
                                rhs,
                                "tmplt",
                            ),
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let second_type = codegen.context.f64_type();
                        let second_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            rhs,
                            second_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OLT,
                                lhs,
                                second_val.into_float_value(),
                                "tmplt",
                            ),
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OLT,
                                lhs,
                                rhs,
                                "tmplt",
                            ),
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::LessThanOrEqual => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(codegen.builder.build_int_compare(
                            IntPredicate::SLE,
                            lhs,
                            rhs,
                            "tmple",
                        )));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let first_type = codegen.context.f64_type();
                        let first_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            lhs,
                            first_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OLE,
                                first_val.into_float_value(),
                                rhs,
                                "tmple",
                            ),
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let second_type = codegen.context.f64_type();
                        let second_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            rhs,
                            second_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OLE,
                                lhs,
                                second_val.into_float_value(),
                                "tmple",
                            ),
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OLE,
                                lhs,
                                rhs,
                                "tmple",
                            ),
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::GreaterThan => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(codegen.builder.build_int_compare(
                            IntPredicate::SGT,
                            lhs,
                            rhs,
                            "tmpgt",
                        )));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let first_type = codegen.context.f64_type();
                        let first_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            lhs,
                            first_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OGT,
                                first_val.into_float_value(),
                                rhs,
                                "tmpgt",
                            ),
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let second_type = codegen.context.f64_type();
                        let second_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            rhs,
                            second_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OGT,
                                lhs,
                                second_val.into_float_value(),
                                "tmpgt",
                            ),
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OGT,
                                lhs,
                                rhs,
                                "tmpgt",
                            ),
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::GreaterThanOrEqual => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(codegen.builder.build_int_compare(
                            IntPredicate::SGE,
                            lhs,
                            rhs,
                            "tmpge",
                        )));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let first_type = codegen.context.f64_type();
                        let first_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            lhs,
                            first_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OGE,
                                first_val.into_float_value(),
                                rhs,
                                "tmpge",
                            ),
                        ));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let second_type = codegen.context.f64_type();
                        let second_val = codegen.builder.build_cast(
                            InstructionOpcode::SIToFP,
                            rhs,
                            second_type,
                            "tmp_cast",
                        );
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OGE,
                                lhs,
                                second_val.into_float_value(),
                                "tmpge",
                            ),
                        ));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_float_compare(
                                FloatPredicate::OGE,
                                lhs,
                                rhs,
                                "tmpge",
                            ),
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::Or => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_or(lhs, rhs, "tmpor"),
                        ));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::And => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return Some(BasicValueEnum::IntValue(
                            codegen.builder.build_and(lhs, rhs, "tmpand"),
                        ));
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
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        // An assumption is that inout expressions are only used on structs
        function_context.requires_pointer = true;
        let expr = LLVMExpression {
            expression: &self.expression.expression,
        }
        .generate(codegen, function_context)
        .unwrap();
        function_context.requires_pointer = false;

        if expr.is_pointer_value()
            && expr
                .into_pointer_value()
                .get_name()
                .to_str()
                .expect("cannot convert cstr to str")
                .eq(codegen.contract_name)
        {
            assert_eq!(
                get_num_pointer_layers(expr.get_type().as_any_type_enum()),
                1
            );
            Some(expr)
        } else if expr.is_pointer_value() {
            Some(expr)
        } else {
            let ptr = codegen.builder.build_alloca(expr.get_type(), "tmp_ptr");
            codegen.builder.build_store(ptr, expr);

            Some(BasicValueEnum::PointerValue(ptr))
        }
    }
}

struct LLVMSelfExpression {}

impl<'a> LLVMSelfExpression {
    pub fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
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
    ) -> Option<BasicValueEnum<'ctx>> {
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
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        let cast_from_val = LLVMExpression {
            expression: &self.expression.expression,
        }
        .generate(codegen, function_context)
        .unwrap();

        let cast_to_type = LLVMType {
            ast_type: &self.expression.cast_type,
        }
        .generate(codegen);

        // TODO: which opcode should we pick here? We need tests for this
        Some(codegen.builder.build_cast(
            InstructionOpcode::Load,
            cast_from_val,
            cast_to_type,
            "tmpcast",
        ))
    }
}

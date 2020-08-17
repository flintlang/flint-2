use super::inkwell::values::BasicValueEnum;
use super::inkwell::{FloatPredicate, IntPredicate};
use crate::ast::expressions::{
    AttemptExpression, BinaryExpression, CastExpression, InoutExpression, SubscriptExpression,
};
use crate::ast::operators::BinOp;
use crate::ast::{Expression, Identifier};
use crate::ewasm::assignment::LLVMAssignment;
use crate::ewasm::call::{LLVMExternalCall, LLVMFunctionCall};
use crate::ewasm::codegen::Codegen;
use crate::ewasm::declaration::LLVMVariableDeclaration;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::literal::LLVMLiteral;

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
            Expression::AttemptExpression(a) => {
                LLVMAttemptExpression { expression: a }.generate(codegen, function_context)
            }
            Expression::Literal(l) => {
                LLVMLiteral { literal: l }.generate(codegen, function_context)
            }

            Expression::ArrayLiteral(a) => {
                let _elements = a
                    .elements
                    .iter()
                    .map(|e| LLVMExpression { expression: e }.generate(codegen, function_context))
                    .collect::<Vec<BasicValueEnum>>();

                unimplemented!();
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

struct LLVMIdentifier<'a> {
    #[allow(dead_code)]
    identifier: &'a Identifier,
}

impl<'a> LLVMIdentifier<'a> {
    fn generate<'ctx>(
        &self,
        _codegen: &Codegen<'_, 'ctx>,
        function_context: &FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        if let Some(var) = function_context.get_declaration(&self.identifier.token) {
            return *var;
        }

        panic!(format!(
            "Variable {} has not been assigned to",
            self.identifier.token
        ))
    }
}

struct LLVMBinaryExpression<'a> {
    #[allow(dead_code)]
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
                    LLVMFunctionCall {
                        function_call: f,
                        module_name: "Self",
                    }
                    .generate(codegen, function_context)
                } else {
                    // TODO do struct accesses
                    unimplemented!()
                }
            }
            BinOp::Equal => {
                LLVMAssignment {
                    lhs: &*self.expression.lhs_expression,
                    rhs: &*self.expression.rhs_expression
                }
                .generate(codegen, function_context)
            }
            BinOp::Plus | BinOp::OverflowingPlus => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_int_add(lhs, rhs, "tmpadd"));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::FloatValue(codegen.builder.build_float_add(lhs.const_signed_to_float(float_type), rhs, "tmpadd"));
                    }
                } else if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        let float_type = codegen.context.f64_type();
                        return BasicValueEnum::FloatValue(codegen.builder.build_float_add(lhs, rhs.const_signed_to_float(float_type), "tmpadd"));
                    } else if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return BasicValueEnum::FloatValue(codegen.builder.build_float_add(lhs, rhs, "tmpadd"));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::OverflowingMinus | BinOp::Minus => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_int_sub(lhs, rhs, "tmpsub"));
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
                        return BasicValueEnum::FloatValue(codegen.builder.build_float_sub(lhs, rhs, "tmpsub"));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::OverflowingTimes | BinOp::Times => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_int_mul(lhs, rhs, "tmpmul"));
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
                        return BasicValueEnum::FloatValue(codegen.builder.build_float_mul(lhs, rhs, "tmpmul"));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::Power => panic!("operator not supported"),
            BinOp::Divide => {
                if let BasicValueEnum::FloatValue(lhs) = lhs {
                    if let BasicValueEnum::FloatValue(rhs) = rhs {
                        return BasicValueEnum::FloatValue(codegen.builder.build_float_div(lhs, rhs, "tmpdiv"));
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
                        return BasicValueEnum::IntValue(codegen
                            .builder
                            .build_int_compare(IntPredicate::EQ, lhs, rhs, "tmpeq"));
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
                        return BasicValueEnum::IntValue(codegen
                            .builder
                            .build_float_compare(FloatPredicate::OEQ, lhs, rhs, "tmpeq"));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::NotEqual => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen
                            .builder
                            .build_int_compare(IntPredicate::NE, lhs, rhs, "tmpne"));
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
                        return BasicValueEnum::IntValue(codegen
                            .builder
                            .build_float_compare(FloatPredicate::ONE, lhs, rhs, "tmpne"));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::LessThan => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen
                            .builder
                            .build_int_compare(IntPredicate::SLT, lhs, rhs, "tmplt"));
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
                        return BasicValueEnum::IntValue(codegen
                            .builder
                            .build_float_compare(FloatPredicate::OLT, lhs, rhs, "tmplt"));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::LessThanOrEqual => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen
                            .builder
                            .build_int_compare(IntPredicate::SLE, lhs, rhs, "tmple"));
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
                        return BasicValueEnum::IntValue(codegen
                            .builder
                            .build_float_compare(FloatPredicate::OLE, lhs, rhs, "tmple"));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::GreaterThan => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen
                            .builder
                            .build_int_compare(IntPredicate::SGT, lhs, rhs, "tmpgt"));
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
                        return BasicValueEnum::IntValue(codegen
                            .builder
                            .build_float_compare(FloatPredicate::OGT, lhs, rhs, "tmpgt"));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::GreaterThanOrEqual => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen
                            .builder
                            .build_int_compare(IntPredicate::SGE, lhs, rhs, "tmpge"));
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
                        return BasicValueEnum::IntValue(codegen
                            .builder
                            .build_float_compare(FloatPredicate::OGE, lhs, rhs, "tmpge"));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::Or => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_or(lhs, rhs, "tmpor"));
                    }
                }

                panic!("Invalid operation supplied")
            }
            BinOp::And => {
                if let BasicValueEnum::IntValue(lhs) = lhs {
                    if let BasicValueEnum::IntValue(rhs) = rhs {
                        return BasicValueEnum::IntValue(codegen.builder.build_and(lhs, rhs, "tmpand"));
                    }
                }

                panic!("Invalid operation supplied")
            }
        }
    }
}

struct LLVMInoutExpression<'a> {
    #[allow(dead_code)]
    expression: &'a InoutExpression,
}

impl<'a> LLVMInoutExpression<'a> {
    fn generate<'ctx>(
        &self,
        _codegen: &Codegen<'_, 'ctx>,
        _function_context: &FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}

struct LLVMAttemptExpression<'a> {
    #[allow(dead_code)]
    expression: &'a AttemptExpression,
}

impl<'a> LLVMAttemptExpression<'a> {
    fn generate<'ctx>(
        &self,
        _codegen: &Codegen<'_, 'ctx>,
        _function_context: &FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}

struct LLVMSelfExpression {}

impl<'a> LLVMSelfExpression {
    pub fn generate<'ctx>(
        &self,
        codegen: &Codegen<'_, 'ctx>,
        function_context: &FunctionContext<'ctx>,
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
        unimplemented!();
    }
}

struct LLVMCastExpression<'a> {
    #[allow(dead_code)]
    expression: &'a CastExpression,
}

impl<'a> LLVMCastExpression<'a> {
    fn generate<'ctx>(
        &self,
        _codegen: &Codegen<'_, 'ctx>,
        _function_context: &FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}

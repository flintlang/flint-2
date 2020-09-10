mod assignment;
mod call;
mod declaration;
mod literal;
mod struct_access;

use crate::ast::expressions::{
    BinaryExpression, CastExpression, InoutExpression, SubscriptExpression,
};
use crate::ast::operators::BinOp;
use crate::ast::{Assertion, Expression, Identifier, Literal};
use crate::ewasm::codegen::Codegen;
use crate::ewasm::expressions::assignment::LLVMAssignment;
use crate::ewasm::expressions::call::{LLVMExternalCall, LLVMFunctionCall};
use crate::ewasm::expressions::declaration::LLVMVariableDeclaration;
use crate::ewasm::expressions::literal::LLVMLiteral;
use crate::ewasm::expressions::struct_access::LLVMStructAccess;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::statements::LLVMAssertion;
use crate::ewasm::types::{get_type_as_string, LLVMType};
use crate::ewasm::utils::*;
use inkwell::types::{AnyType, BasicType, BasicTypeEnum};
use inkwell::values::PointerValue;
use inkwell::values::{BasicValue, BasicValueEnum, InstructionOpcode};
use inkwell::AddressSpace;
use inkwell::{FloatPredicate, IntPredicate};

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
                LLVMFunctionCall { function_call: f }.generate(codegen, function_context)
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

                // If we have an empty list, we default to an empty int32 array
                let elem_type = elements
                    .first()
                    .map_or(codegen.context.i32_type().as_basic_type_enum(), |val| {
                        val.get_type()
                    });
                let arr_ptr = codegen
                    .builder
                    .build_alloca(elem_type.array_type(elements.len() as u32), "arr_ptr");

                unsafe {
                    let zero = codegen.context.i32_type().const_int(0, false);
                    for (index, elem) in elements.into_iter().enumerate() {
                        let index = codegen.context.i32_type().const_int(index as u64, false);
                        let position_ptr = codegen.builder.build_in_bounds_gep(
                            arr_ptr,
                            &[zero, index],
                            "index_ptr",
                        );
                        codegen.builder.build_store(position_ptr, elem);
                    }
                }

                Some(codegen.builder.build_load(arr_ptr, "loaded_arr"))
            }
            Expression::DictionaryLiteral(d) => {
                let elements = d
                    .elements
                    .iter()
                    .map(|(k, v)| {
                        let key = LLVMExpression { expression: k }
                            .generate(codegen, function_context)
                            .unwrap();

                        let value = LLVMExpression { expression: v }
                            .generate(codegen, function_context)
                            .unwrap();

                        (key, value)
                    })
                    .collect::<Vec<(BasicValueEnum, BasicValueEnum)>>();

                // If we have an empty dictionary we default to a empty i32, i32 dictionary
                let (key_type, value_type) = if let Some(element) = elements.first() {
                    (element.0.get_type(), element.1.get_type())
                } else {
                    (
                        codegen.context.i32_type().as_basic_type_enum(),
                        codegen.context.i32_type().as_basic_type_enum(),
                    )
                };

                let struct_type = if let Some(struct_type) =
                    codegen.module.get_struct_type(&format!(
                        "dictionary_element_{}_{}",
                        get_type_as_string(&key_type),
                        get_type_as_string(&value_type)
                    )) {
                    struct_type
                } else {
                    let struct_name = format!(
                        "dictionary_element_{}_{}",
                        get_type_as_string(&key_type),
                        get_type_as_string(&value_type)
                    );

                    let struct_type = codegen.context.opaque_struct_type(&struct_name);
                    struct_type.set_body(&[key_type, value_type], false);

                    let struct_info = (vec!["key".to_string(), "value".to_string()], struct_type);
                    codegen.types.insert(struct_name.to_string(), struct_info);

                    struct_type
                };

                let arr_ptr = codegen
                    .builder
                    .build_alloca(struct_type.array_type(elements.len() as u32), "dict_ptr");

                unsafe {
                    let zero = codegen.context.i32_type().const_int(0, false);
                    for (index, elem) in elements.into_iter().enumerate() {
                        //let elem_ptr = codegen.builder.build_alloca(struct_type, "tmp_alloca");
                        let struct_def = struct_type.const_named_struct(&[elem.0, elem.1]);

                        //codegen.builder.build_store(elem_ptr, struct_def);

                        let index = codegen.context.i32_type().const_int(index as u64, false);
                        let position_ptr = codegen.builder.build_in_bounds_gep(
                            arr_ptr,
                            &[zero, index],
                            "index_ptr",
                        );

                        codegen.builder.build_store(position_ptr, struct_def);
                    }
                }

                Some(codegen.builder.build_load(arr_ptr, "loaded_arr"))
            }
            Expression::SelfExpression => LLVMSelfExpression {}.generate(codegen, function_context),
            Expression::SubscriptExpression(s) => {
                LLVMSubscriptExpression { expression: s }.generate(codegen, function_context)
            }
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
                    lhs_expression: Box::new(Expression::Identifier(Identifier::generated(Identifier::SELF))),
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
                return if let Expression::FunctionCall(f) = &*self.expression.rhs_expression {
                    LLVMFunctionCall { function_call: f }.generate(codegen, function_context)
                } else {
                    LLVMStructAccess {
                        expr: &Expression::BinaryExpression(BinaryExpression {
                            lhs_expression: self.expression.lhs_expression.clone(),
                            rhs_expression: self.expression.rhs_expression.clone(),
                            op: BinOp::Dot,
                            line_info: Default::default(),
                        }),
                    }
                    .generate(codegen, function_context)
                    .map(|opt| opt.as_basic_value_enum())
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
            BinOp::Power => {
                if lhs.is_int_value() && rhs.is_int_value() {
                    let exp_func = codegen.module.get_function("_exp").unwrap();

                    codegen
                        .builder
                        .build_call(exp_func, &[lhs, rhs], "exp")
                        .try_as_basic_value()
                        .left()
                } else {
                    panic!("Only int values can use the power operator")
                }
            }
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
            identifier: &Identifier::generated(Identifier::SELF),
        }
        .generate(codegen, function_context)
    }
}

pub struct LLVMSubscriptExpression<'a> {
    expression: &'a SubscriptExpression,
}

impl<'a> LLVMSubscriptExpression<'a> {
    fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        let previous_requires_ptr = function_context.requires_pointer;
        function_context.requires_pointer = true;

        let arr_ptr = LLVMIdentifier {
            identifier: &self.expression.base_expression,
        }
        .generate(codegen, function_context)
        .unwrap();

        function_context.requires_pointer = previous_requires_ptr;

        assert!(arr_ptr.is_pointer_value());
        let arr_ptr = arr_ptr.into_pointer_value();
        let array_len = arr_ptr
            .get_type()
            .get_element_type()
            .into_array_type()
            .len();

        let index = LLVMExpression {
            expression: &*self.expression.index_expression,
        }
        .generate(codegen, function_context)
        .unwrap();

        if is_dictionary(&arr_ptr) {
            let param_type = arr_ptr.get_type();

            let field_types = param_type
                .get_element_type()
                .into_array_type()
                .get_element_type()
                .into_struct_type()
                .get_field_types();

            let key_type = field_types.get(0).unwrap();
            let value_type = field_types.get(1).unwrap();

            let get_func = if let Some(func) = codegen.module.get_function(&format!(
                "_get_{}_{}_{}",
                array_len,
                get_type_as_string(key_type),
                get_type_as_string(value_type)
            )) {
                func
            } else {
                add_get_runtime_function(&arr_ptr, &index, codegen, key_type, value_type);

                let get_func = function_context.get_current_func();
                let block = get_func.get_last_basic_block().unwrap();
                codegen.builder.position_at_end(block);
                codegen
                    .module
                    .get_function(&format!(
                        "_get_{}_{}_{}",
                        array_len,
                        get_type_as_string(key_type),
                        get_type_as_string(value_type)
                    ))
                    .unwrap()
            };

            let value = codegen
                .builder
                .build_call(
                    get_func,
                    &[arr_ptr.as_basic_value_enum(), index.as_basic_value_enum()],
                    "call",
                )
                .try_as_basic_value()
                .left()
                .unwrap();

            if function_context.requires_pointer {
                Some(value.as_basic_value_enum())
            } else {
                let value = codegen
                    .builder
                    .build_load(value.into_pointer_value(), "result");
                Some(value)
            }
        } else {
            self.build_bounds_check(&arr_ptr, codegen, function_context);

            assert!(index.is_int_value());
            let index = index.into_int_value();
            let zero = codegen.context.i32_type().const_int(0, false);

            let access = unsafe {
                codegen
                    .builder
                    .build_in_bounds_gep(arr_ptr, &[zero, index], "accessed")
            };

            if function_context.requires_pointer {
                Some(access.as_basic_value_enum())
            } else {
                Some(codegen.builder.build_load(access, "loaded"))
            }
        }
    }

    fn build_bounds_check<'ctx>(
        &self,
        arr_ptr: &PointerValue,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) {
        // NOTE: this will only work for statically sized arrays
        // This will need to change when dynamic arrays are introduced TODO
        let max_index = arr_ptr
            .get_type()
            .get_element_type()
            .into_array_type()
            .len()
            - 1;

        let gte_zero_predicate = BinaryExpression {
            lhs_expression: Box::new(*self.expression.index_expression.clone()),
            rhs_expression: Box::new(Expression::Literal(Literal::IntLiteral(0))),
            op: BinOp::GreaterThanOrEqual,
            line_info: Default::default(),
        };

        let lte_max_predicate = BinaryExpression {
            lhs_expression: Box::new(*self.expression.index_expression.clone()),
            rhs_expression: Box::new(Expression::Literal(Literal::IntLiteral(max_index as u64))),
            op: BinOp::LessThanOrEqual,
            line_info: Default::default(),
        };

        let predicate = BinaryExpression {
            lhs_expression: Box::new(Expression::BinaryExpression(gte_zero_predicate)),
            rhs_expression: Box::new(Expression::BinaryExpression(lte_max_predicate)),
            op: BinOp::And,
            line_info: Default::default(),
        };

        let bounds_check = Assertion {
            expression: Expression::BinaryExpression(predicate),
            line_info: self.expression.base_expression.line_info.clone(),
        };

        LLVMAssertion {
            assertion: &bounds_check,
        }
        .generate(codegen, function_context);
    }
}

fn is_dictionary(arr_ptr: &PointerValue) -> bool {
    let element_type = arr_ptr
        .get_type()
        .get_element_type()
        .into_array_type()
        .get_element_type();

    if element_type.is_struct_type() {
        let element_type = element_type.into_struct_type();
        return element_type
            .print_to_string()
            .to_string()
            .contains("dictionary_element");
    }

    false
}

fn add_get_runtime_function<'ctx>(
    arr_ptr: &PointerValue<'ctx>,
    index: &BasicValueEnum<'ctx>,
    codegen: &Codegen<'_, 'ctx>,
    key_type: &BasicTypeEnum<'ctx>,
    value_type: &BasicTypeEnum<'ctx>,
) {
    let array_len = arr_ptr
        .get_type()
        .get_element_type()
        .into_array_type()
        .len();

    arr_ptr.set_name("dictionary");
    index.set_name("index");

    let first_param_type = arr_ptr.get_type();
    let second_param_type = index.get_type();
    let func_type = value_type.ptr_type(AddressSpace::Generic).fn_type(
        &[
            first_param_type.as_basic_type_enum(),
            second_param_type.as_basic_type_enum(),
        ],
        false,
    );
    let get_func = codegen.module.add_function(
        &format!(
            "_get_{}_{}_{}",
            array_len,
            get_type_as_string(key_type),
            get_type_as_string(value_type)
        ),
        func_type,
        None,
    );

    let params = get_func.get_params();
    let index_param = params.get(1).unwrap();
    params.get(0).unwrap().set_name("dictionary");
    params.get(1).unwrap().set_name("index");

    let bb = codegen.context.append_basic_block(get_func, "entry");

    codegen.builder.position_at_end(bb);

    let array_len = codegen
        .context
        .i64_type()
        .const_int(array_len.into(), false);
    let i_ptr = codegen
        .builder
        .build_alloca(codegen.context.i64_type().as_basic_type_enum(), "i_ptr");

    codegen
        .builder
        .build_store(i_ptr, codegen.context.i64_type().const_int(0, false));

    let loop_bb = codegen.context.append_basic_block(get_func, "loop");
    let then_bb = codegen.context.append_basic_block(get_func, "then");
    let else_bb = codegen.context.append_basic_block(get_func, "else");
    let check_cond_bb = codegen.context.append_basic_block(get_func, "check_cond");
    let end_bb = codegen.context.append_basic_block(get_func, "end");

    codegen.builder.build_unconditional_branch(check_cond_bb);
    codegen.builder.position_at_end(loop_bb);

    let zero = codegen.context.i32_type().const_int(0, false);
    let i = codegen.builder.build_load(i_ptr, "i").into_int_value();
    let access = unsafe {
        codegen
            .builder
            .build_in_bounds_gep(*arr_ptr, &[zero, i], "accessed")
    };

    assert!(access.get_type().get_element_type().is_struct_type());

    let key = codegen
        .builder
        .build_struct_gep(access, 0, "key_ptr")
        .unwrap();
    let key = codegen.builder.build_load(key, "key");

    if !index.is_int_value() {
        panic!("Dictionaries can only be implemented for int, address or bool values until notions of equality on structs and pointers exist");
    }

    let is_equal = codegen.builder.build_int_compare(
        IntPredicate::EQ,
        key.into_int_value(),
        index_param.into_int_value(),
        "tmpeq",
    );
    codegen
        .builder
        .build_conditional_branch(is_equal, then_bb, else_bb);
    codegen.builder.position_at_end(then_bb);

    let value = codegen
        .builder
        .build_struct_gep(access, 1, "value_ptr")
        .unwrap();

    codegen.builder.build_return(Some(&value));
    codegen.builder.position_at_end(else_bb);

    let add =
        codegen
            .builder
            .build_int_add(i, codegen.context.i64_type().const_int(1, false), "new_i");

    codegen
        .builder
        .build_store(i_ptr, add.as_basic_value_enum());
    codegen.builder.build_unconditional_branch(check_cond_bb);
    codegen.builder.position_at_end(check_cond_bb);

    let i_loaded = codegen.builder.build_load(i_ptr, "count_load");
    let cond = codegen.builder.build_int_compare(
        IntPredicate::ULT,
        i_loaded.into_int_value(),
        array_len,
        "cond",
    );

    codegen
        .builder
        .build_conditional_branch(cond, loop_bb, end_bb);
    codegen.builder.position_at_end(end_bb);

    let revert_function = codegen
        .module
        .get_function("revert")
        .expect("Could not find revert function");

    // TODO fill program return info with something meaningful
    let zero = codegen.context.i32_type().const_int(0, false);
    let ptr = codegen.builder.build_alloca(zero.get_type(), "mem_ptr");
    codegen.builder.build_store(ptr, zero);

    codegen.builder.build_call(
        revert_function,
        &[ptr.as_basic_value_enum(), zero.as_basic_value_enum()],
        "halt",
    );
    codegen.builder.build_unreachable();
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

        Some(codegen.builder.build_cast(
            InstructionOpcode::Trunc,
            cast_from_val,
            cast_to_type,
            "tmpcast",
        ))
    }
}

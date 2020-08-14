use super::inkwell::values::BasicValueEnum;
use crate::ast::{Expression, Identifier};
use crate::ast::expressions::{BinaryExpression, InoutExpression, AttemptExpression, SubscriptExpression, CastExpression};
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::call::{LLVMExternalCall, LLVMFunctionCall};
use crate::ewasm::declaration::LLVMVariableDeclaration;

pub struct LLVMExpression<'a> {
    pub expression: &'a Expression,
}

impl<'a> LLVMExpression<'a> {
    // We want to take an expression, create any of the intermediary steps to evaluate it,
    // and then return the tmp variable that stores the evaluated result
    pub fn generate<'ctx> (
        &self,
        codegen: &Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        match self.expression {
            Expression::Identifier(i) => LLVMIdentifier {
                identifier: i
            }
            .generate(codegen, function_context),
            Expression::BinaryExpression(b) => LLVMBinaryExpression {
                expression: b
            }
            .generate(codegen, function_context),
            Expression::InoutExpression(i) => LLVMInoutExpression {
                expression: i,
            }
            .generate(codegen, function_context),
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
            Expression::Literal(_l) => {
                unimplemented!();
            }
            
            Expression::ArrayLiteral(a) => {
                let _elements = a
                    .elements
                    .iter()
                    .map(|e| {
                        LLVMExpression {
                            expression: e,
                        }
                        .generate(codegen, function_context)
                    })
                    .collect::<Vec<BasicValueEnum>>();

                unimplemented!();
            }
            Expression::DictionaryLiteral(_) => unimplemented!(),
            Expression::SelfExpression => LLVMSelfExpression {
                token: &Identifier::SELF.to_string(),
            }
            .generate(codegen, function_context),
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
            Expression::Sequence(_) => unimplemented!()

        }
    }
}

pub struct LLVMIdentifier<'a> {
    pub identifier: &'a Identifier
}

impl<'a> LLVMIdentifier<'a> {
    pub fn generate<'ctx>(&self, _codegen: &Codegen<'_, 'ctx>, _function_context: &FunctionContext) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}

pub struct LLVMBinaryExpression<'a> {
    pub expression: &'a BinaryExpression
}

impl<'a> LLVMBinaryExpression<'a> {
    pub fn generate<'ctx>(&self, _codegen: &Codegen<'_, 'ctx>, _function_context: &FunctionContext) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}

pub struct LLVMInoutExpression<'a> {
    pub expression: &'a InoutExpression
}

impl<'a> LLVMInoutExpression<'a> {
    pub fn generate<'ctx>(&self, _codegen: &Codegen<'_, 'ctx>, _function_context: &FunctionContext) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}

pub struct LLVMAttemptExpression<'a> {
    pub expression: &'a AttemptExpression
}

impl<'a> LLVMAttemptExpression<'a> {
    pub fn generate<'ctx>(&self, _codegen: &Codegen<'_, 'ctx>, _function_context: &FunctionContext) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}

pub struct LLVMSelfExpression<'a> {
    pub token: &'a String
}

impl<'a> LLVMSelfExpression<'a> {
    pub fn generate<'ctx>(&self, _codegen: &Codegen<'_, 'ctx>, _function_context: &FunctionContext) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}

pub struct LLVMSubscriptExpression<'a> {
    pub expression: &'a SubscriptExpression,
    pub rhs: Option<LLVMExpression<'a>>
}

impl<'a> LLVMSubscriptExpression<'a> {
    pub fn generate<'ctx>(&self, _codegen: &Codegen<'_, 'ctx>, _function_context: &FunctionContext) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}

pub struct LLVMCastExpression<'a> {
    pub expression: &'a CastExpression
}

impl<'a> LLVMCastExpression<'a> {
    pub fn generate<'ctx>(&self, _codegen: &Codegen<'_, 'ctx>, __function_context: &FunctionContext) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}


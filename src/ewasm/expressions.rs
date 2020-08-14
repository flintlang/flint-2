use super::inkwell::values::BasicValueEnum;
use crate::ast::expressions::{
    AttemptExpression, BinaryExpression, CastExpression, InoutExpression, SubscriptExpression,
};
use crate::ast::{Expression, Identifier};
use crate::ewasm::call::{LLVMExternalCall, LLVMFunctionCall};
use crate::ewasm::codegen::Codegen;
use crate::ewasm::declaration::LLVMVariableDeclaration;
use crate::ewasm::function_context::FunctionContext;

pub struct LLVMExpression<'a> {
    pub expr: &'a Expression,
}

impl<'a> LLVMExpression<'a> {
    // We want to take an expression, create any of the intermediary steps to evaluate it,
    // and then return the tmp variable that stores the evaluated result
    pub fn generate<'ctx>(
        &self,
        codegen: &Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        match self.expr {
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
                expr: &*b.expression,
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
                    .map(|e| LLVMExpression { expr: e }.generate(codegen, function_context))
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
        _function_context: &FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}

struct LLVMBinaryExpression<'a> {
    #[allow(dead_code)]
    expression: &'a BinaryExpression,
}

impl<'a> LLVMBinaryExpression<'a> {
    fn generate<'ctx>(
        &self,
        _codegen: &Codegen<'_, 'ctx>,
        _function_context: &FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        unimplemented!();
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

struct LLVMSelfExpression<'a> {
    #[allow(dead_code)]
    token: &'a String,
}

impl<'a> LLVMSelfExpression<'a> {
    pub fn generate<'ctx>(
        &self,
        _codegen: &Codegen<'_, 'ctx>,
        _function_context: &FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        unimplemented!();
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

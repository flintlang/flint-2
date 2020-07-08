use crate::context::Context;
use crate::visitor::Visitor;
use crate::ast::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalCall {
    pub arguments: Vec<FunctionArgument>,
    pub function_call: BinaryExpression,
    pub external_trait_name: Option<String>,
}

impl Visitable for ExternalCall {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_external_call(self, ctx)?;

        let old_is_external_call = ctx.is_external_function_call;
        let old_external_call_context = ctx.external_call_context.clone();

        ctx.is_external_function_call = true;
        ctx.external_call_context = Option::from(self.clone());

        self.function_call.visit(v, ctx)?;

        ctx.is_external_function_call = old_is_external_call;
        ctx.external_call_context = old_external_call_context;

        v.finish_external_call(self, ctx)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionCall {
    pub identifier: Identifier,
    pub arguments: Vec<FunctionArgument>,
    pub mangled_identifier: Option<Identifier>,
}

impl Visitable for FunctionCall {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_function_call(self, ctx)?;

        ctx.is_function_call_context = true;
        self.identifier.visit(v, ctx)?;
        ctx.is_function_call_context = false;

        let old_context = ctx.external_call_context.clone();
        ctx.external_call_context = None;

        self.arguments.visit(v, ctx)?;
        ctx.external_call_context = old_context;

        v.finish_function_call(self, ctx)?;
        ctx.external_call_context = None;

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionArgument {
    pub identifier: Option<Identifier>,
    pub expression: Expression,
}

impl Visitable for FunctionArgument {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_function_argument(self, ctx)?;

        ctx.is_function_call_argument_label = true;
        if self.identifier.is_some() {
            let ident = self.identifier.clone();
            let mut ident = ident.unwrap();

            ident.visit(v, ctx)?;
            self.identifier = Option::from(ident);
        }
        ctx.is_function_call_argument_label = false;

        ctx.is_function_call_argument = true;
        self.expression.visit(v, ctx)?;
        ctx.is_function_call_argument = false;

        v.finish_function_argument(self, ctx)?;

        Ok(())
    }
}

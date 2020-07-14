use super::*;

pub struct SolidityFunctionCall {
    pub function_call: FunctionCall,
}

impl SolidityFunctionCall {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulExpression {
        let match_result = function_context.environment.match_function_call(
            self.function_call.clone(),
            &function_context.enclosing_type,
            vec![],
            function_context.scope_context.clone(),
        );

        if let FunctionCallMatchResult::MatchedInitializer(i) = match_result {
            let mut arg = self.function_call.arguments.clone();
            let arg = arg.remove(0);
            if i.declaration.generated {
                return SolidityExpression {
                    expression: arg.expression,
                    is_lvalue: false,
                }
                .generate(function_context);
            }
        }

        let args = self.function_call.arguments.clone();
        let args: Vec<YulExpression> = args
            .into_iter()
            .map(|a| {
                SolidityExpression {
                    expression: a.expression,
                    is_lvalue: false,
                }
                .generate(function_context)
            })
            .collect();

        let identifier = if self.function_call.mangled_identifier.is_some() {
            let ident = self.function_call.mangled_identifier.clone();
            let ident = ident.unwrap();
            ident.token
        } else {
            self.function_call.identifier.token.clone()
        };

        YulExpression::FunctionCall(YulFunctionCall {
            name: identifier,
            arguments: args,
        })
    }
}
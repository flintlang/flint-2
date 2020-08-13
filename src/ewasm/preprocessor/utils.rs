use crate::ast::types::Type;
use crate::ast::declarations::Parameter;
use crate::ast::expressions::Identifier;

pub fn mangle_ewasm_function(function_name: &str) -> String {
    unimplemented!();
}

pub fn construct_parameter(name: String, t: Type) -> Parameter {
    let identifier = Identifier {
        token: name,
        enclosing_type: None,
        line_info: Default::default(),
    };
    Parameter {
        identifier,
        type_assignment: t,
        expression: None,
        line_info: Default::default(),
    }
}
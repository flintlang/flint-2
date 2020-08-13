use super::inkwell::values::BasicValueEnum;
use crate::environment::Environment;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FunctionContext<'a> {
    pub environment: &'a Environment,
    pub scope_context: HashMap<String, BasicValueEnum<'a>>,
}

// TODO add separate local and parameter variables
impl<'a> FunctionContext<'a> {
    pub fn from(environment: &'a Environment) -> Self {
        FunctionContext {
            environment,
            scope_context: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_local(&mut self, _name: &str, _val: BasicValueEnum) {
        unimplemented!()
    }
}

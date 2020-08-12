use super::inkwell::values::BasicValueEnum;
use crate::environment::Environment;
use std::collections::HashMap;

// The hashmap scopecontext may be useful as a lookup table for what our identifiers refer to. If we want
// to access say, an int that was previously computed, and we have its name, this will give us it

#[derive(Debug, Clone)]
pub struct FunctionContext<'a> {
    pub environment: &'a Environment,
    pub scope_context: HashMap<String, BasicValueEnum<'a>>,
}

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

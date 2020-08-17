use super::inkwell::values::{BasicValueEnum, FunctionValue};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FunctionContext<'a> {
    this_func: FunctionValue<'a>,
    parameters: HashMap<String, BasicValueEnum<'a>>,
    locals: HashMap<String, BasicValueEnum<'a>>,
}

#[allow(dead_code)]
impl<'a> FunctionContext<'a> {
    pub fn new(func: FunctionValue<'a>, params: HashMap<String, BasicValueEnum<'a>>) -> Self {
        FunctionContext {
            this_func: func,
            parameters: params,
            locals: HashMap::new(),
        }
    }

    pub fn get_current_func(&self) -> FunctionValue {
        self.this_func
    }

    pub fn add_local(&mut self, name: &str, val: BasicValueEnum<'a>) {
        // PRE added local should not already be a parameter
        self.locals.insert(name.to_string(), val);
    }

    pub fn get_declaration(&self, name: &str) -> Option<&BasicValueEnum<'a>> {
        self.parameters
            .get(name)
            .or_else(|| self.locals.get(name).or(None))
    }
}

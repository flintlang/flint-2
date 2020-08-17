use super::inkwell::values::{BasicValueEnum, FunctionValue};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FunctionContext<'a, 'ctx> {
    this_func: FunctionValue<'a>,
    parameters: HashMap<&'a str, BasicValueEnum<'ctx>>,
    locals: HashMap<&'a str, BasicValueEnum<'ctx>>,
}

#[allow(dead_code)]
impl<'a, 'ctx> FunctionContext<'a, 'ctx> {
    pub fn new(func: FunctionValue<'a>, params: HashMap<&'a str, BasicValueEnum<'ctx>>) -> Self {
        FunctionContext {
            this_func: func,
            parameters: params,
            locals: HashMap::new(),
        }
    }

    pub fn get_current_func(&self) -> FunctionValue {
        self.this_func
    }

    pub fn add_local(&mut self, name: &'a str, val: BasicValueEnum<'ctx>) {
        // PRE added local should not already be a parameter
        self.locals.insert(name, val);
    }

    pub fn get_declaration(&self, name: &str) -> Option<&BasicValueEnum<'ctx>> {
        self.parameters
            .get(name)
            .or_else(|| self.locals.get(name).or(None))
    }
}

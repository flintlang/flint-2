use super::inkwell::values::{BasicValueEnum, FunctionValue};
use std::collections::HashMap;

#[derive(Debug)]
pub struct FunctionContext<'a> {
    this_func: FunctionValue<'a>,
    parameters: HashMap<String, (Option<String>, BasicValueEnum<'a>)>,
    locals: HashMap<String, (Option<String>, BasicValueEnum<'a>)>,
}

#[allow(dead_code)]
impl<'a> FunctionContext<'a> {
    pub fn new(
        func: FunctionValue<'a>,
        params: HashMap<String, (Option<String>, BasicValueEnum<'a>)>,
    ) -> Self {
        FunctionContext {
            this_func: func,
            parameters: params,
            locals: HashMap::new(),
        }
    }

    pub fn get_current_func(&self) -> FunctionValue {
        self.this_func
    }

    pub fn add_local(&mut self, name: &str, type_name: Option<String>, val: BasicValueEnum<'a>) {
        // PRE added local should not already be a parameter
        self.locals.insert(name.to_string(), (type_name, val));
    }

    pub fn get_declaration(&self, name: &str) -> Option<&(Option<String>, BasicValueEnum<'a>)> {
        self.parameters
            .get(name)
            .or_else(|| self.locals.get(name).or(None))
    }
}

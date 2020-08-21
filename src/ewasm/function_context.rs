use super::inkwell::values::{BasicValueEnum, FunctionValue};
use std::collections::HashMap;
#[derive(Debug)]
pub struct FunctionContext<'a> {
    this_func: FunctionValue<'a>,
    parameters: HashMap<String, BasicValueEnum<'a>>,
    locals: HashMap<String, BasicValueEnum<'a>>,
    pub assigning: bool,
}

impl<'a> FunctionContext<'a> {
    pub fn new(func: FunctionValue<'a>, params: HashMap<String, BasicValueEnum<'a>>) -> Self {
        FunctionContext {
            this_func: func,
            parameters: params,
            locals: HashMap::new(),
            assigning: false,
        }
    }

    pub fn get_current_func(&self) -> FunctionValue {
        self.this_func
    }

    pub fn add_local(&mut self, name: &str, value: BasicValueEnum<'a>) {
        // PRE added local should not already be a parameter
        self.locals.insert(name.to_string(), value);
    }

    pub fn get_declaration(&self, name: &str) -> Option<&BasicValueEnum<'a>> {
        self.parameters
            .get(name)
            .or_else(|| self.locals.get(name).or(None))
    }

    pub fn update_declaration(&mut self, name: &str, val: BasicValueEnum<'a>) {
        // PRE local should already exist
        if self.parameters.contains_key(name) {
            self.parameters.insert(name.to_string(), val);
        } else {
            assert!(self.locals.contains_key(name));
            self.locals.insert(name.to_string(), val);
        }
    }
}

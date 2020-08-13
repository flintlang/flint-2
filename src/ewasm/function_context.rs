use super::inkwell::values::BasicValueEnum;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FunctionContext<'a> {
    parameters: HashMap<&'a str, BasicValueEnum<'a>>,
    locals: HashMap<&'a str, BasicValueEnum<'a>>,
}

#[allow(dead_code)]
impl<'a> FunctionContext<'a> {
    pub fn new(params: HashMap<&'a str, BasicValueEnum<'a>>) -> Self {
        FunctionContext {
            parameters: params,
            locals: HashMap::new(),
        }
    }

    pub fn add_local(&mut self, name: &'a str, val: BasicValueEnum<'a>) {
        // PRE added local should not already be a parameter
        self.locals.insert(name, val);
    }

    pub fn get_declaration(&self, name: &str) -> Option<&BasicValueEnum<'a>> {
        self.parameters
            .get(name)
            .or_else(|| self.locals.get(name).or(None))
    }
}

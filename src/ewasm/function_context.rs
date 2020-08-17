use super::inkwell::values::{BasicValueEnum, FunctionValue};
use std::collections::HashMap;

#[derive(Debug)]
// TODO consider creating and naming opaque structs and then you would only need the structvalue,
// from which you could get the struct name you assigned, instead of storing it here
struct VariableInfo<'a> {
    type_name: Option<String>,
    value: BasicValueEnum<'a>,
}

#[derive(Debug)]
pub struct FunctionContext<'a> {
    this_func: FunctionValue<'a>,
    parameters: HashMap<String, VariableInfo<'a>>,
    locals: HashMap<String, VariableInfo<'a>>,
}

#[allow(dead_code)]
impl<'a> FunctionContext<'a> {
    pub fn new(
        func: FunctionValue<'a>,
        params: HashMap<String, (Option<String>, BasicValueEnum<'a>)>,
    ) -> Self {
        let params = params
            .into_iter()
            .map(|(key, (type_name, value))| (key, VariableInfo { type_name, value }))
            .collect::<HashMap<String, VariableInfo>>();

        FunctionContext {
            this_func: func,
            parameters: params,
            locals: HashMap::new(),
        }
    }

    pub fn get_current_func(&self) -> FunctionValue {
        self.this_func
    }

    pub fn add_local(&mut self, name: &str, type_name: Option<String>, value: BasicValueEnum<'a>) {
        // PRE added local should not already be a parameter
        self.locals
            .insert(name.to_string(), VariableInfo { type_name, value });
    }

    pub fn get_declaration(&self, name: &str) -> (&Option<String>, BasicValueEnum<'a>) {
        let VariableInfo { type_name, value } = self
            .parameters
            .get(name)
            .or_else(|| self.locals.get(name).or(None))
            .unwrap();

        (type_name, *value)
    }

    pub fn update_declaration(&mut self, name: &str, val: BasicValueEnum<'a>) {
        // PRE local should already exist
        if self.parameters.contains_key(name) {
            self.parameters.get_mut(name).unwrap().value = val;
        } else {
            assert!(self.locals.contains_key(name));
            self.locals.get_mut(name).unwrap().value = val;
        }
    }
}

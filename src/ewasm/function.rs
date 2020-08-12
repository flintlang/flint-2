use crate::ewasm::inkwell::types::{BasicTypeEnum, BasicType};
use crate::ewasm::inkwell::values::BasicValue;
use crate::ast::FunctionDeclaration;
use crate::ewasm::Codegen;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::LLVMType;

pub struct LLVMFunction<'a> {
    pub function_declaration: &'a FunctionDeclaration,
}

impl<'a> LLVMFunction<'a> {
    pub fn generate(&self, codegen: &Codegen) {
        // TODO: declare function context?
        
        let function_name = self.function_declaration.head.identifier.token;

        let parameter_types = &self
            .function_declaration
            .head
            .parameters
            .into_iter()
            .map(|param| LLVMType { ast_type: &param.type_assignment }.generate(codegen))
            .collect::<Vec<BasicTypeEnum>>();

        let parameters: Vec<String> = self
            .function_declaration
            .head
            .parameters
            .into_iter()
            .map(|param| { param.identifier.token })
            .collect();

        // TODO: do I need to check if it returns?
        
        let func_type = if let Some(result_type) = self.function_declaration.get_result_type() {
            // should is_var_args be false?
            LLVMType { ast_type: &result_type }.generate(codegen).fn_type(parameter_types, false)
        } else {
            codegen.context.void_type().fn_type(parameter_types, false)
        };

        // add function type to module
        let func_val = codegen.module.add_function(&function_name, func_type, None);

        // set argument names
        func_val
            .get_param_iter()
            .enumerate()
            .map(|(i, arg)| {
                arg.set_name(parameters[i].as_str())
            });

        let body = codegen.context.append_basic_block(func_val, "entry");
        codegen.builder.position_at_end(body);

        //let function_context = FunctionContext::from(self.environment);
        for statement in self.function_declaration.body.iter() {
            let instr = LLVMStatement { statement }.generate(codegen);
            // Add to context now
        }
        
        codegen.verify_and_optimise(&func_val);
    } 
}
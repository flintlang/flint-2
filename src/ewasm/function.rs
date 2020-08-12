use crate::ast::FunctionDeclaration;
use crate::ewasm::inkwell::types::{BasicType, BasicTypeEnum};
use crate::ewasm::inkwell::values::BasicValue;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::LLVMType;
use crate::ewasm::Codegen;

#[allow(dead_code)]
pub struct LLVMFunction<'a> {
    pub function_declaration: &'a FunctionDeclaration,
}

#[allow(dead_code)]
impl<'a> LLVMFunction<'a> {
    pub fn generate(&self, codegen: &Codegen) {
        // TODO: declare function context?

        let function_name = self.function_declaration.head.identifier.token.as_str();

        let parameter_types = &self
            .function_declaration
            .head
            .parameters
            .iter()
            .map(|param| {
                LLVMType {
                    ast_type: &param.type_assignment,
                }
                    .generate(codegen)
            })
            .collect::<Vec<BasicTypeEnum>>();

        let parameters = self
            .function_declaration
            .head
            .parameters
            .iter()
            .map(|param| param.identifier.token.as_str())
            .collect::<Vec<&str>>();

        // TODO: do I need to check if it returns?

        let func_type = if let Some(result_type) = self.function_declaration.get_result_type() {
            // should is_var_args be false?
            LLVMType {
                ast_type: &result_type,
            }
                .generate(codegen)
                .fn_type(parameter_types, false)
        } else {
            codegen.context.void_type().fn_type(parameter_types, false)
        };

        // add function type to module
        let func_val = codegen.module.add_function(&function_name, func_type, None);

        // set argument names
        for (i, param) in func_val.get_param_iter().enumerate() {
            param.set_name(parameters[i]);
        }

        let body = codegen.context.append_basic_block(func_val, "entry");
        codegen.builder.position_at_end(body);

        //let function_context = FunctionContext::from(self.environment);
        for statement in self.function_declaration.body.iter() {
            let _instr = LLVMStatement { statement }.generate(codegen);
            // Add to context now
        }

        codegen.verify_and_optimise(&func_val);
    }
}

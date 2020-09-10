use crate::ast::expressions::Expression;
use crate::ast::Identifier;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::expressions::LLVMExpression;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::utils::get_num_pointer_layers;
use inkwell::types::AnyType;
use inkwell::values::BasicValueEnum;

#[derive(Debug)]
pub struct LLVMAssignment<'a> {
    pub lhs: &'a Expression,
    pub rhs: &'a Expression,
}

impl<'a> LLVMAssignment<'a> {
    pub fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        function_context.requires_pointer = true;
        let lhs = LLVMExpression {
            expression: self.lhs,
        }
        .generate(codegen, function_context)
        .unwrap();
        function_context.requires_pointer = false;
        let rhs = LLVMExpression {
            expression: self.rhs,
        }
        .generate(codegen, function_context)
        .unwrap();

        let lhs_num_pointers = get_num_pointer_layers(lhs.get_type().as_any_type_enum());
        let rhs_num_pointers = get_num_pointer_layers(rhs.get_type().as_any_type_enum());

        // Update the value either in the context, or by storing into the pointer.
        if lhs_num_pointers == rhs_num_pointers + 1 {
            codegen.builder.build_store(lhs.into_pointer_value(), rhs);
        } else if lhs_num_pointers == rhs_num_pointers {
            if let Expression::Identifier(Identifier { token, .. }) = self.lhs {
                assert!(function_context.get_declaration(token).is_some());
                function_context.update_declaration(token, rhs);
            } else if let Expression::VariableDeclaration(dec) = self.lhs {
                function_context.add_local(&dec.identifier.token, rhs);
            } else {
                codegen.module.print_to_stderr();
                panic!("variable not in scope")
            }
        } else {
            panic!("Invalid assignment")
        }

        None
    }
}

pub mod utils;

use crate::ast::{SpecialDeclaration, StructDeclaration, StructMember, VariableDeclaration};
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function::LLVMFunction;
use crate::ewasm::inkwell::types::BasicTypeEnum;
use crate::ewasm::structs::utils::generate_initialiser;
use crate::ewasm::types::LLVMType;

pub struct LLVMStruct<'a> {
    pub struct_declaration: &'a StructDeclaration,
}

impl<'a> LLVMStruct<'a> {
    pub fn generate(&self, codegen: &mut Codegen) {
        self.create_type(codegen);

        let initialiser = self
            .struct_declaration
            .members
            .iter()
            .filter_map(|m| {
                if let StructMember::SpecialDeclaration(sp) = m {
                    if sp.is_public() && sp.is_init() {
                        return Some(sp);
                    }
                }
                None
            })
            .collect::<Vec<&SpecialDeclaration>>();

        assert_eq!(initialiser.len(), 1);

        let initialiser = initialiser[0];
        generate_initialiser(initialiser, codegen);
        self.generate_functions(codegen);
    }

    fn create_type(&self, codegen: &mut Codegen) {
        let fields = &self
            .struct_declaration
            .members
            .iter()
            .filter_map(|f| {
                if let StructMember::VariableDeclaration(vd, _) = f {
                    Some(vd)
                } else {
                    None
                }
            })
            .collect::<Vec<&VariableDeclaration>>();

        let field_names = fields
            .iter()
            .map(|dec| dec.identifier.token.clone())
            .collect::<Vec<String>>();

        let field_types = &fields
            .iter()
            .map(|dec| {
                LLVMType {
                    ast_type: &dec.variable_type,
                }
                .generate(codegen)
            })
            .collect::<Vec<BasicTypeEnum>>();

        let struct_type = codegen.context.struct_type(field_types, false);
        let struct_info = (field_names, struct_type);

        codegen.types.insert(
            self.struct_declaration.identifier.token.clone(),
            struct_info,
        );
    }

    fn generate_functions(&self, codegen: &Codegen) {
        self.struct_declaration
            .members
            .iter()
            .filter_map(|m| {
                if let StructMember::FunctionDeclaration(fd) = m {
                    Some(fd)
                } else {
                    None
                }
            })
            .for_each(|func| {
                LLVMFunction {
                    function_declaration: func,
                }
                .generate(codegen)
            });
    }
}

pub mod utils;

use crate::ast::{SpecialDeclaration, StructDeclaration, StructMember, VariableDeclaration};
use crate::ast::declarations::FunctionDeclaration;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function::{LLVMFunction, generate_function_type};
use crate::ewasm::inkwell::types::BasicTypeEnum;
use crate::ewasm::structs::utils::generate_initialiser;
use crate::ewasm::types::LLVMType;

pub struct LLVMStruct<'a> {
    pub struct_declaration: &'a StructDeclaration,
}

impl<'a> LLVMStruct<'a> {
    pub fn generate(&self, codegen: &mut Codegen) {
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

    fn generate_functions(&self, codegen: &mut Codegen) {
        let function_declarations = self.struct_declaration
            .members
            .iter()
            .filter_map(|m| {
                if let StructMember::FunctionDeclaration(fd) = m {
                    Some(fd)
                } else {
                    None
                }
            })
            .collect::<Vec<&FunctionDeclaration>>();

        function_declarations
            .iter()
            .for_each(|func| generate_function_type(func, codegen));
        
        function_declarations
            .iter()
            .for_each(|func| {
                LLVMFunction {
                    function_declaration: func,
                }
                .generate(codegen)
            });
    }
}

pub fn create_type(struct_declaration: &StructDeclaration, codegen: &mut Codegen) {
    let fields = struct_declaration
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

    let struct_name = struct_declaration.identifier.token.as_str();

    let struct_type = match codegen.types.get(struct_name) {
        Some((_, struct_type)) => *struct_type,
        None => codegen.context.opaque_struct_type(struct_name),
    };

    struct_type.set_body(field_types, false);
    let struct_info = (field_names, struct_type);
    codegen.types.insert(struct_name.to_string(), struct_info);
}

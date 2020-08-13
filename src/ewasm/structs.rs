use super::inkwell::types::BasicTypeEnum;
use super::inkwell::values::BasicValue;
use crate::ast::{SpecialDeclaration, StructDeclaration, StructMember};
use crate::ewasm::codegen::Codegen;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::LLVMType;

#[allow(dead_code)]
pub struct EWASMStruct<'a> {
    pub struct_declaration: &'a StructDeclaration,
}

#[allow(dead_code)]
impl<'a> EWASMStruct<'a> {
    fn create_type(&self, codegen: &mut Codegen) {
        let field_types = self
            .struct_declaration
            .members
            .iter()
            .filter_map(|f| {
                if let StructMember::VariableDeclaration(vd, _) = f {
                    Some(
                        LLVMType {
                            ast_type: &vd.variable_type,
                        }
                        .generate(codegen),
                    )
                } else {
                    None
                }
            })
            .collect::<Vec<BasicTypeEnum>>();

        let struct_type = codegen.context.struct_type(&field_types, false);
        codegen.types.insert(
            self.struct_declaration.identifier.token.clone(),
            struct_type,
        );
    }

    fn generate_functions(&self, _codegen: &Codegen) {
        // let _functions = self
        //     .struct_declaration
        //     .members
        //     .iter()
        //     .filter_map(|m| {
        //         if let StructMember::FunctionDeclaration(fd) = m {
        //             Some(m)
        //         } else {
        //             None
        //         }
        //     })
        //     .collect::<Vec<&FunctionDeclaration>>();

        // TODO call Jess' function generation
    }

    fn generate_initialiser(&self, codegen: &Codegen) {
        // TODO add function context
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

        let params = &initialiser.head.parameters;
        let param_types = params
            .iter()
            .map(|param| {
                LLVMType {
                    ast_type: &param.type_assignment,
                }
                .generate(codegen)
            })
            .collect::<Vec<BasicTypeEnum>>();

        let void_type = codegen.context.void_type().fn_type(&param_types, false);

        let func_name = &format!("{}Init", initialiser.head.enclosing_type.as_ref().unwrap());
        let init_func = codegen.module.add_function(func_name, void_type, None);

        for (i, param) in init_func.get_param_iter().enumerate() {
            param.set_name(params[i].identifier.token.as_str());
        }

        let _block = codegen.context.append_basic_block(init_func, "entry");
        for statement in initialiser.body.iter() {
            LLVMStatement { statement }.generate(codegen);
        }

        codegen.verify_and_optimise(&init_func);
    }

    // TODO generate instantiation of a specific struct too?
}

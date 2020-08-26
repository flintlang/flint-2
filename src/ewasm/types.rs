use super::inkwell::types::{BasicType, BasicTypeEnum};
use super::inkwell::AddressSpace;
use crate::ast::{FixedSizedArrayType, InoutType, Type};
use crate::ewasm::Codegen;

pub struct LLVMType<'a> {
    pub ast_type: &'a Type,
}

impl<'a> LLVMType<'a> {
    pub fn generate<'ctx>(&self, codegen: &mut Codegen<'_, 'ctx>) -> BasicTypeEnum<'ctx> {
        let context = codegen.context;

        match self.ast_type {
            Type::InoutType(inout) => self.inout_to_llvm(inout, codegen),
            Type::ArrayType(_) => unimplemented!(), // Just a fixed size array with a large size?
            Type::RangeType(_) => unimplemented!(),
            Type::FixedSizedArrayType(fixed_arr_type) => self.llvm_array(fixed_arr_type, codegen),
            Type::DictionaryType(_) => unimplemented!(),
            Type::UserDefinedType(definition) => {
                self.extract_defined_type(definition.token.as_str(), codegen)
            }
            Type::Solidity(_) => unimplemented!(),
            Type::SelfType => unimplemented!(),
            Type::Bool => context.bool_type().as_basic_type_enum(),
            Type::Int => context.i64_type().as_basic_type_enum(),
            Type::String => unimplemented!(),
            Type::Address => context.custom_width_int_type(160).as_basic_type_enum(),
            Type::Error => unimplemented!(),
            Type::TypeState => context.i8_type().as_basic_type_enum(),
        }
    }

    fn inout_to_llvm<'ctx>(
        &self,
        inout: &InoutType,
        codegen: &mut Codegen<'_, 'ctx>,
    ) -> BasicTypeEnum<'ctx> {
        let inner_type = LLVMType {
            ast_type: inout.key_type.as_ref(),
        }
        .generate(codegen);
        BasicTypeEnum::PointerType(inner_type.ptr_type(AddressSpace::Generic))
    }

    fn llvm_array<'ctx>(
        &self,
        fixed_arr_type: &FixedSizedArrayType,
        codegen: &mut Codegen<'_, 'ctx>,
    ) -> BasicTypeEnum<'ctx> {
        let elem_type = LLVMType {
            ast_type: fixed_arr_type.key_type.as_ref(),
        }
        .generate(codegen);
        BasicTypeEnum::ArrayType(elem_type.array_type(fixed_arr_type.size as u32))
    }

    fn extract_defined_type<'ctx>(
        &self,
        type_name: &str,
        codegen: &mut Codegen<'_, 'ctx>,
    ) -> BasicTypeEnum<'ctx> {
        if let Some((_, struct_type)) = codegen.types.get(type_name) {
            return struct_type.as_basic_type_enum();
        }

        let struct_value = codegen.context.opaque_struct_type(type_name);
        codegen
            .types
            .insert(type_name.to_string(), (vec![], struct_value));
        struct_value.as_basic_type_enum()
    }
}

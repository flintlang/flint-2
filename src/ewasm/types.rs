use crate::ast::{Type, InoutType, FixedSizedArrayType};
use super::inkwell::types::{BasicTypeEnum, BasicType};
use super::inkwell::AddressSpace;
use super::inkwell::context::Context;

// I wonder whether we should return our own wrapper type, so we may add more information if we want?

// TODO add address space parameter? (see documentation)
pub fn to_llvm_type<'a>(t: &Type, context: &'a Context) -> BasicTypeEnum<'a> {
    match t {
        Type::InoutType(inout) => inout_to_llvm(inout, context),
        Type::ArrayType(_) => unimplemented!(), // Just a fixed size array with a large size?
        Type::RangeType(_) => unimplemented!(),
        Type::FixedSizedArrayType(fixed_arr_type) => llvm_array(fixed_arr_type, context),
        Type::DictionaryType(_) => unimplemented!(),
        Type::UserDefinedType(_) => unimplemented!(), // TODO need to create an llvm type, but for this we need more detail than just the identifier
        Type::Solidity(_) => unimplemented!(),
        Type::SelfType => unimplemented!(), // TODO this depends on how we represent contract data
        Type::Bool => context.bool_type().as_basic_type_enum(),
        Type::Int => context.i64_type().as_basic_type_enum(),
        Type::String => unimplemented!(),
        Type::Address => context.custom_width_int_type(160).as_basic_type_enum(), // Needs to be a 160 bit number?
        Type::Error => unimplemented!(),
        Type::TypeState => context.i8_type().as_basic_type_enum(),
    }
}

fn inout_to_llvm<'a>(inout: &InoutType, context: &'a Context) -> BasicTypeEnum<'a> {
    let inner_type = to_llvm_type(inout.key_type.as_ref(), context);
    BasicTypeEnum::PointerType(inner_type.ptr_type(AddressSpace::Global))
}

fn llvm_array<'a>(fixed_arr_type: &FixedSizedArrayType, context: &'a Context) -> BasicTypeEnum<'a> {
    let elem_type = to_llvm_type(fixed_arr_type.key_type.as_ref(), context);
    BasicTypeEnum::ArrayType(elem_type.array_type(fixed_arr_type.size as u32))
}
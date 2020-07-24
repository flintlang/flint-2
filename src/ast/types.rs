use crate::ast::*;
use crate::context::Context;
use crate::environment::Environment;
use crate::visitor::Visitor;

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    InoutType(InoutType),
    ArrayType(ArrayType),
    RangeType(RangeType),
    FixedSizedArrayType(FixedSizedArrayType),
    DictionaryType(DictionaryType),
    UserDefinedType(Identifier),
    Solidity(SolidityType),
    SelfType,
    Bool,
    Int,
    String,
    Address,
    Error,
    TypeState,
}

impl Type {
    pub fn type_from_identifier(identifier: Identifier) -> Type {
        Type::UserDefinedType(identifier)
    }

    #[allow(dead_code)]
    pub fn name_is_basic_type(name: &str) -> bool {
        match name {
            "Bool" => true,
            "Address" => true,
            "Int" => true,
            "String" => true,
            _ => false,
        }
    }

    pub fn is_dictionary_type(&self) -> bool {
        match self {
            Type::DictionaryType(_) => true,
            _ => false,
        }
    }

    pub fn is_currency_type(&self) -> bool {
        return matches!(
            self,
            Type::UserDefinedType(ref i)
                if i.token == "Wei" || i.token == "Libra" || i.token == "LibraCoin.T"
        )
    }

    #[allow(dead_code)]
    pub fn is_currency_original_type(&self) -> bool {
        return matches!(
            self,
            Type::UserDefinedType(ref i)
                if i.token == "Wei" || i.token == "Libra"
        )
    }

    pub fn is_dynamic_type(&self) -> bool {
        match self {
            Type::Int => false,
            Type::Address => false,
            Type::Bool => false,
            Type::String => false,
            _ => true,
        }
    }

    pub fn is_address_type(&self) -> bool {
        match self {
            Type::Address => true,
            _ => false,
        }
    }

    pub fn is_bool_type(&self) -> bool {
        match self {
            Type::Bool => true,
            _ => false,
        }
    }

    pub fn is_inout_type(&self) -> bool {
        match self {
            Type::InoutType(_) => true,
            _ => false,
        }
    }

    pub fn is_user_defined_type(&self) -> bool {
        match self {
            Type::UserDefinedType(_) => true,
            _ => false,
        }
    }

    pub fn is_built_in_type(&self) -> bool {
        match self {
            Type::InoutType(i) => i.key_type.is_built_in_type(),
            Type::ArrayType(a) => a.key_type.is_built_in_type(),
            Type::RangeType(r) => r.key_type.is_built_in_type(),
            Type::FixedSizedArrayType(a) => a.key_type.is_built_in_type(),
            Type::DictionaryType(_) => unimplemented!(),
            Type::UserDefinedType(_) => false,
            Type::Bool => true,
            Type::Int => true,
            Type::String => true,
            Type::Address => true,
            Type::Error => true,
            Type::SelfType => unimplemented!(),
            Type::Solidity(_) => unimplemented!(),
            Type::TypeState => true,
        }
    }

    pub fn name(&self) -> String {
        match self {
            Type::InoutType(i) => {
                let name = i.key_type.name();
                format!("$inout{name}", name = name)
            }
            Type::ArrayType(_) => unimplemented!(),
            Type::RangeType(_) => unimplemented!(),
            Type::FixedSizedArrayType(_) => unimplemented!(),
            Type::DictionaryType(_) => unimplemented!(),
            Type::UserDefinedType(i) => i.token.clone(),
            Type::Bool => "Bool".to_string(),
            Type::Int => "Int".to_string(),
            Type::String => "String".to_string(),
            Type::Address => "Address".to_string(),
            Type::Error => "Quartz$ErrorType".to_string(),
            Type::SelfType => "Self".to_string(),
            Type::Solidity(s) => format!("{:?}", s),
            Type::TypeState => unimplemented!(),
        }
    }

    pub fn replacing_self(&self, t: &TypeIdentifier) -> Type {
        let input_type = self.clone();

        if Type::SelfType == input_type {
            return Type::UserDefinedType(Identifier {
                token: t.to_string(),
                enclosing_type: None,
                line_info: Default::default(),
            });
        }

        if let Type::InoutType(i) = input_type.clone() {
            if let Type::SelfType = *i.key_type {
                return Type::InoutType(InoutType {
                    key_type: Box::new(Type::UserDefinedType(Identifier {
                        token: t.to_string(),
                        enclosing_type: None,
                        line_info: Default::default(),
                    })),
                });
            }
        }

        input_type
    }

    pub fn is_external_contract(&self, environment: Environment) -> bool {
        let mut internal_type = self.clone();

        if let Type::InoutType(i) = internal_type {
            internal_type = *i.key_type;
        }

        if let Type::UserDefinedType(u) = internal_type {
            if environment.is_trait_declared(&u.token) {
                if let Some(type_infos) = environment.types.get(&u.token) {
                    if !type_infos.is_external_struct() {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn is_external_resource(&self, environment: Environment) -> bool {
        let mut internal_type = self.clone();

        if let Type::InoutType(i) = internal_type {
            internal_type = *i.key_type;
        }

        if let Type::UserDefinedType(u) = internal_type {
            if environment.is_trait_declared(&u.token) {
                if let Some(type_infos) = environment.types.get(&u.token) {
                    if type_infos.is_external_resource() {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn is_external_module(&self, environment: Environment) -> bool {
        let mut internal_type = self.clone();

        if let Type::InoutType(i) = internal_type {
            internal_type = *i.key_type;
        }

        if let Type::UserDefinedType(u) = internal_type {
            if environment.is_trait_declared(&u.token) {
                if let Some(type_infos) = environment.types.get(&u.token) {
                    if type_infos.is_external_module() {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn from_identifier(identifier: Identifier) -> Type {
        match identifier.token.as_str() {
            "Address" => Type::Address,
            "Bool" => Type::Bool,
            "Int" => Type::Int,
            "String" => Type::String,
            _ => Type::UserDefinedType(identifier),
        }
    }
}

impl Visitable for Type {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_type(self, ctx)?;

        v.finish_type(self, ctx)?;

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SolidityType {
    ADDRESS,
    STRING,
    BOOL,
    INT8,
    INT16,
    INT24,
    INT32,
    INT40,
    INT48,
    INT56,
    INT64,
    INT72,
    INT80,
    INT88,
    INT96,
    INT104,
    INT112,
    INT120,
    INT128,
    INT136,
    INT144,
    INT152,
    INT160,
    INT168,
    INT176,
    INT184,
    INT192,
    INT200,
    INT208,
    INT216,
    INT224,
    INT232,
    INT240,
    INT248,
    INT256,
    UINT8,
    UINT16,
    UINT24,
    UINT32,
    UINT40,
    UINT48,
    UINT56,
    UINT64,
    UINT72,
    UINT80,
    UINT88,
    UINT96,
    UINT104,
    UINT112,
    UINT120,
    UINT128,
    UINT136,
    UINT144,
    UINT152,
    UINT160,
    UINT168,
    UINT176,
    UINT184,
    UINT192,
    UINT200,
    UINT208,
    UINT216,
    UINT224,
    UINT232,
    UINT240,
    UINT248,
    UINT256,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DictionaryType {
    pub key_type: Box<Type>,
    pub value_type: Box<Type>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RangeType {
    pub key_type: Box<Type>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayType {
    pub key_type: Box<Type>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixedSizedArrayType {
    pub key_type: Box<Type>,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InoutType {
    pub key_type: Box<Type>,
}

#[derive(Debug)]
pub struct TypeAnnotation {
    pub colon: std::string::String,
    pub type_assigned: Type,
}

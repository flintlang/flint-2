use super::*;

pub enum SolidityIRType {
    Uint256,
    Address,
    Bytes32,
}

impl SolidityIRType {
    pub fn map_to_solidity_type(input: Type) -> SolidityIRType {
        match input {
            Type::InoutType(i) => SolidityIRType::map_to_solidity_type(*i.key_type),
            Type::ArrayType(_) => panic!("Can not convert this type to Solidity Type"),
            Type::RangeType(_) => panic!("Can not convert this type to Solidity Type"),
            Type::FixedSizedArrayType(_) => panic!("Can not convert this type to Solidity Type"),
            Type::DictionaryType(_) => panic!("Can not convert this type to Solidity Type"),
            Type::UserDefinedType(_) => SolidityIRType::Uint256,
            Type::Bool => SolidityIRType::Uint256,
            Type::Int => SolidityIRType::Uint256,
            Type::String => SolidityIRType::Bytes32,
            Type::Address => SolidityIRType::Address,
            Type::Error => panic!("Can not convert Error type to Solidity Type"),
            Type::SelfType => panic!("Can not convert this type to Solidity Type"),
            Type::Solidity(_) => panic!("Can not convert this type to Solidity Type"),
        }
    }

    pub fn if_maps_to_solidity_type(input: Type) -> bool {
        match input {
            Type::InoutType(i) => SolidityIRType::if_maps_to_solidity_type(*i.key_type),
            Type::UserDefinedType(_) => true,
            Type::Bool => true,
            Type::Int => true,
            Type::String => true,
            Type::Address => true,
            _ => false,
        }
    }

    pub fn generate(&self) -> String {
        match self {
            SolidityIRType::Uint256 => "uint256".to_string(),
            SolidityIRType::Address => "address".to_string(),
            SolidityIRType::Bytes32 => "bytes32".to_string(),
        }
    }
}
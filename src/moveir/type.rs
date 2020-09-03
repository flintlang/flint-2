use super::function::FunctionContext;
use super::ir::MoveIRType;
use crate::ast::{FunctionCall, Identifier, Type};
use crate::environment::Environment;
use crate::target::libra;

#[derive(Debug, Clone)]
pub(crate) enum MoveType {
    U8,
    U64,
    Address,
    Bool,
    ByteArray,
    Signer,
    Resource(String),
    StructType(String),
    Reference(Box<MoveType>),
    MutableReference(Box<MoveType>),
    Vector(Box<MoveType>),
    External(String, Box<MoveType>),
}

impl MoveType {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRType {
        match self {
            MoveType::U8 => MoveIRType::U8,
            MoveType::U64 => MoveIRType::U64,
            MoveType::Address => MoveIRType::Address,
            MoveType::Bool => MoveIRType::Bool,
            MoveType::ByteArray => MoveIRType::ByteArray,
            MoveType::Signer => MoveIRType::Signer,
            MoveType::Resource(s) => {
                let comp = s.clone();

                let resource_type = Type::UserDefinedType(Identifier {
                    token: comp.clone(),
                    enclosing_type: None,
                    line_info: Default::default(),
                });

                if comp == "Libra" {
                    let string = format!("Self.{}", s);
                    return MoveIRType::Resource(string);
                }
                if function_context.enclosing_type == *s {
                    let string = "Self.T".to_string();
                    return MoveIRType::Resource(string);
                }
                if resource_type.is_currency_type(&libra::currency()) {
                    return MoveIRType::Resource(s.to_string());
                }
                let string = format!("{}.T", s);
                MoveIRType::Resource(string)
            }
            MoveType::StructType(s) => {
                let string = s.clone();
                if string == "Libra.Libra<LBR.LBR>" {
                    return MoveIRType::StructType(string);
                }
                let string = format!("Self.{}", string);
                MoveIRType::StructType(string)
            }
            MoveType::Reference(base_type) => {
                MoveIRType::Reference(Box::from(base_type.generate(function_context)))
            }
            MoveType::MutableReference(base_type) => {
                MoveIRType::MutableReference(Box::from(base_type.generate(function_context)))
            }
            MoveType::Vector(v) => MoveIRType::Vector(Box::from(v.generate(function_context))),
            MoveType::External(module, typee) => match *typee.clone() {
                MoveType::Resource(name) => {
                    MoveIRType::Resource(format!("{module}.{name}", module = module, name = name))
                }
                MoveType::StructType(name) => {
                    MoveIRType::StructType(format!("{module}.{name}", module = module, name = name))
                }
                _ => panic!("Only External Structs and Resources are Supported"),
            },
        }
    }

    pub fn move_type(original: Type, environment: Option<Environment>) -> MoveType {
        match original.clone() {
            Type::InoutType(r) => {
                let base_type = MoveType::move_type(*r.key_type, environment);
                MoveType::MutableReference(Box::from(base_type))
            }
            Type::ArrayType(a) => {
                MoveType::Vector(Box::from(MoveType::move_type(*a.key_type, None)))
            }
            Type::FixedSizedArrayType(a) => {
                MoveType::Vector(Box::from(MoveType::move_type(*a.key_type, None)))
            }
            Type::DictionaryType(d) => MoveType::move_type(*d.value_type, None),
            Type::UserDefinedType(i) => {
                if let Some(environment) = environment {
                    if i.token == "&signer" {
                        return MoveType::Reference(Box::new(MoveType::Signer));
                    } else if i.token == "signer" {
                        return MoveType::Signer;
                    } else if MoveType::is_resource_type(original.clone(), &i.token, &environment) {
                        return MoveType::Resource(i.token);
                    } else if original.is_external_contract(environment.clone()) {
                        return MoveType::Address;
                    } else if original.is_external_module(environment.clone()) {
                        if let Some(type_info) = environment.types.get(&i.token) {
                            let modifiers: Vec<FunctionCall> = type_info
                                .modifiers
                                .clone()
                                .into_iter()
                                .filter(|m| m.identifier.token == "resource")
                                .collect();
                            return if modifiers.is_empty() {
                                MoveType::External(
                                    i.token,
                                    Box::from(MoveType::StructType("T".to_string())),
                                )
                            } else {
                                MoveType::External(
                                    i.token,
                                    Box::from(MoveType::Resource("T".to_string())),
                                )
                            };
                        }
                    }
                    if environment.is_enum_declared(&i.token) {
                        unimplemented!()
                    } else {
                        MoveType::StructType(i.token)
                    }
                } else {
                    MoveType::StructType(i.token)
                }
            }
            Type::Bool => MoveType::Bool,
            Type::Int => MoveType::U64,
            Type::String => MoveType::ByteArray,
            Type::Address => MoveType::Address,
            Type::RangeType(_) => panic!("Cannot convert type to move equivalent"),
            Type::SelfType => panic!("Cannot convert type to move equivalent"),
            Type::Error => panic!("Cannot convert type error to move equivalent"),
            Type::Solidity(_) => panic!("Cannot convert Solidity Type to move equivalent"),
            Type::TypeState => MoveType::U8,
        }
    }

    pub fn is_resource_type(original: Type, type_id: &str, environment: &Environment) -> bool {
        environment.is_contract_declared(type_id) || original.is_currency_type(&libra::currency())
    }

    pub fn is_resource(&self) -> bool {
        match self {
            MoveType::Resource(_) => true,
            MoveType::External(_, v) => {
                if let MoveType::Resource(_) = **v {
                    return true;
                }
                false
            }
            _ => false,
        }
    }
}

pub(crate) mod move_runtime_types {
    use crate::moveir::ir::{MoveIRModuleImport, MoveIRStatement};

    pub fn get_all_declarations() -> Vec<String> {
        vec![]
        /* TURN OFF LIBRA
        let libra = "resource Libra_Coin { \n coin: Libra.Libra<LBR.LBR>  \n }".to_string();
        vec![libra]
        */
    }

    pub fn get_all_imports() -> Vec<MoveIRStatement> {
        let signer = MoveIRStatement::Import(MoveIRModuleImport {
            name: "Signer".to_string(),
            address: "0x1".to_string(),
        });
        let lbr = MoveIRStatement::Import(MoveIRModuleImport {
            name: "LBR".to_string(),
            address: "0x1".to_string(),
        });
        let libra_account = MoveIRStatement::Import(MoveIRModuleImport {
            name: "LibraAccount".to_string(),
            address: "0x1".to_string(),
        });
        let vector = MoveIRStatement::Import(MoveIRModuleImport {
            name: "Vector".to_string(),
            address: "0x1".to_string(),
        });
        vec![signer, vector, lbr, libra_account]
    }
}

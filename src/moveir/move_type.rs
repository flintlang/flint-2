use crate::moveir::*;

#[derive(Debug, Clone)]
pub enum MoveType {
    U64,
    Address,
    Bool,
    ByteArray,
    Resource(String),
    StructType(String),
    MutableReference(Box<MoveType>),
    Vector(Box<MoveType>),
    External(String, Box<MoveType>),
}

impl MoveType {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRType {
        match self {
            MoveType::U64 => MoveIRType::U64,
            MoveType::Address => MoveIRType::Address,
            MoveType::Bool => MoveIRType::Bool,
            MoveType::ByteArray => MoveIRType::ByteArray,
            MoveType::Resource(s) => {
                let wei = "Wei".to_string();
                let libra = "Libra".to_string();
                let comp = s.clone();

                let resource_type = Type::UserDefinedType(Identifier {
                    token: comp.clone(),
                    enclosing_type: None,
                    line_info: Default::default(),
                });
                if comp == wei || comp == libra {
                    let string = format!("Self.{}", s);
                    return MoveIRType::Resource(string);
                }
                if function_context.enclosing_type == s.to_string() {
                    let string = "Self.T".to_string();
                    return MoveIRType::Resource(string);
                }
                if resource_type.is_currency_type() {
                    return MoveIRType::Resource(s.to_string());
                }
                let string = format!("{}.T", s);
                MoveIRType::Resource(string)
            }
            MoveType::StructType(s) => {
                let string = s.clone();
                if string == "LibraCoin.T".to_string() {
                    return MoveIRType::StructType(string);
                }
                let string = format!("Self.{}", string);
                MoveIRType::StructType(string)
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
                if environment.is_some() {
                    let environment_value = environment.unwrap();
                    if MoveType::is_resource_type(original.clone(), &i.token, &environment_value) {
                        return MoveType::Resource(i.token.clone());
                    } else if original.is_external_contract(environment_value.clone()) {
                        return MoveType::Address;
                    } else if original.is_external_module(environment_value.clone()) {
                        let type_info = environment_value.types.get(&i.token);
                        if type_info.is_some() {
                            let type_info = type_info.unwrap();
                            let modifiers = type_info.modifiers.clone();
                            let modifiers: Vec<FunctionCall> = modifiers
                                .into_iter()
                                .filter(|m| m.identifier.token == "resource".to_string())
                                .collect();
                            if modifiers.is_empty() {
                                return MoveType::External(
                                    i.token.clone(),
                                    Box::from(MoveType::StructType("T".to_string())),
                                );
                            } else {
                                return MoveType::External(
                                    i.token.clone(),
                                    Box::from(MoveType::Resource("T".to_string())),
                                );
                            }
                        }
                    }
                    if environment_value.is_enum_declared(&i.token) {
                        unimplemented!()
                    } else {
                        MoveType::StructType(i.token.clone())
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
        }
    }

    pub fn is_resource_type(original: Type, t: &TypeIdentifier, environment: &Environment) -> bool {
        environment.is_contract_declared(t) || original.is_currency_type()
    }

    pub fn is_resource(&self) -> bool {
        match self {
            MoveType::Resource(_) => true,
            MoveType::External(_, v) => {
                let ext = v.clone();
                if let MoveType::Resource(_) = *ext {
                    return true;
                }
                false
            }
            _ => false,
        }
    }
}

pub struct MoveRuntimeTypes {}

impl MoveRuntimeTypes {
    pub fn get_all_declarations() -> Vec<String> {
        let libra = "resource Libra_Coin {{ \n coin: LibraCoin.T  \n }}".to_string();
        vec![libra]
    }

    pub fn get_all_imports() -> Vec<MoveIRStatement> {
        let libra = MoveIRStatement::Import(MoveIRModuleImport {
            name: "LibraCoin".to_string(),
            address: "0x0".to_string(),
        });
        let libra_account = MoveIRStatement::Import(MoveIRModuleImport {
            name: "LibraAccount".to_string(),
            address: "0x0".to_string(),
        });
        let vector = MoveIRStatement::Import(MoveIRModuleImport {
            name: "Vector".to_string(),
            address: "0x0".to_string(),
        });
        vec![libra, libra_account, vector]
    }
}
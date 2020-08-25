extern crate json;

use self::json::JsonValue;
use crate::ast::{ContractBehaviourDeclaration, FunctionDeclaration, Parameter};
use crate::ast::{ContractBehaviourMember, Type};

pub fn generate_abi(behaviour_declarations: &[&ContractBehaviourDeclaration]) -> String {
    let functions_and_specials = behaviour_declarations
        .iter()
        .flat_map(|dec| dec.members.iter())
        .collect::<Vec<&ContractBehaviourMember>>();

    let mut functions_json = functions_and_specials
        .iter()
        .filter_map(|m| {
            if let ContractBehaviourMember::FunctionDeclaration(fd) = m {
                if fd.is_external && fd.is_public() {
                    return Some(fd);
                }
            }
            None
        })
        .map(|fd| generate_function_abi(fd))
        .collect::<json::Array>();

    let special_json = functions_and_specials
        .iter()
        .find_map(|m| {
            if let ContractBehaviourMember::SpecialDeclaration(sd) = m {
                if sd.is_public() && sd.is_init() {
                    let abi_inputs = sd
                        .head
                        .parameters
                        .iter()
                        .map(|param| generate_parameter_abi(param))
                        .collect::<json::Array>();

                    let constructor_abi = json::object! {
                        type: "contructor",
                        name: "",
                        inputs: abi_inputs,
                        outputs: json::array![],
                        stateMutability: "nonpayable"
                    };
                    return Some(constructor_abi);
                }
            }
            None
        })
        .expect("Contract must have a constructor");

    functions_json.push(special_json);
    (json::object! {abi: functions_json}).dump()
}

fn generate_function_abi(declaration: &FunctionDeclaration) -> JsonValue {
    let func_name = declaration.head.identifier.token.as_str();
    let func_inputs = declaration
        .head
        .parameters
        .iter()
        .map(|param| generate_parameter_abi(param))
        .collect::<json::Array>();
    let func_outputs = generate_return_type_abi(&declaration.head.result_type);
    let state_mutability = "nonpayable"; // TODO implement

    json::object! {
        type: "function",
        name: func_name,
        inputs: func_inputs,
        outputs: func_outputs,
        stateMutability: state_mutability,
    }
}

fn generate_parameter_abi(param: &Parameter) -> JsonValue {
    let abi_name = param.identifier.token.as_str();
    let ether_type = generate_ether_type(&param.type_assignment);
    // TODO implement component types. These will only be used if the ether type is tuple
    json::object! {
        name: abi_name,
        type: ether_type,
    }
}

fn generate_return_type_abi(opt_return_type: &Option<Type>) -> JsonValue {
    if let Some(return_type) = opt_return_type {
        let abi_name = "";
        let ether_type = generate_ether_type(return_type);
        json::array![json::object! {
            name: abi_name,
            type: ether_type,
        }]
    } else {
        json::array![]
    }
}

fn generate_ether_type(flint_type: &Type) -> &str {
    match flint_type {
        Type::Int => "uint64",
        Type::Bool => "bool",
        Type::Address => "address",
        // TODO implement other types
        other => panic!("unimplemented type: {:?}", other),
    }
}

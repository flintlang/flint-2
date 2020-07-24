use crate::type_checker::ExpressionChecker;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use hex::encode;
use sha3::{Digest, Keccak256};

use super::ast::*;
use super::context::*;
use super::environment::*;

mod assignment;
mod contract;
mod expression;
mod external_call;
mod function;
mod function_call;
mod function_selector;
mod identifier;
mod interface;
mod ir_type;
mod literal;
pub mod preprocessor;
mod property;
mod runtime_function;
mod statement;
mod structure;
mod yul;

use self::assignment::*;
use self::contract::*;
use self::expression::*;
use self::external_call::*;
use self::function::*;
use self::function_call::*;
use self::function_selector::*;
use self::identifier::*;
use self::interface::*;
use self::ir_type::*;
use self::literal::*;
use self::property::*;
use self::runtime_function::*;
use self::statement::*;
use self::structure::*;
use self::yul::*;

pub fn generate(module: Module, context: &mut Context) {
    let mut contracts: Vec<SolidityContract> = Vec::new();

    for declaration in &module.declarations {
        if let TopLevelDeclaration::ContractDeclaration(c) = declaration {
            let contract_behaviour_declarations: Vec<ContractBehaviourDeclaration> = module
                .declarations
                .clone()
                .into_iter()
                .filter_map(|d| match d {
                    TopLevelDeclaration::ContractBehaviourDeclaration(cbd) => Some(cbd),
                    _ => None,
                })
                .filter(|cbd| cbd.identifier.token == c.identifier.token)
                .collect();

            let struct_declarations: Vec<StructDeclaration> = module
                .declarations
                .clone()
                .into_iter()
                .filter_map(|d| match d {
                    TopLevelDeclaration::StructDeclaration(s) => Some(s),
                    _ => None,
                })
                .collect();

            let contract = SolidityContract {
                declaration: c.clone(),
                behaviour_declarations: contract_behaviour_declarations,
                struct_declarations,
                environment: context.environment.clone(),
            };
            contracts.push(contract);
        }
    }

    for contract in contracts {
        let c = contract.generate();
        let interface = SolidityInterface {
            contract: contract.clone(),
            environment: context.environment.clone(),
        }
        .generate();

        let mut code = CodeGen {
            code: "".to_string(),
            indent_level: 0,
            indent_size: 2,
        };

        code.add(c);
        code.add(interface);
        print!("{}", code.code);

        let name = contract.declaration.identifier.token.clone();
        let path = &format!("output/{name}.sol", name = name);
        let path = Path::new(path);
        let display = path.display();

        let mut file = match File::create(&path) {
            Err(why) => panic!("couldn't create {}: {}", display, why),
            Ok(file) => file,
        };

        match file.write_all(code.code.as_bytes()) {
            Err(why) => panic!("couldn't write to {}: {}", display, why),
            Ok(_) => println!("successfully wrote to {}", display),
        }
    }
}

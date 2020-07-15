use crate::type_checker::ExpressionCheck;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use super::ast::*;
use super::context::*;
use self::contract::MoveContract;

pub mod preprocessor;
mod asset;
mod assignment;
mod call;
mod contract;
mod declaration;
mod expression;
mod function;
mod identifier;
mod ir;
mod literal;
mod property_access;
mod runtime_function;
mod statement;
mod r#struct;
mod r#type;

#[derive(Debug, Clone)]
pub enum MovePosition {
    Left,
    Accessed,
    Normal,
    Inout,
}

impl Default for MovePosition {
    fn default() -> Self {
        MovePosition::Normal
    }
}

pub fn generate(module: Module, context: &mut Context) {
    let trait_declarations: Vec<TraitDeclaration> = module
        .declarations
        .clone()
        .into_iter()
        .filter_map(|d| match d {
            TopLevelDeclaration::TraitDeclaration(t) => Some(t),
            _ => None,
        })
        .collect();

    let mut contracts: Vec<MoveContract> = Vec::new();
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

            let asset_declarations: Vec<AssetDeclaration> = module
                .declarations
                .clone()
                .into_iter()
                .filter_map(|d| match d {
                    TopLevelDeclaration::AssetDeclaration(a) => Some(a),
                    _ => None,
                })
                .collect();

            let contract = MoveContract {
                contract_declaration: c.clone(),
                contract_behaviour_declarations,
                struct_declarations,
                asset_declarations,
                environment: context.environment.clone(),
                external_traits: trait_declarations.clone(),
            };
            contracts.push(contract);
        }
    }

    for contract in contracts {
        let c = contract.generate();

        let mut code = CodeGen {
            code: "".to_string(),
            indent_level: 0,
            indent_size: 2,
        };

        code.add(c);
        print!("{}", code.code);

        let name = contract.contract_declaration.identifier.token.clone();
        let path = &format!("output/{name}.mvir", name = name);
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

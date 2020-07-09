mod move_asset;
mod move_contract;
mod move_struct;
mod move_statement;
mod move_function;
mod move_call;
mod move_expression;
mod move_type;
mod move_ir;
mod move_assignment;
mod move_runtime_function;
mod move_declaration;
mod move_identifier;
mod move_self;
mod move_property_access;
mod move_literal;

use crate::type_checker::ExpressionCheck;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use super::ast::*;
use super::context::*;
use super::environment::*;

pub mod preprocessor;

use crate::moveir::move_asset::*;
use crate::moveir::move_contract::*;
use crate::moveir::move_struct::*;
use crate::moveir::move_statement::*;
use crate::moveir::move_function::*;
use crate::moveir::move_call::*;
use crate::moveir::move_expression::*;
use crate::moveir::move_type::*;
use crate::moveir::move_assignment::*;
use crate::moveir::move_runtime_function::*;
use crate::moveir::move_declaration::*;
use crate::moveir::move_identifier::*;
use crate::moveir::move_ir::*;
use crate::moveir::move_self::*;
use crate::moveir::move_property_access::*;
use crate::moveir::move_literal::*;

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




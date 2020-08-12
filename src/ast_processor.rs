use super::ast::*;
use super::context::*;
use super::environment::*;
use super::moveir;
use super::semantic_analysis::*;
use super::type_assigner::*;
use super::type_checker::*;
use crate::ewasm;
use crate::ast_processor::Blockchain::*;

pub fn process_ast(mut module: Module, environment: Environment, target: Target) -> VResult {
    let type_assigner = &mut TypeAssigner {};
    let semantic_analysis = &mut SemanticAnalysis {};
    let type_checker = &mut TypeChecker {};
    let context = &mut Context {
        environment,
        target: target.clone(),
        ..Default::default()
    };

    module.visit(type_assigner, context)?;
    module.visit(semantic_analysis, context)?;
    module.visit(type_checker, context)?;

    match target.identifier {
        Libra => {
            let move_preprocessor = &mut moveir::preprocessor::MovePreProcessor {};
            module.visit(move_preprocessor, context)?;
            moveir::generate(module, context);
        }
        Ethereum => {
            ewasm::generate(&module, context);
        }
        _ => panic!("Target not currently supported"),
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct Target {
    pub identifier: Blockchain,
    pub currency: Currency,
}

impl Default for Target {
    fn default() -> Self {
        Target {
            identifier: Blockchain::None,
            currency: Currency::default(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Blockchain {
    Libra,
    Ethereum,
    None,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Currency {
    pub identifier: String,
    pub currency_types: Vec<String>,
}

impl Currency {
    pub fn libra() -> Self {
        Currency {
            identifier: "Libra".to_string(),
            currency_types: vec!["Libra".to_string(), "LibraCoin.T".to_string()],
        }
    }
    pub fn ether() -> Self {
        Currency {
            identifier: "Wei".to_string(),
            currency_types: vec!["Wei".to_string()],
        }
    }
}

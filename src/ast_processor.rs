use super::context::*;
use super::environment::*;
use super::move_codegen;
use super::semantic_analysis::*;
use super::solidity_codegen;
use super::type_assigner::*;
use super::type_checker::*;
use super::ast::*;
use crate::move_codegen::move_preprocessor;
use crate::solidity_codegen::solidity_preprocessor;

pub fn process_ast(mut module: Module, environment: Environment, target: Target) {
    let type_assigner = &mut TypeAssigner {};
    let semantic_analysis = &mut SemanticAnalysis {};
    let type_checker = &mut TypeChecker {};
    let solidity_preprocessor = &mut solidity_preprocessor::SolidityPreProcessor {};
    let move_preprocessor = &mut move_preprocessor::MovePreProcessor {};
    let context = &mut Context {
        environment,
        ..Default::default()
    };

    let result = module.visit(type_assigner, context);

    match result {
        Ok(_) => {}
        Err(_) => return,
    }

    let result = module.visit(semantic_analysis, context);

    match result {
        Ok(_) => {}
        Err(_) => return,
    }

    let result = module.visit(type_checker, context);

    match result {
        Ok(_) => {}
        Err(_) => return,
    }

    if let Target::Move = target {
        let result = module.visit(move_preprocessor, context);

        match result {
            Ok(_) => {}
            Err(_) => return,
        }

        let _result = move_codegen::generate(module, context);
    } else {
        let result = module.visit(solidity_preprocessor, context);

        match result {
            Ok(_) => {}
            Err(_) => return,
        }

        let _result = solidity_codegen::generate(module, context);
    }
}

pub enum Target {
    Move,
    Ether,
}

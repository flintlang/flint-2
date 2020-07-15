use super::ast::*;
use super::context::*;
use super::environment::*;
use super::moveir;
use super::semantic_analysis::*;
use super::solidity;
use super::type_assigner::*;
use super::type_checker::*;

pub fn process_ast(mut module: Module, environment: Environment, target: Target) {
    let type_assigner = &mut TypeAssigner {};
    let semantic_analysis = &mut SemanticAnalysis {};
    let type_checker = &mut TypeChecker {};
    let solidity_preprocessor = &mut solidity::preprocessor::SolidityPreProcessor {};
    let move_preprocessor = &mut moveir::preprocessor::MovePreProcessor {};
    let context = &mut Context {
        environment,
        ..Default::default()
    };

    println!(
        "\n\nvvv BLEEEP BLORP vvv\n\n{:#?}\n\n^^^ BLEEEP BLORP ^^^\n\n",
        module
    );
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

        let _result = moveir::generate(module, context);
    } else {
        let result = module.visit(solidity_preprocessor, context);

        match result {
            Ok(_) => {}
            Err(_) => return,
        }

        let _result = solidity::generate(module, context);
    }
}

pub enum Target {
    Move,
    Ether,
}

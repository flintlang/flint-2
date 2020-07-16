use super::ast::*;
use super::context::*;
use super::environment::*;
use super::moveir;
use super::semantic_analysis::*;
use super::solidity;
use super::type_assigner::*;
use super::type_checker::*;

pub fn process_ast(mut module: Module, environment: Environment, target: Target) -> VResult {
    let type_assigner = &mut TypeAssigner {};
    let semantic_analysis = &mut SemanticAnalysis {};
    let type_checker = &mut TypeChecker {};
    let solidity_preprocessor = &mut solidity::preprocessor::SolidityPreProcessor {};
    let move_preprocessor = &mut moveir::preprocessor::MovePreProcessor {};
    let context = &mut Context {
        environment,
        ..Default::default()
    };

    module.visit(type_assigner, context)?;
    module.visit(semantic_analysis, context)?;
    module.visit(type_checker, context)?;

    if let Target::Move = target {
        module.visit(move_preprocessor, context)?;
        moveir::generate(module, context);
    } else {
        module.visit(solidity_preprocessor, context)?;
        solidity::generate(module, context);
    }

    Ok(())
}

pub enum Target {
    Move,
    Ether,
}

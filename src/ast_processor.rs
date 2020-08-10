use super::ast::*;
use super::context::*;
use super::environment::*;
use super::moveir;
use super::semantic_analysis::*;
use super::type_assigner::*;
use super::type_checker::*;
use crate::ewasm;

pub fn process_ast(mut module: Module, environment: Environment, target: Target) -> VResult {
    let type_assigner = &mut TypeAssigner {};
    let semantic_analysis = &mut SemanticAnalysis {};
    let type_checker = &mut TypeChecker {};
    let context = &mut Context {
        environment,
        ..Default::default()
    };

    module.visit(type_assigner, context)?;
    module.visit(semantic_analysis, context)?;
    module.visit(type_checker, context)?;

    if let Target::Move = target {
        let move_preprocessor = &mut moveir::preprocessor::MovePreProcessor {};
        module.visit(move_preprocessor, context)?;
        moveir::generate(module, context);
    } else {
        // TODO eWASM preprocessor visit
        ewasm::generate(&module, context);
    }

    Ok(())
}

pub enum Target {
    Move,
    Ether,
}

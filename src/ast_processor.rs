use super::ast::*;
use super::context::*;
use super::environment::*;
use super::semantic_analysis::*;
use super::type_assigner::*;
use super::type_checker::*;
use crate::target::Target;

pub fn process_ast(mut module: Module, environment: Environment, mut target: Target) -> VResult {
    let type_assigner = &mut TypeAssigner {};
    let semantic_analysis = &mut SemanticAnalysis {};
    let type_checker = &mut TypeChecker {};
    let context = &mut Context {
        environment,
        target: crate::context::Target::from(&target),
        ..Default::default()
    };

    module.visit(type_assigner, context)?;
    module.visit(semantic_analysis, context)?;
    module.visit(type_checker, context)?;

    module.visit(&mut *target.processor, context)?;
    (target.generate)(&module, context);

    Ok(())
}

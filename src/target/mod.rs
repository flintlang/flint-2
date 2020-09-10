pub mod currency;
pub mod ethereum;
pub mod libra;

use crate::ast::Module;
use crate::context::Context;
use crate::target::currency::Currency;
use crate::visitor::Visitor;
use std::path::Path;

/// Integrating any future target should simply require updating this module by adding a new
/// impl of Target, and registering it in io target(&str)
pub struct Target {
    pub(crate) name: &'static str,
    pub(crate) currency: Currency,
    pub(crate) processor: Box<dyn Visitor>,
    pub(crate) generate: fn(module: &Module, context: &mut Context) -> (),
    pub(crate) stdlib_path: &'static Path,
}

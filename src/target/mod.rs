pub mod currency;
pub mod ethereum;
pub mod libra;

/*
   Integrating any future target should simply require updating this module by adding a new
   impl of Target, and registering it in io target(&str)
*/

use crate::ast::Module;
use crate::context::Context;
use crate::target::currency::Currency;
use crate::visitor::Visitor;

pub struct Target {
    pub(crate) name: &'static str,
    pub(crate) currency: Currency,
    pub(crate) processor: Box<dyn Visitor>,
    pub(crate) generate: fn(module: &Module, context: &mut Context) -> (),
}
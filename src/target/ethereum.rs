use super::currency::Currency;
use super::Target;
use crate::ewasm;
use crate::ewasm::preprocessor::LLVMPreProcessor;
use std::path::Path;

pub(crate) fn currency() -> Currency {
    Currency {
        identifier: "Wei",
        currency_types: vec!["Wei"],
    }
}

pub(crate) fn target() -> Target {
    Target {
        name: "eWASM",
        currency: currency(),
        processor: Box::new(LLVMPreProcessor {}),
        generate: ewasm::generate,
        stdlib_path: Path::new("stdlib/ether/global.flint"),
    }
}

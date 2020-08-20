use super::currency::Currency;
use super::Target;
use crate::ewasm;
use crate::ewasm::preprocessor::LLVMPreProcessor;

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
    }
}

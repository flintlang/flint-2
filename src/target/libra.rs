use crate::moveir;
use crate::moveir::preprocessor::MovePreProcessor;
use crate::target::currency::Currency;
use crate::target::Target;
use std::path::Path;

pub(crate) fn currency() -> Currency {
    Currency {
        identifier: "Libra",
        currency_types: vec!["Libra", "LibraCoin.T"],
    }
}

pub(crate) fn target() -> Target {
    Target {
        name: "Libra",
        currency: currency(),
        processor: Box::new(MovePreProcessor {}),
        generate: moveir::generate,
        stdlib_path: Path::new("stdlib/libra/global.flint"),
    }
}

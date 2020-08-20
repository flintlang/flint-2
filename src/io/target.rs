use crate::target::ethereum;
use crate::target::libra;
use crate::target::Target;

pub(crate) fn target(target: &str) -> Option<Target> {
    match target.to_lowercase().as_str() {
        "ethereum" => Some(ethereum::target()),
        "libra" => Some(libra::target()),
        _ => None,
    }
}

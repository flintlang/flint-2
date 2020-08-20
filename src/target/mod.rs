pub mod currency;
pub mod ethereum;
pub mod libra;
pub mod target;

pub use self::target::*;

/*
   Integrating any future target should simply require updating this module by adding a new
   impl of Target, and registering it in io target(&str)
*/

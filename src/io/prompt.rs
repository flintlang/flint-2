use crate::target::Target;
use std::path::PathBuf;

pub struct Configuration {
    pub bin: PathBuf,
    pub file: PathBuf,
    pub target: Target,
}

pub fn process<I: Iterator<Item = String>>(arguments: &mut I) -> Option<Configuration> {
    let bin = PathBuf::from(arguments.next().unwrap_or_default());
    let target = arguments.next().or_else(instructions)?;
    if &*target == "-h" {
        instructions()?
    } else if &*target == "-v" {
        version()?
    }
    let target = super::target::target(&*target).or_else(instructions)?;

    let file = arguments.next().or_else(instructions)?;
    let file = PathBuf::from(file);
    Some(Configuration { bin, file, target })
}

fn instructions<A>() -> Option<A> {
    println!(
        "\
flint [TARGET] [INPUT]

TARGET:         Output code to build
    libra           Move IR for the Libra
    ethereum        eWASM for Ethereum

INPUT:          Input flint file to compile"
    );
    None
}

fn version<A>() -> Option<A> {
    println!(
        r"Flint 2 - unreleased development build

Developed thanks to the work at the Department of Computing,
Imperial College London under the supervision of Professor
Susan Eisenbach, undertaken by Jessica Lally, Matthew Ross
Rachar, and George Stacey, with the core of the project the
Quartz research compiler by Ali Chaudhry, built for his MEng
thesis.

     ..           _________       __     ___
    &&&&         / ____/ (_)___  / /_   |__ \
  &&&&&&&&      / /_  / / / __ \/ __/   __/ /
 &&&>**<&&&    / __/ / / / / / / /_    / __/
 %/      \%   /_/   /_/_/_/ /_/\__/   /____/
"
    );
    None
}

pub mod error {
    use std::path::Path;

    pub fn unable_to_open_file(path: &Path, error: std::io::Error) -> ! {
        eprintln!(
            "Unable to open file `{}`: {}",
            path.to_str().unwrap_or_default(),
            error
        );
        std::process::exit(2)
    }

    pub fn unable_to_read_file(path: &Path, error: std::io::Error) -> ! {
        eprintln!(
            "Unable to read file `{}`: {}",
            path.to_str().unwrap_or_default(),
            error
        );
        std::process::exit(2)
    }

    pub fn parse_failed(error: &str) -> ! {
        eprintln!("Parse error: {}", error);
        std::process::exit(3)
    }

    pub fn semantic_check_failed(error: &dyn std::error::Error) -> ! {
        eprintln!("Semantic check error: {}", error);
        std::process::exit(4)
    }
}

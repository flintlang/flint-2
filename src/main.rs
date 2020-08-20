mod ewasm;

mod ast;
mod ast_processor;
mod context;
mod environment;
mod io;
mod moveir;
mod parser;
mod semantic_analysis;
mod target;
mod type_assigner;
mod type_checker;
mod utils;
mod visitor;

use self::io::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::process::exit;

fn main() {
    let configuration = prompt::process(&mut env::args()).unwrap_or_else(|| exit(1));

    let mut file = File::open(&configuration.file).expect(&*format!(
        "Unable to open file at path `{}`",
        configuration.file.to_str().unwrap_or("<?>")
    ));

    let mut program = String::new();
    file.read_to_string(&mut program)
        .expect("Unable to read the file");

    let (module, environment) = parser::parse_program(&program).unwrap_or_else(|err| {
        println!("Could not parse file: {}", err);
        std::process::exit(1);
    });

    ast_processor::process_ast(module, environment, configuration.target).unwrap_or_else(|err| {
        println!("Could not parse invalid flint file: {}", err);
        std::process::exit(1);
    });
}

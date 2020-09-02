mod ewasm;
mod ewasm_tests;

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

    let mut file = File::open(&configuration.file)
        .unwrap_or_else(|err| prompt::error::unable_to_open_file(&configuration.file, err));

    let mut program = String::new();
    file.read_to_string(&mut program)
        .unwrap_or_else(|err| prompt::error::unable_to_read_file(&configuration.file, err));

    let mut file = File::open(&configuration.target.stdlib_path).unwrap_or_else(|err| {
        prompt::error::unable_to_open_file(&configuration.target.stdlib_path, err)
    });

    file.read_to_string(&mut program).unwrap_or_else(|err| {
        prompt::error::unable_to_read_file(&configuration.target.stdlib_path, err)
    });

    let (module, environment) =
        parser::parse_program(&program).unwrap_or_else(|err| prompt::error::parse_failed(&*err));

    ast_processor::process_ast(module, environment, configuration.target)
        .unwrap_or_else(|err| prompt::error::semantic_check_failed(&*err));
}

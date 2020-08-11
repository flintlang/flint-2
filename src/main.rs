mod ast;
mod ast_processor;
mod context;
mod environment;
mod moveir;
mod parser;
mod semantic_analysis;
mod type_assigner;
mod type_checker;
mod utils;
mod visitor;

use crate::ast_processor::{Blockchain::*, Currency};

#[allow(clippy::all)] // Solidity is deprecated, no need to lint
mod solidity;

use crate::ast_processor::Target;
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    if args.len() < 3 {
        panic!("Incorrect number of Arguments supplied, Expecting 2 arguments");
    }

    let target = &args[1];
    let target = if target == "libra" {
        Target {
            identifier: Libra,
            currency: Currency::libra(),
        }
    //Target::Move
    } else if target == "ether" {
        Target {
            identifier: Ethereum,
            currency: Currency::ether(),
        }
    //Target::Ether
    } else {
        panic!("Incorrect Target Argument specified, expecting \"ether\" or \"libra\"");
    };

    let filename = &args[2];

    let mut file =
        File::open(filename).expect(&*format!("Unable to open file at path {} ", filename));

    let mut program = String::new();
    file.read_to_string(&mut program)
        .expect("Unable to read the file");
    if target.identifier == Libra {
        /* TURN OFF LIBRA
        let mut file =
            File::open("src/stdlib/libra/libra.quartz").expect("Unable to open libra stdlib file ");
        let mut libra = String::new();
        file.read_to_string(&mut libra)
            .expect("Unable to read the stdlib Libra file");

        let mut file = File::open("src/stdlib/libra/global.quartz")
            .expect("Unable to open libra stdlib file ");
        let mut global = String::new();
        file.read_to_string(&mut global)
            .expect("Unable to read the stdlib global file");
        program = format!(
            "{libra} \n {global} \n {program}",
            libra = libra,
            global = global,
            program = program
        )
        */
    } else if target.identifier == Ethereum {
        let mut file =
            File::open("src/stdlib/ether/wei.quartz").expect("Unable to open libra stdlib file ");
        let mut ether = String::new();
        file.read_to_string(&mut ether)
            .expect("Unable to read the stdlib Libra file");

        let mut file = File::open("src/stdlib/ether/global.quartz")
            .expect("Unable to open quartz stdlib file ");
        let mut global = String::new();
        file.read_to_string(&mut global)
            .expect("Unable to read the stdlib global file");

        program = format!(
            "{ether} \n {global} \n {program}",
            ether = ether,
            global = global,
            program = program
        )
    } else {
        panic!("Invalid target identifier")
    }

    let (module, environment) = parser::parse_program(&program).unwrap_or_else(|err| {
        println!("Could not parse file: {}", err);
        std::process::exit(1);
    });

    ast_processor::process_ast(module, environment, target).unwrap_or_else(|err| {
        println!("Could not parse invalid flint file: {}", err);
        std::process::exit(1);
    });
}

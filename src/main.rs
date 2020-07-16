mod ast;
mod ast_processor;
mod context;
mod environment;
mod moveir;
mod parser;
mod semantic_analysis;
mod solidity;
mod type_assigner;
mod type_checker;
mod visitor;
use crate::ast_processor::Target;
use std::env;
use std::fs::File;
use std::io::prelude::*;

// TODO do not allow 'any' as a declared typestate
// TODO do not allow for duplicate typestates (e.g. Decrementable, Zero, Zero, Incrementable)
// TODO write tests for parsing typestates
// TODO check 'become' parsing
// TODO start contract in one of the states
// TODO decide how to represent the state of the contract
// TODO decide how to represent the state of the contract in move
// TODO test that the counter with states example properly works

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    if args.len() < 3 {
        panic!("Incorrect number of Arguments supplied, Expecting 2 arguments");
    }

    let target = &args[1];
    let target = if target == "libra" {
        Target::Move
    } else if target == "ether" {
        Target::Ether
    } else {
        panic!("Incorrect Target Argument specified, expecting \"ether\" or \"libra\"");
    };

    let filename = &args[2];

    let mut file =
        File::open(filename).expect(&*format!("Unable to open file at path {} ", filename));

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Unable to read the file");
    let mut program = contents.clone();

    if let Target::Move = target {
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
    } else {
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

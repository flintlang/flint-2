#[cfg(test)]
mod ewasm_tests {
    extern crate ewasm_api;
    extern crate libchisel;
    extern crate parity_wasm;
    extern crate pwasm_utils;

    use crate::io::target::target;
    use crate::{ast_processor, parser};
    use libchisel::{checkstartfunc::*, verifyexports::*, verifyimports::*};
    use libchisel::{ModulePreset, ModuleValidator};
    use std::fs;
    use std::io::Read;
    use std::path::Path;

    #[test]
    fn test_ewasm_validity() {
        // List the filenames we want to test separated by a space
        // TODO refactor this process not to rely on move test folder
        let file_names = "counter factorial shapes assert traffic_lights operators memory"
            .split(' ')
            .zip("Counter Factorial Shapes Assert TrafficLights Operators Memory".split(' '))
            .collect::<Vec<(&str, &str)>>();

        for (flint_file_name, wasm_file_name) in file_names {
            let file_path = format!("tests/move_tests/{}.flint", flint_file_name);
            let file_path = Path::new(file_path.as_str());
            let mut file = fs::File::open(file_path).expect(&*format!(
                "Unable to open file at path `{}`",
                file_path.to_str().unwrap_or("<?>")
            ));

            let mut program = String::new();
            file.read_to_string(&mut program)
                .expect("Unable to read the file");

            let (module, environment) = parser::parse_program(&program).unwrap_or_else(|err| {
                println!("Could not parse file: {}", err);
                std::process::exit(1);
            });

            ast_processor::process_ast(module, environment, target("ethereum").unwrap())
                .unwrap_or_else(|err| {
                    println!("Could not parse invalid flint file: {}", err);
                    std::process::exit(1);
                });

            let output_path = format!("output/{}.wasm", wasm_file_name);
            let output_path = Path::new(output_path.as_str());
            assert!(output_path.exists());
            let output_wasm = fs::read(output_path).expect(&format!(
                "Could not read wasm from contract `{}`",
                flint_file_name
            ));
            assert!(validate_contract(&output_wasm));
        }
    }

    fn validate_contract(module: &[u8]) -> bool {
        let module = libchisel::Module::from_bytes(module);
        if module.is_err() {
            return false;
        }
        let module = module.unwrap();

        // Ensure no start functions is present.
        if !CheckStartFunc::new(false).validate(&module).unwrap() {
            return false;
        }

        // Ensure only valid exports are present.
        if !VerifyExports::with_preset("ewasm")
            .unwrap()
            .validate(&module)
            .unwrap()
        {
            return false;
        }

        // Ensure only valid imports are used.
        if !VerifyImports::with_preset("ewasm")
            .unwrap()
            .validate(&module)
            .unwrap()
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod ewasm_tests {
    extern crate ewasm_api;
    extern crate inkwell;
    extern crate libchisel;
    extern crate parity_wasm;
    extern crate pwasm_utils;

    use self::inkwell::context::Context;
    use self::inkwell::execution_engine::{ExecutionEngine, JitFunction};
    use self::inkwell::module::Module;
    use self::inkwell::OptimizationLevel;
    use crate::io::target::target;
    use crate::{ast_processor, parser};
    use libchisel::{checkstartfunc::*, verifyexports::*, verifyimports::*};
    use libchisel::{ModulePreset, ModuleValidator};
    use std::fs;
    use std::io::Read;
    use std::path::Path;
    use crate::io::*;

    #[test]
    fn test_ewasm_validity() {
        // List the filenames we want to test separated by a space
        // TODO refactor this process not to rely on move test folder
        let input_file_names = "counter factorial shapes assert traffic_lights operators memory inits rockpaperscissors public_and_visible typestates_counter property_modification structs callerprotections_counter arrays callerprotections_lottery callerprotections_bank dynamic_check runtime_functions".split(' ');
        let output_file_names = "Counter Factorial Shapes Assert TrafficLights Operators Memory Inits RockPaperScissors MyContract Counter PropertyModification C Counter2 Arrays Lottery Bank DynamicCheck Money".split(' ');
        let runtime_tests: Vec<Option<fn(&Module)>> = vec![
            Some(counter),
            Some(factorial),
            Some(shapes),
            None,
            Some(traffic_lights),
            Some(operators),
            Some(memory),
            Some(inits),
            Some(rock_paper_scissors),
            Some(public_and_visible),
            Some(typestates_counter),
            Some(property_modification),
            Some(structs),
            Some(caller_protections_counter),
            Some(arrays),
            Some(caller_protections_lottery),
            Some(caller_protections_bank),
            Some(dynamic_check),
            None
        ];

        let test_info = input_file_names
            .zip(output_file_names)
            .collect::<Vec<(&str, &str)>>();

        for (test_no, (flint_file_name, output_file_name)) in test_info.iter().enumerate() {
            let file_path = format!("tests/move_tests/{}.flint", flint_file_name);
            let file_path = Path::new(file_path.as_str());
            let mut file = fs::File::open(file_path).expect(&*format!(
                "Unable to open file at path `{}`",
                file_path.to_str().unwrap_or("<?>")
            ));

            let mut program = String::new();
            file.read_to_string(&mut program)
                .expect("Unable to read the file");
            
            let mut file = fs::File::open("stdlib/ether/global.flint").unwrap_or_else(|err| {
                prompt::error::unable_to_open_file(Path::new("stdlib/ether/global.flint"), err)
            });
            
            file.read_to_string(&mut program).unwrap_or_else(|err| {
                prompt::error::unable_to_read_file(Path::new("stdlib/ether/global.flint"), err)
            });
        

            let (module, environment) = parser::parse_program(&program).unwrap_or_else(|err| {
                println!("Could not parse file: {}", err);
                std::process::exit(1);
            });

            ast_processor::process_ast(module, environment, target("ethereum").unwrap())
                .unwrap_or_else(|err| {
                    println!("Could not parse invalid flint file: {}", err);
                    std::process::exit(1);
                });

            // Validate ewasm
            let output_path = format!("output/{}.wasm", output_file_name);
            let output_path = Path::new(output_path.as_str());
            assert!(output_path.exists());
            let output_wasm = fs::read(output_path).expect(&format!(
                "Could not read wasm from contract `{}`",
                flint_file_name
            ));
            assert!(vaildate_ewasm(&output_wasm));

            // Test runtime LLVM
            if let Some(test_func) = runtime_tests[test_no] {
                let output_path = format!("output/{}.ll", output_file_name);
                let output_path = Path::new(output_path.as_str());
                assert!(output_path.exists());
                let buffer = inkwell::memory_buffer::MemoryBuffer::create_from_file(output_path)
                    .expect("Could not create memory buffer from LLVM file");
                let context = Context::create();
                let module = context
                    .create_module_from_ir(buffer)
                    .expect("Could not create module from memory buffer");
                test_func(&module);
            }
        }
    }

    fn vaildate_ewasm(module: &[u8]) -> bool {
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

    type VoidToVoid = unsafe extern "C" fn() -> ();

    fn set_up_tests<'a>(module: &'a Module) -> ExecutionEngine<'a> {
        let engine = module
            .create_jit_execution_engine(OptimizationLevel::None)
            .expect("Could not make engine");

        assert!(module.verify().is_ok());

        engine
    }

    fn counter(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init = engine
                .get_function::<VoidToVoid>("CounterInit")
                .expect("Could not find CounterInit");

            let getter: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("getValue")
                .expect("Could not find getter");

            init.call();

            assert_eq!(0, getter.call());

            let increment: JitFunction<VoidToVoid> = engine
                .get_function("increment")
                .expect("Could not find increment");

            let decrement: JitFunction<VoidToVoid> = engine
                .get_function("decrement")
                .expect("Could not find decrement");

            increment.call();
            assert_eq!(1, getter.call());
            increment.call();
            assert_eq!(2, getter.call());
            decrement.call();
            assert_eq!(1, getter.call());
            println!("Counter test passed");
        }
    }

    fn factorial(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init = engine
                .get_function::<VoidToVoid>("FactorialInit")
                .expect("Could not find FactorialInit");

            let getter: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("getValue")
                .expect("Could not find getter");

            init.call();

            assert_eq!(0, getter.call());

            let calculate: JitFunction<unsafe extern "C" fn(i64)> = engine
                .get_function("calculate")
                .expect("Could not find decrement");

            calculate.call(1);
            assert_eq!(1, getter.call());
            calculate.call(2);
            assert_eq!(2, getter.call());
            calculate.call(10);
            assert_eq!(3628800, getter.call());
            println!("Factorial test passed");
        }
    }

    fn shapes(module: &Module) {
        let engine = set_up_tests(&module);

        unsafe {
            let init: JitFunction<unsafe extern "C" fn(i64)> = engine
                .get_function("ShapesInit")
                .expect("Could not find ShapesInit");

            let area: JitFunction<unsafe extern "C" fn() -> i64> =
                engine.get_function("area").expect("Could not find area");

            let semi_perimeter: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("semiPerimeter")
                .expect("Could not find semiPerimeter");

            let perimeter: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("perimeter")
                .expect("Could not find perimeter");

            let smaller_width: JitFunction<unsafe extern "C" fn(i64) -> bool> = engine
                .get_function("smallerWidth")
                .expect("Could not find smallerWidth");

            init.call(10);
            assert_eq!(200, area.call());
            assert_eq!(30, semi_perimeter.call());
            assert_eq!(60, perimeter.call());
            assert!(smaller_width.call(21));
            assert!(!smaller_width.call(19));
            println!("Shapes test passed");
        }
    }

    fn operators(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("OperatorsInit")
                .expect("Could not find OperatorsInit");

            let lt: JitFunction<unsafe extern "C" fn(i64, i64) -> bool> =
                engine.get_function("lt").expect("Could not find lt");

            let plus: JitFunction<unsafe extern "C" fn(i64, i64) -> i64> =
                engine.get_function("plus").expect("Could not find plus");

            let divide: JitFunction<unsafe extern "C" fn(i64, i64) -> i64> = engine
                .get_function("divide")
                .expect("Could not find divide");

            let modulus: JitFunction<unsafe extern "C" fn(i64, i64) -> i64> = engine
                .get_function("remainder")
                .expect("Could not find remainder");

            init.call();
            assert!(lt.call(5, 10));
            assert_eq!(15, plus.call(10, 5));
            assert_eq!(2, divide.call(10, 5));
            assert_eq!(2, divide.call(11, 5));
            assert_eq!(2, modulus.call(12, 5));

            println!("Operators test passed");
        }
    }

    fn inits(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<unsafe extern "C" fn(i64, bool)> = engine
                .get_function("InitsInit")
                .expect("Could not find InitsInit");

            let get_a: JitFunction<unsafe extern "C" fn() -> i64> =
                engine.get_function("getA").expect("Could not find getA");

            let get_b: JitFunction<unsafe extern "C" fn() -> i128> =
                engine.get_function("getB").expect("Could not find getB");

            let get_s: JitFunction<unsafe extern "C" fn() -> bool> =
                engine.get_function("getS").expect("Could not find getS");

            let get_z: JitFunction<unsafe extern "C" fn() -> i128> =
                engine.get_function("getZ").expect("Could not find getZ");

            let set_t: JitFunction<unsafe extern "C" fn(i64, bool)> =
                engine.get_function("setT").expect("Could not find setT");

            let get_tx: JitFunction<unsafe extern "C" fn() -> i64> =
                engine.get_function("getTx").expect("Could not find getTx");

            let get_ty: JitFunction<unsafe extern "C" fn() -> bool> =
                engine.get_function("getTy").expect("Could not find getTy");

            let get_ts: JitFunction<unsafe extern "C" fn() -> bool> =
                engine.get_function("getTs").expect("Could not find getTs");

            init.call(10, true);
            assert_eq!(10, get_a.call());
            assert_eq!(0x1000, get_b.call());
            assert!(get_s.call());
            assert_eq!(0x72981077347248757091884308802679, get_z.call());

            set_t.call(5, true);
            assert_eq!(5, get_tx.call());
            assert!(get_ty.call());
            assert!(!get_ts.call());

            println!("Inits test passed");
        }
    }

    fn memory(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("MemoryInit")
                .expect("Could not find initialiser");

            let get_sa: JitFunction<unsafe extern "C" fn() -> u64> =
                engine.get_function("getSa").expect("Could not find getSa");

            let get_ss: JitFunction<unsafe extern "C" fn() -> u128> =
                engine.get_function("getSs").expect("Could not find getSs");

            let get_vx: JitFunction<unsafe extern "C" fn() -> u64> =
                engine.get_function("getVx").expect("Could not find getVx");

            let set_s: JitFunction<unsafe extern "C" fn(u64, u128) -> ()> =
                engine.get_function("setS").expect("Could not find setS");

            let set_v1: JitFunction<unsafe extern "C" fn(u64) -> ()> =
                engine.get_function("setV1").expect("Could not find setV1");

            let set_v2: JitFunction<unsafe extern "C" fn(u64) -> ()> =
                engine.get_function("setV2").expect("Could not find setV2");

            let set_v3: JitFunction<unsafe extern "C" fn(bool, u64, u64) -> ()> =
                engine.get_function("setV3").expect("Could not find setV3");

            init.call();

            assert_eq!(get_sa.call(), 0);
            assert_eq!(get_ss.call(), 0);
            assert_eq!(get_vx.call(), 1);

            set_s.call(1, 1);

            assert_eq!(get_sa.call(), 2);
            assert_eq!(get_ss.call(), 1);

            set_v1.call(2);
            assert_eq!(get_vx.call(), 2);

            set_v2.call(2);
            assert_eq!(get_vx.call(), 3);

            set_v3.call(true, 2, 3);
            assert_eq!(get_vx.call(), 3);

            set_v3.call(false, 2, 3);
            assert_eq!(get_vx.call(), 4);

            println!("Memory test passed");
        }
    }

    fn rock_paper_scissors(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("RockPaperScissorsInit")
                .expect("Could not find initialiser");

            let left_wins: JitFunction<unsafe extern "C" fn(i64, i64) -> ()> = engine
                .get_function("leftWins")
                .expect("Could not find leftWins");

            let get_winner: JitFunction<unsafe extern "C" fn() -> bool> = engine
                .get_function("getWinner")
                .expect("Could not find getWinner");

            init.call();

            assert_eq!(get_winner.call(), false);
            left_wins.call(0, 0);
            assert_eq!(get_winner.call(), false);
            left_wins.call(1, 1);
            assert_eq!(get_winner.call(), false);
            left_wins.call(2, 2);
            assert_eq!(get_winner.call(), false);
            left_wins.call(0, 1);
            assert_eq!(get_winner.call(), false);
            left_wins.call(0, 2);
            assert_eq!(get_winner.call(), true);
            left_wins.call(1, 0);
            assert_eq!(get_winner.call(), true);
            left_wins.call(1, 2);
            assert_eq!(get_winner.call(), false);
            left_wins.call(2, 0);
            assert_eq!(get_winner.call(), false);
            left_wins.call(2, 1);
            assert_eq!(get_winner.call(), true);

            println!("RockPaperScissors test passed");
        }
    }

    fn public_and_visible(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("MyContractInit")
                .expect("Could not find initialiser");

            let get_value: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("getValue")
                .expect("Could not find getValue");

            let get_other_value: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("getOtherValue")
                .expect("Could not find getOtherValue");

            let set_other_value: JitFunction<unsafe extern "C" fn(i64) -> ()> = engine
                .get_function("setOtherValue")
                .expect("Could not find setOtherValue");

            init.call();

            assert_eq!(get_value.call(), 0);
            assert_eq!(get_other_value.call(), 0);

            set_other_value.call(9);
            assert_eq!(get_other_value.call(), 9);

            println!("Public and Visible test passed");
        }
    }

    fn property_modification(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("PropertyModificationInit")
                .expect("Could not find initialiser");

            let get_u: JitFunction<unsafe extern "C" fn() -> i64> =
                engine.get_function("getU").expect("Could not find getU");

            let get_vx: JitFunction<unsafe extern "C" fn() -> i64> =
                engine.get_function("getVx").expect("Could not find getVx");

            init.call();
            assert_eq!(2, get_u.call());
            assert_eq!(4, get_vx.call());

            println!("Property modification test passed");
        }
    }

    pub fn structs(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("CInit")
                .expect("Could not find initialiser");

            let get_ax: JitFunction<unsafe extern "C" fn() -> i64> =
                engine.get_function("getAx").expect("Could not find getAx");

            let set_ax: JitFunction<unsafe extern "C" fn(i64) -> ()> =
                engine.get_function("setAx").expect("Could not find setAx");

            let get_ay: JitFunction<unsafe extern "C" fn() -> bool> =
                engine.get_function("getAy").expect("Could not find getAy");

            let set_ay: JitFunction<unsafe extern "C" fn(bool) -> ()> =
                engine.get_function("setAy").expect("Could not find setAy");

            let get_bxx: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("getBxx")
                .expect("Could not find getBxx");

            let set_bxx: JitFunction<unsafe extern "C" fn(i64) -> ()> = engine
                .get_function("setBxx")
                .expect("Could not find setBxx");

            let get_bxx2: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("getBxx2")
                .expect("Could not find getBxx2");

            let get_bxx3: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("getBxx3")
                .expect("Could not find getBxx3");

            let set_bxx3: JitFunction<unsafe extern "C" fn(i64) -> ()> = engine
                .get_function("setBxx3")
                .expect("Could not find setBxx");

            let get_cxx: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("getCxx")
                .expect("Could not find getCxx");

            let set_cxx: JitFunction<unsafe extern "C" fn(i64) -> ()> = engine
                .get_function("setCxx")
                .expect("Could not find setCxx");

            let set_cxx2: JitFunction<unsafe extern "C" fn(i64) -> ()> = engine
                .get_function("setCxx2")
                .expect("Could not find setCxx2");

            let get_bxy: JitFunction<unsafe extern "C" fn() -> bool> = engine
                .get_function("getBxy")
                .expect("Could not find getBxy");

            let set_bxy: JitFunction<unsafe extern "C" fn(bool) -> ()> = engine
                .get_function("setBxy")
                .expect("Could not find setBxy");

            let get_by: JitFunction<unsafe extern "C" fn() -> i64> =
                engine.get_function("getBy").expect("Could not find getBy");

            let set_by: JitFunction<unsafe extern "C" fn(i64) -> ()> =
                engine.get_function("setBy").expect("Could not find setBy");

            let get_size: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("getSize")
                .expect("Could not find getSize");

            let get: JitFunction<unsafe extern "C" fn(i64) -> i64> =
                engine.get_function("get").expect("Could not find get");

            let append: JitFunction<unsafe extern "C" fn(i64) -> ()> = engine
                .get_function("append")
                .expect("Could not find append");

            let get_d: JitFunction<unsafe extern "C" fn() -> i64> =
                engine.get_function("getD").expect("Could not find getD");

            let get_e: JitFunction<unsafe extern "C" fn() -> bool> =
                engine.get_function("getE").expect("Could not find getE");

            init.call();

            // Ax
            assert_eq!(get_ax.call(), 0);
            set_ax.call(10);
            assert_eq!(get_ax.call(), 10);

            // Ay
            assert!(!get_ay.call());
            set_ay.call(true);
            assert!(get_ay.call());

            // Bxx
            assert_eq!(get_bxx.call(), 0);
            set_bxx.call(0);
            assert_eq!(get_bxx.call(), 0);
            assert_eq!(get_bxx2.call(), 0);
            assert_eq!(get_bxx3.call(), 256);
            set_bxx3.call(5);
            assert_eq!(get_bxx.call(), 5);

            // Cxx
            assert_eq!(get_cxx.call(), 0);
            set_cxx.call(10);
            assert_eq!(get_cxx.call(), 10);
            set_cxx2.call(5);
            assert_eq!(get_cxx.call(), 5);

            // Bxy
            assert!(!get_bxy.call());
            set_bxy.call(true);
            assert!(get_bxy.call());

            // By
            assert_eq!(get_by.call(), 0);
            set_by.call(5);
            assert_eq!(get_by.call(), 5);

            // arr
            assert_eq!(get_size.call(), 0);
            assert_eq!(get.call(0), 0);
            append.call(5);
            assert_eq!(get_size.call(), 1);
            assert_eq!(get.call(0), 5);

            // D
            assert_eq!(get_d.call(), 5);

            // E
            assert!(get_e.call());

            println!("Structs test passed");
        }
    }

    fn typestates_counter(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("CounterInit")
                .expect("Could not find initialiser");

            let increment: JitFunction<unsafe extern "C" fn(i64) -> ()> = engine
                .get_function("increment")
                .expect("Could not find increment function");

            let reset: JitFunction<VoidToVoid> = engine
                .get_function("reset")
                .expect("Could not find the reset function");

            let getter: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("getCount")
                .expect("Could not find getCount");

            init.call();

            assert_eq!(getter.call(), 0);
            increment.call(1);
            assert_eq!(getter.call(), 1);
            reset.call();
            assert_eq!(getter.call(), 0);

            println!("Typestates counter test passed, up until where it should fail, which cannot be tested");

            // NOTE this should cause a SEGFAULT as we call revert, which is defined by ewasm, not us
            // so we cannot test it here TODO
            // reset.call();
        }
    }

    #[allow(unused_variables)]
    fn traffic_lights(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("TrafficLightsInit")
                .expect("Could not find TrafficLightsInit");

            let move_to_red: JitFunction<VoidToVoid> = engine
                .get_function("moveToRed")
                .expect("Could not find moveToRed");
            let move_to_amber: JitFunction<VoidToVoid> = engine
                .get_function("moveToAmber")
                .expect("Could not find moveToAmber");
            let move_to_green: JitFunction<VoidToVoid> = engine
                .get_function("moveToGreen")
                .expect("Could not find moveToGreen");

            let get_signal: JitFunction<unsafe extern "C" fn() -> u64> = engine
                .get_function("getSignal")
                .expect("Could not find getSignal");

            init.call();

            assert_eq!(get_signal.call(), 0);
            move_to_amber.call();
            assert_eq!(get_signal.call(), 1);
            move_to_green.call();
            assert_eq!(get_signal.call(), 2);

            println!(
                "Traffic lights test passed, up until where it should fail, which cannot be tested"
            );

            // NOTE this should cause a SEGFAULT as we call revert, which is defined by ewasm, not us
            // so we cannot test it here TODO
            // move_to_red.call();
        }
    }

    fn caller_protections_counter(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("Counter2Init")
                .expect("Could not find initialiser");

            let get_count: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("getCount")
                .expect("Could not find getCount");

            let get_owner: JitFunction<unsafe extern "C" fn() -> i128> = engine
                .get_function("getOwner")
                .expect("Could not find getOwner");

            let get_friend: JitFunction<unsafe extern "C" fn() -> i128> = engine
                .get_function("getFriend")
                .expect("Could not find getFriend");

            let increment: JitFunction<VoidToVoid> = engine
                .get_function("increment")
                .expect("Could not find increment");

            let switch: JitFunction<VoidToVoid> = engine
                .get_function("switch")
                .expect("Could not find switch");

            init.call();
            assert_eq!(0, get_count.call());
            assert_eq!(21267647932558653966460912964485513216, get_owner.call());
            assert_eq!(1, get_friend.call());

            switch.call();

            assert_eq!(1, get_owner.call());
            assert_eq!(21267647932558653966460912964485513216, get_friend.call());

            // this call to increment should pass the caller protections
            increment.call();
            assert_eq!(1, get_count.call());

            switch.call();

            // NOTE this should cause a SEGFAULT as we call revert, which is defined by ewasm, not us
            // so we cannot test it here TODO
            // increment.call();
            println!("Caller protections counter test passed");
        }
    }

    #[allow(unused_variables)]
    fn caller_protections_lottery(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("LotteryInit")
                .expect("Could not find initialiser");

            let get_winnings: JitFunction<unsafe extern "C" fn() -> i64> = engine
                .get_function("getWinnings")
                .expect("Could not find getWinnings");

            let get_first: JitFunction<unsafe extern "C" fn() -> i128> = engine
                .get_function("getFirstPerson")
                .expect("Could not find getFirstPerson");

            let get_second: JitFunction<unsafe extern "C" fn() -> i128> = engine
                .get_function("getSecondPerson")
                .expect("Could not find getSecondPerson");

            let get_last: JitFunction<unsafe extern "C" fn() -> i128> = engine
                .get_function("getLastPerson")
                .expect("Could not find getLastPerson");

            let out_of_bounds: JitFunction<unsafe extern "C" fn() -> i128> = engine
                .get_function("outOfBounds")
                .expect("Could not find outOfBounds");

            let first_address_is_winner: JitFunction<unsafe extern "C" fn() -> bool> = engine
                .get_function("firstAddressIsWinner")
                .expect("Could not find firstAddressIsWinner");

            let is_winner: JitFunction<unsafe extern "C" fn() -> bool> = engine
                .get_function("isWinner")
                .expect("Could not find isWinner");

            let change_address: JitFunction<VoidToVoid> = engine
                .get_function("changeAddress")
                .expect("Could not find changeAddress");

            init.call();

            assert_eq!(1000, get_winnings.call());
            assert_eq!(0, get_first.call());
            assert_eq!(1, get_second.call());
            assert_eq!(2, get_last.call());
            assert_eq!(1, get_second.call());
            assert!(!first_address_is_winner.call());
            change_address.call();
            assert_eq!(3, get_last.call());

            // NOTE this should cause a SEGFAULT as we call revert, which is defined by ewasm, not us
            // so we cannot test it here TODO
            // out_of_bounds.call();

            // NOTE this should cause a SEGFAULT as we call revert, which is defined by ewasm, not us
            // so we cannot test it here TODO
            // is_winner.call();

            println!("Caller protections lottery test passed");
        }
    }

    fn caller_protections_bank(module: &Module) {
        // tests static checking of caller protections
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("BankInit")
                .expect("Could not find initialiser");

            init.call();

            println!("Caller protections bank test passed");
        }
    }

    fn dynamic_check(module: &Module) {
        // tests dynamic checking of caller protections
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("DynamicCheckInit")
                .expect("Could not find initialiser");

            let try_bang: JitFunction<unsafe extern "C" fn(i64)> = engine
                .get_function("tryBang")
                .expect("Could not find tryBang");

            let try_question: JitFunction<unsafe extern "C" fn(i64) -> bool> = engine
                .get_function("tryQuestion")
                .expect("Could not find tryQuestion");

            init.call();
            try_bang.call(3);
            assert!(try_question.call(2));

            println!("Dynamic check test passed");
        }
    }

    fn arrays(module: &Module) {
        let engine = set_up_tests(module);

        unsafe {
            let init: JitFunction<VoidToVoid> = engine
                .get_function("ArraysInit")
                .expect("Could not find initialiser");

            let get: JitFunction<unsafe extern "C" fn(i64) -> i64> =
                engine.get_function("get").expect("Could not find get");

            let set: JitFunction<unsafe extern "C" fn(i64, i64) -> i64> =
                engine.get_function("set").expect("Could not find set");

            init.call();
            assert_eq!(get.call(0), 1);
            assert_eq!(get.call(1), 2);
            assert_eq!(get.call(2), 3);

            set.call(0, 5);
            set.call(1, 6);
            set.call(2, 7);

            assert_eq!(get.call(0), 5);
            assert_eq!(get.call(1), 6);
            assert_eq!(get.call(2), 7);

            // NOTE this should cause a SEGFAULT as we call revert, which is defined by ewasm, not us
            // so we cannot test it here TODO
            // get.call(3); // Goes out of variable bounds

            println!("Arrays test passed");
        }
    }
}

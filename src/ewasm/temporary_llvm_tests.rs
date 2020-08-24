use super::inkwell::execution_engine::{ExecutionEngine, JitFunction};
use super::inkwell::OptimizationLevel;
use crate::ewasm::codegen::Codegen;

#[allow(dead_code)]
fn set_up_tests<'a>(codegen: &'a Codegen) -> ExecutionEngine<'a> {
    let engine = codegen
        .module
        .create_jit_execution_engine(OptimizationLevel::None)
        .expect("Could not make engine");

    assert!(codegen.module.verify().is_ok());
    codegen.module.print_to_stderr();

    engine
}

#[allow(dead_code)]
pub fn counter(codegen: &Codegen) {
    let engine = set_up_tests(codegen);

    unsafe {
        type VoidToVoid = unsafe extern "C" fn() -> ();

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
    }
}

#[allow(dead_code)]
pub fn factorial(codegen: &Codegen) {
    let engine = set_up_tests(codegen);

    unsafe {
        type VoidToVoid = unsafe extern "C" fn() -> ();

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
    }
}

#[allow(dead_code)]
pub fn shapes(codegen: &Codegen) {
    let engine = set_up_tests(&codegen);

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
    }
}

#[allow(dead_code)]
pub fn operators(codegen: &Codegen) {
    let engine = set_up_tests(codegen);

    unsafe {
        type VoidToVoid = unsafe extern "C" fn() -> ();
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

        init.call();
        assert!(lt.call(5, 10));
        assert_eq!(15, plus.call(10, 5));
        assert_eq!(2, divide.call(10, 5));
        assert_eq!(2, divide.call(11, 5));
    }
}

#[allow(dead_code)]
pub fn traffic_lights(codegen: &Codegen) {
    let engine = set_up_tests(&codegen);

    unsafe {
        type VoidToVoid = unsafe extern "C" fn() -> ();
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

        // NOTE this should cause a SIGILL
        println!("We should now get a SIGILL, and should have had no errors up until now");
        move_to_red.call();
    }
}

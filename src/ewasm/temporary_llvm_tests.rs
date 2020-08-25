use super::inkwell::execution_engine::{ExecutionEngine, JitFunction};
use super::inkwell::OptimizationLevel;
use crate::ewasm::codegen::Codegen;

type VoidToVoid = unsafe extern "C" fn() -> ();

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
        println!("Test passed");
    }
}

#[allow(dead_code)]
pub fn factorial(codegen: &Codegen) {
    let engine = set_up_tests(codegen);

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
        println!("Test passed");
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
        println!("Test passed");
    }
}

#[allow(dead_code)]
pub fn operators(codegen: &Codegen) {
    let engine = set_up_tests(codegen);

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

        println!("Test passed");
    }
}

#[allow(dead_code)]
pub fn traffic_lights(codegen: &Codegen) {
    let engine = set_up_tests(&codegen);

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

        // NOTE this should cause a SIGILL
        println!("We should now get a SIGILL, and should have had no errors up until now");
        println!("If we do, then test passed!");
        move_to_red.call();
    }
}

#[allow(dead_code)]
pub fn inits(codegen: &Codegen) {
    let engine = set_up_tests(codegen);

    unsafe {
        type VoidToVoid = unsafe extern "C" fn() -> ();
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

        println!("Test passed");
    }
}

#[allow(dead_code)]
pub fn memory(codegen: &Codegen) {
    let engine = set_up_tests(codegen);

    unsafe {
        let init: JitFunction<VoidToVoid> = engine
            .get_function("MemoryInit")
            .expect("Could not find initialiser");

        let get_sa: JitFunction<unsafe extern "C" fn() -> u64> =
            engine.get_function("getSa").expect("Could not find getSa");

        // TODO u128 not big enough for an address
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

        println!("Test passed");
    }
}

#[allow(dead_code)]
pub fn rock_paper_scissors(codegen: &Codegen) {
    let engine = set_up_tests(codegen);

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

        println!("Test passed");
    }
}

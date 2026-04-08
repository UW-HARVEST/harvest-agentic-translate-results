use emlang::env::{Env, RuntimeError, GC_FREQUENCY_IN_TICKS};
use emlang::parser::Parser;
use emlang::stack;

fn run_program(input: &str) -> (Result<(), RuntimeError>, i64) {
    let mut p = Parser::new();
    p.load_mem(input);
    let r = p.parse();
    let prog = r.prog.unwrap();
    let mut env = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let result = env.run(&prog);
    match result.em {
        Ok(_) => (Ok(()), result.ex),
        Err(e) => (Err(e), result.ex),
    }
}

#[test]
fn test_add() {
    let (res, ex) = run_program("10 20 ;) ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_exit_code() {
    let (res, ex) = run_program("42 X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 42);
}

#[test]
fn test_stack_underflow() {
    let (res, _) = run_program(":P ");
    assert_eq!(res.unwrap_err(), RuntimeError::StackUnderflow);
}

#[test]
fn test_div_by_zero() {
    let (res, _) = run_program("5 0 x( ");
    assert_eq!(res.unwrap_err(), RuntimeError::DivByZero);
}

#[test]
fn test_incorrect_type_add() {
    let (res, _) = run_program("\"a\" 1 ;) ");
    assert_eq!(res.unwrap_err(), RuntimeError::IncorrectType);
}

#[test]
fn test_if_true() {
    let (res, ex) = run_program("1 :/ 99 0 X_X :\\ ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_if_false_skips_body() {
    let (res, ex) = run_program("0 :/ 99 0 X_X :\\ ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_sub() {
    let (res, ex) = run_program("10 3 ;( ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_mul() {
    let (res, ex) = run_program("10 3 x) ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_div() {
    let (res, ex) = run_program("10 3 x( ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_grt_true() {
    let (res, ex) = run_program("5 3 :> ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_grt_false() {
    let (res, ex) = run_program("3 5 :> ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_equ_true() {
    let (res, ex) = run_program("5 5 :| ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_equ_false() {
    let (res, ex) = run_program("5 5 x| ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_dup() {
    let (res, ex) = run_program("42 0 :D :P 0 X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_swap() {
    let (res, ex) = run_program("10 20 1 :S :P 0 X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_invalid_dup() {
    let (res, _) = run_program("99 :D ");
    assert_eq!(res.unwrap_err(), RuntimeError::InvalidAccess);
}

#[test]
fn test_invalid_swap() {
    let (res, _) = run_program("99 :S ");
    assert_eq!(res.unwrap_err(), RuntimeError::InvalidAccess);
}

#[test]
fn test_less_true() {
    let (res, ex) = run_program("3 5 :< ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_less_false() {
    let (res, ex) = run_program("5 3 :< ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_nequ() {
    let (res, ex) = run_program("5 3 x| ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_if_incorrect_type() {
    let (res, _) = run_program("\"a\" :/ 1 :\\ ");
    assert_eq!(res.unwrap_err(), RuntimeError::IncorrectType);
}

#[test]
fn test_loop() {
    let (res, ex) = run_program("0 1 :@ 1 ;) 0 :D 3 :< @: ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_runtime_error_display() {
    assert_eq!(format!("{}", RuntimeError::StackUnderflow), "Stack underflow");
    assert_eq!(format!("{}", RuntimeError::InvalidAccess), "Invalid access");
    assert_eq!(format!("{}", RuntimeError::DivByZero), "Division by zero");
    assert_eq!(format!("{}", RuntimeError::IncorrectType), "Incorrect type");
}

#[test]
fn test_gc_frequency_constant() {
    assert_eq!(GC_FREQUENCY_IN_TICKS, 64);
}

#[test]
fn test_env_new() {
    let env = Env::new(1024, 32);
    assert_eq!(env.ip, 0);
    assert_eq!(env.ex, 0);
    assert_eq!(env.tick, 0);
    assert_eq!(env.halt, false);
    assert_eq!(env.print, false);
    assert_eq!(env.print_from, 0);
}

#[test]
fn test_exit_with_value() {
    // Push 5, exit -> exit code 5
    let (res, ex) = run_program("5 X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 5);
}

#[test]
fn test_arithmetic_result_on_stack() {
    // 10 + 20 = 30, then exit with that value
    let (res, ex) = run_program("10 20 ;) X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 30);
}

#[test]
fn test_sub_result() {
    let (res, ex) = run_program("10 3 ;( X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 7);
}

#[test]
fn test_mul_result() {
    let (res, ex) = run_program("10 3 x) X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 30);
}

#[test]
fn test_div_result() {
    let (res, ex) = run_program("10 3 x( X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 3);
}

#[test]
fn test_grt_result_true() {
    let (res, ex) = run_program("5 3 :> X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_grt_result_false() {
    let (res, ex) = run_program("3 5 :> X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_less_result_true() {
    let (res, ex) = run_program("3 5 :< X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_less_result_false() {
    let (res, ex) = run_program("5 3 :< X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_equ_result_true() {
    let (res, ex) = run_program("5 5 :| X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_equ_result_false() {
    let (res, ex) = run_program("5 3 :| X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_nequ_result_true() {
    let (res, ex) = run_program("5 3 x| X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_nequ_result_false() {
    let (res, ex) = run_program("5 5 x| X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_dup_result() {
    // push 42, push 0, dup -> stack has [42, 42], add -> 84, exit
    let (res, ex) = run_program("42 0 :D ;) X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 84);
}

#[test]
fn test_swap_result() {
    // push 10, push 20, push 1, swap -> [20, 10], pop 10, exit with 20
    let (res, ex) = run_program("10 20 1 :S :P X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 20);
}

#[test]
fn test_loop_count() {
    // Count from 0 to 3: push 0, push 1, loop: add 1, dup, compare < 3, end loop
    // After loop, stack top = 3
    let (res, ex) = run_program("0 1 :@ 1 ;) 0 :D 3 :< @: X_X ");
    assert!(res.is_ok());
    assert_eq!(ex, 3);
}

fn main() {}

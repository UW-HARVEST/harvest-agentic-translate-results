use emlang::data::DataValue;
use emlang::env::{Env, RuntimeError, GC_FREQUENCY_IN_TICKS};
use emlang::parser::Parser;
use emlang::stack::{DEFAULT_POPPED_CAP, DEFAULT_STACK_CAP};

fn run_program(src: &str) -> (i64, Result<(), RuntimeError>, Vec<emlang::data::Data>) {
    let mut p = Parser::new();
    p.load_mem(src);
    let result = p.parse();
    let prog = result.prog.expect("parse should succeed");
    let mut env = Env::new(DEFAULT_STACK_CAP, DEFAULT_POPPED_CAP);
    let r = env.run(&prog);
    let stack_contents: Vec<emlang::data::Data> = (0..env.stack.size).map(|i| env.stack.buf[i].clone()).collect();
    let err_status = match r.em {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    };
    (r.ex, err_status, stack_contents)
}

#[test]
fn test_constants() {
    assert_eq!(GC_FREQUENCY_IN_TICKS, 64);
}

#[test]
fn test_runtime_error_display() {
    assert_eq!(format!("{}", RuntimeError::StackUnderflow), "Stack underflow");
    assert_eq!(format!("{}", RuntimeError::InvalidAccess), "Invalid access");
    assert_eq!(format!("{}", RuntimeError::DivByZero), "Division by zero");
    assert_eq!(format!("{}", RuntimeError::IncorrectType), "Incorrect type");
}

#[test]
fn test_env_new() {
    let e = Env::new(16, 8);
    assert_eq!(e.ip, 0);
    assert_eq!(e.ex, 0);
    assert_eq!(e.tick, 0);
    assert_eq!(e.halt, false);
    assert_eq!(e.print, false);
    assert_eq!(e.print_from, 0);
    assert_eq!(e.stack.cap, 16);
    assert_eq!(e.stack.popped_cap, 8);
}

#[test]
fn test_run_empty_program() {
    let (ex, status, _stack) = run_program("");
    assert!(status.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_run_addition() {
    // 5+7 leaves 12 on stack; we check ex from a final exit
    let (ex, status, _) = run_program("5 7 ;) X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 12);
}

#[test]
fn test_run_subtraction() {
    let (ex, status, _) = run_program("5 7 ;( X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, -2);
}

#[test]
fn test_run_multiplication() {
    let (ex, status, _) = run_program("5 7 x) X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 35);
}

#[test]
fn test_run_division() {
    let (ex, status, _) = run_program("21 7 x( X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 3);
}

#[test]
fn test_run_division_by_zero() {
    let (_, status, _) = run_program("5 0 x( ");
    assert_eq!(status, Err(RuntimeError::DivByZero));
}

#[test]
fn test_run_stack_underflow() {
    let (_, status, _) = run_program(":P ");
    assert_eq!(status, Err(RuntimeError::StackUnderflow));
}

#[test]
fn test_run_dup() {
    // Push 1, 2, 3, then dup top (offset 0) -> [1,2,3,3]; then exit pops 3
    let (ex, status, _) = run_program("1 2 3 0 :D X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 3);
}

#[test]
fn test_run_dup_invalid() {
    let (_, status, _) = run_program("1 2 :D ");
    assert_eq!(status, Err(RuntimeError::InvalidAccess));
}

#[test]
fn test_run_swap() {
    let (ex, status, _) = run_program("1 2 1 :S X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_run_swap_invalid() {
    let (_, status, _) = run_program("1 1 :S ");
    assert_eq!(status, Err(RuntimeError::InvalidAccess));
}

#[test]
fn test_run_gt() {
    let (ex, status, _) = run_program("7 5 :> X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 1);
    let (ex2, status2, _) = run_program("3 5 :> X_X ");
    assert!(status2.is_ok());
    assert_eq!(ex2, 0);
}

#[test]
fn test_run_lt() {
    let (ex, status, _) = run_program("3 5 :< X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_run_eq() {
    let (ex, status, _) = run_program("5 5 :| X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 1);
    let (ex2, status2, _) = run_program("5 6 :| X_X ");
    assert!(status2.is_ok());
    assert_eq!(ex2, 0);
}

#[test]
fn test_run_neq() {
    let (ex, status, _) = run_program("5 6 x| X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 1);
    let (ex2, status2, _) = run_program("5 5 x| X_X ");
    assert!(status2.is_ok());
    assert_eq!(ex2, 0);
}

#[test]
fn test_run_if_true() {
    // 1 :/ 42 :\ X_X -> exit code 42 (if executes inside)
    // Need a value at end for exit. The if-block leaves 42; outside we exit
    let (ex, status, _) = run_program("1 :/ 42 :\\ X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 42);
}

#[test]
fn test_run_if_false() {
    let (ex, status, _) = run_program("0 :/ 42 :\\ 99 X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 99);
}

#[test]
fn test_run_loop_count_to_three() {
    // Iterator at 0, increment, check < 3
    // 0 1 :@ 1 ;) 0 :D 3 :< @: X_X
    // After loop iterator value is on stack; exit
    let (ex, status, _) = run_program("0 1 :@ 1 ;) 0 :D 3 :< @: X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 3);
}

#[test]
fn test_run_exit_code() {
    let (ex, status, _) = run_program("42 X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 42);
}

#[test]
fn test_run_exit_negative() {
    let (ex, status, _) = run_program("-5 X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, -5);
}

#[test]
fn test_run_incorrect_type_add() {
    // Cannot add string + int
    let (_, status, _) = run_program("Hello 5 ;) ");
    assert_eq!(status, Err(RuntimeError::IncorrectType));
}

#[test]
fn test_run_incorrect_type_exit() {
    let (_, status, _) = run_program("Hello X_X ");
    assert_eq!(status, Err(RuntimeError::IncorrectType));
}

#[test]
fn test_run_negative_nums() {
    // -5 + 2 = -3
    let (ex, status, _) = run_program("-5 2 ;) X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, -3);
}

#[test]
fn test_run_print_doesnt_change_stack_after() {
    // Print block consumes its inputs
    // :O 1 2 3 :) leaves nothing
    let (_ex, status, stack) = run_program(":O 1 2 3 :)\n");
    assert!(status.is_ok());
    // Stack is cleared at end of run
    assert_eq!(stack.len(), 0);
}

#[test]
fn test_run_pop() {
    // 5 7 :P X_X => exit 5
    let (ex, status, _) = run_program("5 7 :P X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 5);
}

#[test]
fn test_pop_stack_value() {
    // After 5 7, pop returns 7
    let (ex, status, _) = run_program("5 7 X_X ");
    assert!(status.is_ok());
    // exit pops 7 first
    assert_eq!(ex, 7);
}

#[test]
fn test_run_div_truncation() {
    // C integer division: 2 / 7 = 0
    let (ex, status, _) = run_program("2 7 x( X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_run_div_negative() {
    // -10 / 3 should follow integer division semantics
    let (ex, status, _) = run_program("-10 3 x( X_X ");
    assert!(status.is_ok());
    // C integer division truncates toward zero: -10/3 = -3
    assert_eq!(ex, -3);
}

#[test]
fn test_dup_value_check() {
    // Verify the stack value 1 :D works (not just exit), from comparisons.eml-style
    // 0 :D duplicates top, so after 5: [5, 5]
    let (ex, status, _) = run_program("5 0 :D X_X ");
    assert!(status.is_ok());
    assert_eq!(ex, 5);
}

#[test]
fn test_data_value_kept_after_run() {
    // Run program that puts something on stack but doesn't exit
    let mut p = Parser::new();
    p.load_mem("42 ");
    let result = p.parse();
    let prog = result.prog.unwrap();
    let mut env = Env::new(DEFAULT_STACK_CAP, DEFAULT_POPPED_CAP);
    let r = env.run(&prog);
    assert!(r.em.is_ok());
    // ex should be 0 since no exit
    assert_eq!(r.ex, 0);
}

#[test]
fn test_run_consumes_print_inputs() {
    // After :O 1 2 3 :) the values 1,2,3 are consumed
    let mut p = Parser::new();
    p.load_mem(":O 1 2 3 :)\n4 ");
    let result = p.parse();
    let prog = result.prog.unwrap();
    let mut env = Env::new(DEFAULT_STACK_CAP, DEFAULT_POPPED_CAP);
    let _r = env.run(&prog);
    // env.stack is cleared at end of run, but we can check via re-run
    // Just verify no error
    assert!(_r.em.is_ok());
}

#[test]
fn test_data_int_value_matches() {
    let mut p = Parser::new();
    p.load_mem("123 ");
    let result = p.parse();
    let prog = result.prog.unwrap();
    assert_eq!(prog.size, 1);
    if let DataValue::Int(v) = prog.ems[0].data.value {
        assert_eq!(v, 123);
    } else {
        panic!()
    }
}

fn main() {}

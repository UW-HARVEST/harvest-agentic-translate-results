use emlang::data::Data;
use emlang::em::{Em, EmType, Program};
use emlang::env::{Env, RuntimeError};
use emlang::parser::Parser;
use emlang::stack;

fn parse_mem(input: &str) -> Program {
    let mut p = Parser::new();
    p.load_mem(input);
    let r = p.parse();
    r.prog.unwrap()
}

fn run_mem(input: &str) -> (i64, Result<(), RuntimeError>) {
    let prog = parse_mem(input);
    let mut e = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    let status = match &r.em {
        Ok(_) => Ok(()),
        Err(err) => Err(*err),
    };
    (r.ex, status)
}

// --- Exit code tests ---

#[test]
fn test_exit_zero() {
    let (ex, res) = run_mem("0 X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_exit_nonzero() {
    let (ex, res) = run_mem("1 X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_exit_code_42() {
    let (ex, res) = run_mem("42 X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 42);
}

// --- Stack underflow ---

#[test]
fn test_pop_empty_stack() {
    let (_, res) = run_mem(":P\n");
    assert_eq!(res.unwrap_err(), RuntimeError::StackUnderflow);
}

#[test]
fn test_add_underflow() {
    let (_, res) = run_mem("1 ;)\n");
    assert_eq!(res.unwrap_err(), RuntimeError::StackUnderflow);
}

// --- Arithmetic ---

#[test]
fn test_add() {
    let (ex, res) = run_mem("2 7 ;) X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 9);
}

#[test]
fn test_sub() {
    let (ex, res) = run_mem("2 7 ;( X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, -5);
}

#[test]
fn test_mul() {
    let (ex, res) = run_mem("2 7 x) X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 14);
}

#[test]
fn test_div() {
    let (ex, res) = run_mem("14 7 x( X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 2);
}

#[test]
fn test_div_truncates() {
    let (ex, res) = run_mem("2 7 x( X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_div_by_zero() {
    let (_, res) = run_mem("5 0 x(\n");
    assert_eq!(res.unwrap_err(), RuntimeError::DivByZero);
}

// --- Comparisons ---

#[test]
fn test_grt_true() {
    let (ex, res) = run_mem("7 2 :> X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_grt_false() {
    let (ex, res) = run_mem("2 7 :> X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_less_true() {
    let (ex, res) = run_mem("2 7 :< X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_less_false() {
    let (ex, res) = run_mem("7 2 :< X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_equ_true() {
    let (ex, res) = run_mem("5 5 :| X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_equ_false() {
    let (ex, res) = run_mem("5 6 :| X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_nequ_true() {
    let (ex, res) = run_mem("5 6 x| X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_nequ_false() {
    let (ex, res) = run_mem("5 5 x| X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

// --- Type errors ---

#[test]
fn test_add_type_error() {
    let (_, res) = run_mem("\"a\" 1 ;)\n");
    assert_eq!(res.unwrap_err(), RuntimeError::IncorrectType);
}

#[test]
fn test_if_type_error() {
    let (_, res) = run_mem("\"a\" :/ :\\\n");
    assert_eq!(res.unwrap_err(), RuntimeError::IncorrectType);
}

// --- If/else ---

#[test]
fn test_if_true_branch() {
    let (ex, res) = run_mem("1 :/ 42 X_X :\\\n");
    assert!(res.is_ok());
    assert_eq!(ex, 42);
}

#[test]
fn test_if_false_branch() {
    let (ex, res) = run_mem("0 :/ 99 X_X :\\ 0 X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

// --- Loop ---

#[test]
fn test_loop_basic() {
    // Start with 0 on stack, push 1 (loop cond), loop: add 1, dup, check < 3
    let (ex, res) = run_mem("0 1 :@ 1 ;) 0 :D 3 :< @: 0 :D X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 3);
}

#[test]
fn test_loop_zero_iterations() {
    let (ex, res) = run_mem("0 :@ 99 X_X @: 0 X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

// --- Dup ---

#[test]
fn test_dup() {
    let (ex, res) = run_mem("5 0 :D ;) X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 10);
}

#[test]
fn test_dup_invalid_access() {
    let (_, res) = run_mem("5 5 :D\n");
    assert_eq!(res.unwrap_err(), RuntimeError::InvalidAccess);
}

// --- Swap ---

#[test]
fn test_swap() {
    // Push 1, 2; swap(1) swaps buf[0] and buf[1] -> [2,1]; pop 1 -> [2]; exit 2
    let (ex, res) = run_mem("1 2 1 :S :P X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 2);
}

#[test]
fn test_swap_invalid_access() {
    let (_, res) = run_mem("1 5 :S\n");
    assert_eq!(res.unwrap_err(), RuntimeError::InvalidAccess);
}

// --- Pop ---

#[test]
fn test_pop() {
    let (ex, res) = run_mem("1 2 :P X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

// --- RuntimeError display ---

#[test]
fn test_runtime_error_display() {
    assert_eq!(format!("{}", RuntimeError::StackUnderflow), "Stack underflow");
    assert_eq!(format!("{}", RuntimeError::InvalidAccess), "Invalid access");
    assert_eq!(format!("{}", RuntimeError::DivByZero), "Division by zero");
    assert_eq!(format!("{}", RuntimeError::IncorrectType), "Incorrect type");
}

// --- Integration: parse file and run ---

#[test]
fn test_run_comments_eml() {
    let mut p = Parser::new();
    assert_eq!(p.load_file("resources/tests/comments.eml"), 0);
    let prog = p.parse().prog.unwrap();
    let mut e = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 0);
}

#[test]
fn test_run_hello_world_eml() {
    let mut p = Parser::new();
    assert_eq!(p.load_file("resources/tests/hello_world.eml"), 0);
    let prog = p.parse().prog.unwrap();
    let mut e = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 0);
}

#[test]
fn test_run_math_eml() {
    let mut p = Parser::new();
    assert_eq!(p.load_file("resources/tests/math.eml"), 0);
    let prog = p.parse().prog.unwrap();
    let mut e = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 0);
}

#[test]
fn test_run_comparisons_eml() {
    let mut p = Parser::new();
    assert_eq!(p.load_file("resources/tests/comparisons.eml"), 0);
    let prog = p.parse().prog.unwrap();
    let mut e = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 0);
}

#[test]
fn test_run_count_to_10_eml() {
    let mut p = Parser::new();
    assert_eq!(p.load_file("resources/tests/count_to_10.eml"), 0);
    let prog = p.parse().prog.unwrap();
    let mut e = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 0);
}

#[test]
fn test_run_if_eml() {
    let mut p = Parser::new();
    assert_eq!(p.load_file("resources/tests/if.eml"), 0);
    let prog = p.parse().prog.unwrap();
    let mut e = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 0);
}

#[test]
fn test_run_negative_nums_eml() {
    let mut p = Parser::new();
    assert_eq!(p.load_file("resources/tests/negative_nums.eml"), 0);
    let prog = p.parse().prog.unwrap();
    let mut e = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 0);
}

#[test]
fn test_run_error_eml() {
    let mut p = Parser::new();
    assert_eq!(p.load_file("resources/tests/error.eml"), 0);
    let prog = p.parse().prog.unwrap();
    let mut e = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 1);
}

#[test]
fn test_run_runtime_error_eml() {
    let mut p = Parser::new();
    assert_eq!(p.load_file("resources/tests/runtime_error.eml"), 0);
    let prog = p.parse().prog.unwrap();
    let mut e = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    assert!(r.em.is_err());
    assert_eq!(r.em.unwrap_err(), RuntimeError::DivByZero);
}

// --- Manual program construction ---

#[test]
fn test_manual_push_exit() {
    let mut prog = Program::new(8);
    prog.push(Em::new_with_data(EmType::Push, Data::new_int(7)));
    prog.push(Em::new(EmType::Exit));
    let mut e = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 7);
}

#[test]
fn test_empty_program() {
    let prog = Program::new(8);
    let mut e = Env::new(stack::DEFAULT_STACK_CAP, stack::DEFAULT_POPPED_CAP);
    let r = e.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 0);
}

#[test]
fn test_gc_frequency() {
    let mut input = String::new();
    for _ in 0..70 {
        input.push_str("\"x\" :P ");
    }
    input.push_str("0 X_X\n");
    let (ex, res) = run_mem(&input);
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

fn main() {}

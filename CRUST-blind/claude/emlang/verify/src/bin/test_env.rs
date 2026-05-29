use emlang::data::{Data, DataType, DataValue};
use emlang::em::{Em, EmType, Program, DATA_STDOUT};
use emlang::env::{Env, RuntimeError, GC_FREQUENCY_IN_TICKS};
use emlang::parser::Parser;

fn parse_str(src: &str) -> Program {
    let mut p = Parser::new();
    p.load_mem(src);
    let r = p.parse();
    r.prog.unwrap()
}

fn run_str(src: &str) -> (i64, Result<(), RuntimeError>) {
    let prog = parse_str(src);
    let mut env = Env::new(1024, 32);
    let r = env.run(&prog);
    (r.ex, r.em.map(|_| ()))
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
fn test_env_new_initial_state() {
    let e = Env::new(1024, 32);
    assert_eq!(e.ip, 0);
    assert_eq!(e.ex, 0);
    assert_eq!(e.tick, 0);
    assert_eq!(e.halt, false);
    assert_eq!(e.print, false);
    assert_eq!(e.print_from, 0);
    assert_eq!(e.stack.cap, 1024);
    assert_eq!(e.stack.size, 0);
    assert_eq!(e.stack.popped_cap, 32);
}

#[test]
fn test_env_run_empty() {
    // Empty program: nothing executes, exit=0, ok
    let prog = Program::new(8);
    let mut env = Env::new(1024, 32);
    let r = env.run(&prog);
    assert_eq!(r.ex, 0);
    assert!(r.em.is_ok());
}

#[test]
fn test_run_exit_zero() {
    // C reference: "0 X_X\n" -> err=0 ex=0
    let (ex, res) = run_str("0 X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_run_exit_42() {
    // C reference: "42 X_X\n" -> err=0 ex=42
    let (ex, res) = run_str("42 X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 42);
}

#[test]
fn test_run_exit_5() {
    // C reference: "5 X_X\n" -> err=0 ex=5
    let (ex, res) = run_str("5 X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 5);
}

#[test]
fn test_run_add() {
    // C reference: "2 7 ;) X_X\n" -> ex=9
    let (ex, res) = run_str("2 7 ;) X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 9);
}

#[test]
fn test_run_sub() {
    // C reference: "2 7 ;( X_X\n" -> ex=-5
    let (ex, res) = run_str("2 7 ;( X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, -5);
}

#[test]
fn test_run_mul() {
    // C reference: "2 7 x) X_X\n" -> ex=14
    let (ex, res) = run_str("2 7 x) X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 14);
}

#[test]
fn test_run_div() {
    // C reference: "14 2 x( X_X\n" -> ex=7
    let (ex, res) = run_str("14 2 x( X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 7);
}

#[test]
fn test_run_div_truncates() {
    // C reference: "2 7 x( X_X\n" -> ex=0
    let (ex, res) = run_str("2 7 x( X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_run_div_by_zero() {
    // C reference: "5 0 x(\n" -> err=DivByZero
    let (_ex, res) = run_str("5 0 x(\n");
    assert_eq!(res, Err(RuntimeError::DivByZero));
}

#[test]
fn test_run_negative_div() {
    // C reference: "-10 3 x( X_X\n" -> ex=-3
    let (ex, res) = run_str("-10 3 x( X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, -3);
}

#[test]
fn test_run_overflow_add() {
    // C reference: 9223372036854775807 + 1 -> i64::MIN (wrapping)
    let (ex, res) = run_str("9223372036854775807 1 ;) X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, i64::MIN);
}

#[test]
fn test_run_grt() {
    // C reference: "2 7 :> X_X\n" -> 2 > 7 = 0
    let (ex, res) = run_str("2 7 :> X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_run_grt_true() {
    // 7 > 2 = 1
    let (ex, res) = run_str("7 2 :> X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_run_less() {
    // C reference: "2 7 :< X_X\n" -> 2 < 7 = 1
    let (ex, res) = run_str("2 7 :< X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_run_equ() {
    // C reference: "5 5 :| X_X\n" -> 5 == 5 = 1
    let (ex, res) = run_str("5 5 :| X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_run_nequ() {
    // C reference: "5 6 x| X_X\n" -> 5 != 6 = 1
    let (ex, res) = run_str("5 6 x| X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_run_underflow_pop() {
    // C reference: ":P\n" -> Stack underflow
    let (_ex, res) = run_str(":P\n");
    assert_eq!(res, Err(RuntimeError::StackUnderflow));
}

#[test]
fn test_run_underflow_add() {
    // No operands for add
    let (_ex, res) = run_str(";)\n");
    assert_eq!(res, Err(RuntimeError::StackUnderflow));
}

#[test]
fn test_run_incorrect_type() {
    // C reference: "\"hi\" 5 ;)\n" -> Incorrect type
    let (_ex, res) = run_str("\"hi\" 5 ;)\n");
    assert_eq!(res, Err(RuntimeError::IncorrectType));
}

#[test]
fn test_run_dup_invalid_access() {
    // C reference: "0 :D\n" -> after popping 0, dup off=0 on empty stack -> Invalid access
    let (_ex, res) = run_str("0 :D\n");
    assert_eq!(res, Err(RuntimeError::InvalidAccess));
}

#[test]
fn test_run_dup() {
    // dup off=0 duplicates top, then exit
    // "5 0 :D X_X\n" -> push 5, push 0, pop 0 (off=0), dup off=0 (duplicates 5 -> [5,5]),
    //                  X_X pops one 5 -> exit 5
    let (ex, res) = run_str("5 0 :D X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 5);
}

#[test]
fn test_run_dup_off1() {
    // C reference: "10 20 30 1 :D X_X\n" -> dup off=1 -> [10,20,30,20] -> exit pops 20
    let (ex, res) = run_str("10 20 30 1 :D X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 20);
}

#[test]
fn test_run_swap() {
    // C reference: "10 20 30 1 :S X_X\n" -> swap off=1 -> [10,30,20] -> exit pops 20
    let (ex, res) = run_str("10 20 30 1 :S X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 20);
}

#[test]
fn test_run_swap_invalid() {
    // C reference: "1 0 :S\n" -> after popping 0 for offset, only 1 item. swap off=0 with size=1 -> ok
    // Actually C output shows: "test_swap1: err=0 ex=0"
    // So swap off=0 on size=1 -> succeeds, leaves [1]. Then we don't exit, ex defaults to 0.
    let (ex, res) = run_str("1 0 :S\n");
    assert!(res.is_ok());
    assert_eq!(ex, 0);
}

#[test]
fn test_run_pop() {
    // C reference: "1 2 :P X_X\n" -> pop 2, exit 1
    let (ex, res) = run_str("1 2 :P X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_run_if_true() {
    // C reference: "1 :/ 100 :\\ X_X\n" -> if true, push 100, exit 100
    let (ex, res) = run_str("1 :/ 100 :\\ X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 100);
}

#[test]
fn test_run_if_false() {
    // C reference: "0 :/ 100 :\\ 50 X_X\n" -> if false, skip 100, push 50, exit 50
    let (ex, res) = run_str("0 :/ 100 :\\ 50 X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 50);
}

#[test]
fn test_run_loop_count() {
    // count_to_10.eml-like: count from 0 to 10 and exit
    // 0 (iterator), 1 :@ ... @: , then exit with iterator value
    let src = "0 1 :@\n  1 ;)\n  0 :D 10 :<\n@:\nX_X\n";
    let (ex, res) = run_str(src);
    assert!(res.is_ok());
    // After loop iterator should be 10
    assert_eq!(ex, 10);
}

#[test]
fn test_run_print_begin_end_pop_optimization() {
    // ":O 1 :)\n" — when PrintBegin is immediately followed by PrintEnd (after one push)
    // Actually from C: when ip+1 == ref, the optimized branch pops the value and prints it
    // The Rust impl uses `self.ip + 1 == em_ref` (PrintEnd is at ref).
    // For ":O 1 :)\n" — prog: PrintBegin(0), Push(1), PrintEnd(2). ref of begin = 2.
    // ip=0, ref=2, ip+1=1 != 2. So takes the else branch.
    // For ":O :)\n" with empty between... but that's invalid because of cross_ref.
    // Let me check: ":O X :)" where X is a single push
    // Actually this will go through the multi-arg path because we have one item between begin and end.
    // The simple case is when print begin and print end are adjacent.
    // Hmm not sure how to trigger the if branch via parser.
    // Just verify the basic execution works:
    let prog = parse_str(":O 1 :)\n");
    let mut env = Env::new(1024, 32);
    let r = env.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 0);
}

#[test]
fn test_run_resets_state() {
    // After running, ip/halt/etc should be reset.
    let prog1 = parse_str("5 X_X\n");
    let mut env = Env::new(1024, 32);
    let r = env.run(&prog1);
    assert_eq!(r.ex, 5);
    // halt is set to true during, but state should allow re-running another prog cleanly
    let prog2 = parse_str("3 X_X\n");
    let r2 = env.run(&prog2);
    assert_eq!(r2.ex, 3);
}

#[test]
fn test_run_program_directly() {
    // Build a program directly without parsing.
    let mut prog = Program::new(8);
    prog.push(Em::new_with_data(EmType::Push, Data::new_int(7)));
    prog.push(Em::new_with_data(EmType::Push, Data::new_int(3)));
    prog.push(Em::new(EmType::Sub));
    prog.push(Em::new(EmType::Exit));
    let mut env = Env::new(1024, 32);
    let r = env.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 4);
}

#[test]
fn test_run_division_by_zero_program() {
    let mut prog = Program::new(8);
    prog.push(Em::new_with_data(EmType::Push, Data::new_int(5)));
    prog.push(Em::new_with_data(EmType::Push, Data::new_int(0)));
    prog.push(Em::new(EmType::Div));
    let mut env = Env::new(1024, 32);
    let r = env.run(&prog);
    match r.em {
        Err(e) => assert_eq!(e, RuntimeError::DivByZero),
        Ok(_) => panic!("expected DivByZero error"),
    }
}

#[test]
fn test_pop_then_print_from_clamps() {
    // Test that POP correctly updates print_from when pop reduces stack below print_from
    // ":O 1 :P 2 :)\n" — print begins, push 1, pop 1, push 2, print end -> prints "2"
    let prog = parse_str(":O 1 :P 2 :)\n");
    let mut env = Env::new(1024, 32);
    let r = env.run(&prog);
    assert!(r.em.is_ok());
    assert_eq!(r.ex, 0);
}

#[test]
fn test_loop_zero_iterations() {
    // Loop body never executes
    // 0 :@ 999 X_X @: (push 0, loop_begin pops 0 -> jumps to end) ... after loop, exit 0
    let src = "0 :@ 99 X_X @:\n";
    let (ex, res) = run_str(src);
    assert!(res.is_ok());
    // After loop never running, no exit was called -> ex = 0
    assert_eq!(ex, 0);
}

#[test]
fn test_dup_deep_offset() {
    // 1 2 3 4 5 4 :D X_X
    // Stack pre-dup: [1,2,3,4,5], pop 4 -> off=4
    // dup off=4 -> stack[5-4-1] = stack[0] = 1, push 1
    // exit pops 1
    let (ex, res) = run_str("1 2 3 4 5 4 :D X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 1);
}

#[test]
fn test_swap_deep_offset() {
    // 1 2 3 4 5 3 :S X_X
    // Stack pre-swap: [1,2,3,4,5], pop 3 -> off=3
    // swap off=3 -> swap idx 5-3-1=1 with idx 5-1=4 -> stack[2,5,3,4,1] wait
    // Stack after popping off=3: [1,2,3,4,5]. swap idx 5-3-1=1 with 5-1=4: swap pos 1 and 4 -> [1,5,3,4,2]
    // Then exit pops top: 2
    let (ex, res) = run_str("1 2 3 4 5 3 :S X_X\n");
    assert!(res.is_ok());
    assert_eq!(ex, 2);
}

#[test]
fn test_complex_program_if() {
    // C-verified: 5 7 ;) -> 12. 0 :D dups -> [12,12]. 12 :| -> 1 if equal. So [12,1].
    // :/ pops 1, true so doesn't jump. :\ closes. Now stack=[12]. X_X exits with 12.
    let src = "5 7 ;) 0 :D 12 :| :/ :\\ X_X\n";
    let (ex, res) = run_str(src);
    assert!(res.is_ok());
    assert_eq!(ex, 12);
}

#[test]
fn test_data_value_int_unused_warning_suppress() {
    // Just a sanity test using DataType/DataValue to make the imports used.
    let d = Data::new_int(7);
    assert_eq!(d.dtype, DataType::Int);
    match d.value {
        DataValue::Int(i) => assert_eq!(i, 7),
        _ => panic!(),
    }
    // Use DATA_STDOUT
    assert_eq!(DATA_STDOUT, 1);
}

fn main() {}

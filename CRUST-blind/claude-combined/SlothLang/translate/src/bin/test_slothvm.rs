use SlothLang::parser;
use SlothLang::slothvm::{self, SlothProgram};

fn make_prog(codes: Vec<u8>) -> Option<SlothProgram> {
    Some(SlothProgram { codes, pc: 0 })
}

#[test]
fn test_execute_exit_empty_stack_returns_zero() {
    let mut prog = make_prog(vec![0x00]);
    assert_eq!(slothvm::execute(&mut prog), 0);
}

#[test]
fn test_execute_push_then_exit_returns_pushed_value() {
    let mut prog = make_prog(vec![0x01, 5, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 5);
}

#[test]
fn test_execute_add() {
    // PUSH 5, PUSH 3, ADD, EXIT -> 8
    let mut prog = make_prog(vec![0x01, 5, 0x01, 3, 0x02, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 8);
}

#[test]
fn test_execute_sub() {
    // PUSH 10, PUSH 4, SUB, EXIT -> 6
    let mut prog = make_prog(vec![0x01, 10, 0x01, 4, 0x03, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 6);
}

#[test]
fn test_execute_sub_negative() {
    // PUSH 3, PUSH 5, SUB, EXIT -> -2
    let mut prog = make_prog(vec![0x01, 3, 0x01, 5, 0x03, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), -2);
}

#[test]
fn test_execute_mult() {
    // PUSH 6, PUSH 7, MULT, EXIT -> 42
    let mut prog = make_prog(vec![0x01, 6, 0x01, 7, 0x04, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 42);
}

#[test]
fn test_execute_mult_127_127() {
    // PUSH 127, PUSH 127, MULT, EXIT -> 16129
    let mut prog = make_prog(vec![0x01, 127, 0x01, 127, 0x04, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 16129);
}

#[test]
fn test_execute_div() {
    // PUSH 20, PUSH 4, DIV, EXIT -> 5
    let mut prog = make_prog(vec![0x01, 20, 0x01, 4, 0x05, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 5);
}

#[test]
fn test_execute_comp_eq() {
    // PUSH 5, PUSH 5, COMP, EQ, EXIT -> 1
    let mut prog = make_prog(vec![0x01, 5, 0x01, 5, 0x06, 0x01, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_execute_comp_neq_same() {
    // PUSH 5, PUSH 5, COMP, NEQ, EXIT -> 0
    let mut prog = make_prog(vec![0x01, 5, 0x01, 5, 0x06, 0x02, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 0);
}

#[test]
fn test_execute_comp_lt_true() {
    // PUSH 3, PUSH 5, COMP, LT, EXIT -> 1
    let mut prog = make_prog(vec![0x01, 3, 0x01, 5, 0x06, 0x03, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_execute_comp_le() {
    // PUSH 5, PUSH 7, COMP, LE, EXIT -> 1
    let mut prog = make_prog(vec![0x01, 5, 0x01, 7, 0x06, 0x04, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_execute_comp_gt_true() {
    // PUSH 7, PUSH 5, COMP, GT, EXIT -> 1
    let mut prog = make_prog(vec![0x01, 7, 0x01, 5, 0x06, 0x05, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_execute_comp_ge_equal() {
    // PUSH 5, PUSH 5, COMP, GE, EXIT -> 1
    let mut prog = make_prog(vec![0x01, 5, 0x01, 5, 0x06, 0x06, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_execute_dup() {
    // PUSH 7, DUP, ADD, EXIT -> 14
    let mut prog = make_prog(vec![0x01, 7, 0x0A, 0x02, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 14);
}

#[test]
fn test_execute_goto_taken() {
    // PUSH 1, GOTO 7, PUSH 99, EXIT, [pad?], PUSH 5, EXIT
    // pc=0: PUSH 1 -> stack [1], pc=2
    // pc=2: GOTO -> pc=3, spop=1, pc = P[3]=7
    // pc=7: PUSH 5 -> stack [5], pc=9
    // pc=9: EXIT -> 5
    let mut prog = make_prog(vec![0x01, 1, 0x09, 7, 0x01, 99, 0x00, 0x01, 5, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 5);
}

#[test]
fn test_execute_goto_not_taken() {
    // PUSH 0, GOTO 7, PUSH 99, EXIT, ..., PUSH 5, EXIT
    // pc=0: PUSH 0; pc=2: GOTO -> pc=3, spop=0, pc=4
    // pc=4: PUSH 99; pc=6: EXIT -> 99
    let mut prog = make_prog(vec![0x01, 0, 0x09, 7, 0x01, 99, 0x00, 0x01, 5, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 99);
}

#[test]
fn test_execute_count_program_returns_11() {
    // From C tests.c: parsing and executing Count.sloth should return 11.
    let prog = parser::parse("c_src/Examples/Count.sloth");
    let mut p = prog;
    assert_eq!(slothvm::execute(&mut p), 11);
}

#[test]
fn test_execute_none_returns_zero() {
    let mut prog: Option<SlothProgram> = None;
    assert_eq!(slothvm::execute(&mut prog), 0);
}

fn main() {}

use SlothLang::slothvm::{self, SlothProgram};

fn make_program(codes: Vec<u8>) -> Option<SlothProgram> {
    Some(SlothProgram { codes, pc: 0 })
}

// EXIT with empty stack returns 0
#[test]
fn test_exit_empty_stack() {
    let mut prog = make_program(vec![0x00]);
    assert_eq!(slothvm::execute(&mut prog), 0);
}

// EXIT with value on stack returns that value
#[test]
fn test_exit_with_value() {
    let mut prog = make_program(vec![0x01, 0x05, 0x00]); // PUSH 5, EXIT
    assert_eq!(slothvm::execute(&mut prog), 5);
}

// PUSH puts value on stack
#[test]
fn test_push() {
    let mut prog = make_program(vec![0x01, 0x0A, 0x00]); // PUSH 10, EXIT
    assert_eq!(slothvm::execute(&mut prog), 10);
}

// ADD: a + b
#[test]
fn test_add() {
    let mut prog = make_program(vec![0x01, 0x03, 0x01, 0x04, 0x02, 0x00]);
    // PUSH 3, PUSH 4, ADD, EXIT → 3+4=7
    assert_eq!(slothvm::execute(&mut prog), 7);
}

// SUB: a - b (first pushed - second pushed)
#[test]
fn test_sub() {
    let mut prog = make_program(vec![0x01, 0x0A, 0x01, 0x03, 0x03, 0x00]);
    // PUSH 10, PUSH 3, SUB, EXIT → 10-3=7
    assert_eq!(slothvm::execute(&mut prog), 7);
}

// MULT: a * b
#[test]
fn test_mult() {
    let mut prog = make_program(vec![0x01, 0x03, 0x01, 0x04, 0x04, 0x00]);
    // PUSH 3, PUSH 4, MULT, EXIT → 3*4=12
    assert_eq!(slothvm::execute(&mut prog), 12);
}

// DIV: a / b
#[test]
fn test_div() {
    let mut prog = make_program(vec![0x01, 0x0C, 0x01, 0x03, 0x05, 0x00]);
    // PUSH 12, PUSH 3, DIV, EXIT → 12/3=4
    assert_eq!(slothvm::execute(&mut prog), 4);
}

// COMP EQ: equal
#[test]
fn test_comp_eq_true() {
    let mut prog = make_program(vec![0x01, 0x05, 0x01, 0x05, 0x06, 0x01, 0x00]);
    // PUSH 5, PUSH 5, COMP EQ, EXIT → 1
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_eq_false() {
    let mut prog = make_program(vec![0x01, 0x05, 0x01, 0x06, 0x06, 0x01, 0x00]);
    // PUSH 5, PUSH 6, COMP EQ, EXIT → 0
    assert_eq!(slothvm::execute(&mut prog), 0);
}

// COMP NEQ
#[test]
fn test_comp_neq_true() {
    let mut prog = make_program(vec![0x01, 0x05, 0x01, 0x06, 0x06, 0x02, 0x00]);
    // PUSH 5, PUSH 6, COMP NEQ, EXIT → 1
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_neq_false() {
    let mut prog = make_program(vec![0x01, 0x05, 0x01, 0x05, 0x06, 0x02, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 0);
}

// COMP LT
#[test]
fn test_comp_lt_true() {
    let mut prog = make_program(vec![0x01, 0x03, 0x01, 0x05, 0x06, 0x03, 0x00]);
    // PUSH 3, PUSH 5, COMP LT, EXIT → 1 (3 < 5)
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_lt_false() {
    let mut prog = make_program(vec![0x01, 0x05, 0x01, 0x03, 0x06, 0x03, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 0);
}

// COMP LE
#[test]
fn test_comp_le_equal() {
    let mut prog = make_program(vec![0x01, 0x05, 0x01, 0x05, 0x06, 0x04, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_le_less() {
    let mut prog = make_program(vec![0x01, 0x03, 0x01, 0x05, 0x06, 0x04, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_le_greater() {
    let mut prog = make_program(vec![0x01, 0x05, 0x01, 0x03, 0x06, 0x04, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 0);
}

// COMP GT
#[test]
fn test_comp_gt_true() {
    let mut prog = make_program(vec![0x01, 0x05, 0x01, 0x03, 0x06, 0x05, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_gt_false() {
    let mut prog = make_program(vec![0x01, 0x03, 0x01, 0x05, 0x06, 0x05, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 0);
}

// COMP GE
#[test]
fn test_comp_ge_equal() {
    let mut prog = make_program(vec![0x01, 0x05, 0x01, 0x05, 0x06, 0x06, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_ge_greater() {
    let mut prog = make_program(vec![0x01, 0x05, 0x01, 0x03, 0x06, 0x06, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_ge_less() {
    let mut prog = make_program(vec![0x01, 0x03, 0x01, 0x05, 0x06, 0x06, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 0);
}

// GOTO: conditional jump
#[test]
fn test_goto_taken() {
    // PUSH 1, GOTO 6, PUSH 99, EXIT, PUSH 42, EXIT
    // If condition is 1, jump to address 6 (PUSH 42)
    let mut prog = make_program(vec![
        0x01, 0x01, // PUSH 1
        0x09, 0x06, // GOTO 6
        0x01, 0x63, // PUSH 99 (skipped)
        0x01, 0x2A, // PUSH 42 (target)
        0x00,       // EXIT
    ]);
    assert_eq!(slothvm::execute(&mut prog), 42);
}

#[test]
fn test_goto_not_taken() {
    // PUSH 0, GOTO 6, PUSH 99, EXIT, PUSH 42, EXIT
    let mut prog = make_program(vec![
        0x01, 0x00, // PUSH 0
        0x09, 0x06, // GOTO 6 (not taken, 0 != 1)
        0x01, 0x63, // PUSH 99
        0x00,       // EXIT
        0x01, 0x2A, // PUSH 42 (not reached)
        0x00,       // EXIT
    ]);
    assert_eq!(slothvm::execute(&mut prog), 99);
}

// DUP: duplicate top of stack
#[test]
fn test_dup() {
    let mut prog = make_program(vec![0x01, 0x07, 0x0A, 0x02, 0x00]);
    // PUSH 7, DUP, ADD, EXIT → 7+7=14
    assert_eq!(slothvm::execute(&mut prog), 14);
}

// OUT INT: prints integer, pops value
#[test]
fn test_out_int() {
    let mut prog = make_program(vec![0x01, 0x05, 0x08, 0x01, 0x00]);
    // PUSH 5, OUT INT, EXIT → returns 0 (stack empty after OUT pops)
    assert_eq!(slothvm::execute(&mut prog), 0);
}

// OUT CHR: prints char, pops value
#[test]
fn test_out_chr() {
    let mut prog = make_program(vec![0x01, 0x41, 0x08, 0x02, 0x00]);
    // PUSH 65, OUT CHR, EXIT → returns 0 (prints 'A')
    assert_eq!(slothvm::execute(&mut prog), 0);
}

// Chained arithmetic
#[test]
fn test_chained_arithmetic() {
    // (3 + 4) * 2 = 14
    let mut prog = make_program(vec![
        0x01, 0x03, // PUSH 3
        0x01, 0x04, // PUSH 4
        0x02,       // ADD → 7
        0x01, 0x02, // PUSH 2
        0x04,       // MULT → 14
        0x00,       // EXIT
    ]);
    assert_eq!(slothvm::execute(&mut prog), 14);
}

// Count.sloth bytecodes directly - execute returns 11
#[test]
fn test_count_program() {
    let mut prog = make_program(vec![
        0x01, 0x01, 0x0a, 0x08, 0x01, 0x01, 0x0a, 0x08, 0x02,
        0x01, 0x01, 0x02, 0x0a, 0x01, 0x0b, 0x03, 0x01, 0x01,
        0x01, 0x01, 0x03, 0x06, 0x03, 0x09, 0x02, 0x00,
    ]);
    assert_eq!(slothvm::execute(&mut prog), 11);
}

// ADD with zero
#[test]
fn test_add_zero() {
    let mut prog = make_program(vec![0x01, 0x05, 0x01, 0x00, 0x02, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 5);
}

// SUB resulting in zero
#[test]
fn test_sub_to_zero() {
    let mut prog = make_program(vec![0x01, 0x05, 0x01, 0x05, 0x03, 0x00]);
    assert_eq!(slothvm::execute(&mut prog), 0);
}

// Multiple pushes, exit returns top
#[test]
fn test_exit_returns_top() {
    let mut prog = make_program(vec![0x01, 0x01, 0x01, 0x02, 0x01, 0x03, 0x00]);
    // PUSH 1, PUSH 2, PUSH 3, EXIT → returns 3 (top of stack)
    assert_eq!(slothvm::execute(&mut prog), 3);
}

// DIV truncates toward zero (integer division)
#[test]
fn test_div_truncation() {
    let mut prog = make_program(vec![0x01, 0x07, 0x01, 0x02, 0x05, 0x00]);
    // PUSH 7, PUSH 2, DIV, EXIT → 7/2=3
    assert_eq!(slothvm::execute(&mut prog), 3);
}

fn main() {}

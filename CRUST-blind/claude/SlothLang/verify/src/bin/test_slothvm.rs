use SlothLang::slothvm::{self, SlothProgram, Opcodes, CompCodes, TypeCode};

fn run(codes: Vec<u8>) -> i32 {
    let mut prog = Some(SlothProgram { codes, pc: 0 });
    slothvm::execute(&mut prog)
}

// -------- Opcodes / enum constants ---------

#[test]
fn test_opcodes_values() {
    assert_eq!(Opcodes::Exit as u8, 0x00);
    assert_eq!(Opcodes::Push as u8, 0x01);
    assert_eq!(Opcodes::Add as u8, 0x02);
    assert_eq!(Opcodes::Sub as u8, 0x03);
    assert_eq!(Opcodes::Mult as u8, 0x04);
    assert_eq!(Opcodes::Div as u8, 0x05);
    assert_eq!(Opcodes::Comp as u8, 0x06);
    assert_eq!(Opcodes::Inp as u8, 0x07);
    assert_eq!(Opcodes::Out as u8, 0x08);
    assert_eq!(Opcodes::Goto as u8, 0x09);
    assert_eq!(Opcodes::Dup as u8, 0x0A);
}

#[test]
fn test_compcodes_values() {
    assert_eq!(CompCodes::Eq as u8, 0x01);
    assert_eq!(CompCodes::Neq as u8, 0x02);
    assert_eq!(CompCodes::Lt as u8, 0x03);
    assert_eq!(CompCodes::Le as u8, 0x04);
    assert_eq!(CompCodes::Gt as u8, 0x05);
    assert_eq!(CompCodes::Ge as u8, 0x06);
}

#[test]
fn test_typecode_values() {
    assert_eq!(TypeCode::Int as u8, 0x01);
    assert_eq!(TypeCode::Chr as u8, 0x02);
}

// -------- execute() ---------

#[test]
fn test_execute_none_returns_zero() {
    let mut prog: Option<SlothProgram> = None;
    let r = slothvm::execute(&mut prog);
    assert_eq!(r, 0);
}

#[test]
fn test_execute_immediate_exit() {
    // EXIT with empty stack -> 0
    let r = run(vec![0x00]);
    assert_eq!(r, 0);
}

#[test]
fn test_execute_push_and_exit() {
    // PUSH 42 ; EXIT -> 42
    let r = run(vec![0x01, 42, 0x00]);
    assert_eq!(r, 42);
}

#[test]
fn test_execute_push_max_byte() {
    // PUSH 255 ; EXIT -> 255 (push reads byte as int 0..255)
    let r = run(vec![0x01, 255, 0x00]);
    assert_eq!(r, 255);
}

#[test]
fn test_execute_add() {
    // PUSH 5 ; PUSH 3 ; ADD ; EXIT -> 8
    let r = run(vec![0x01, 5, 0x01, 3, 0x02, 0x00]);
    assert_eq!(r, 8);
}

#[test]
fn test_execute_sub() {
    // PUSH 10 ; PUSH 4 ; SUB ; EXIT -> 6
    let r = run(vec![0x01, 10, 0x01, 4, 0x03, 0x00]);
    assert_eq!(r, 6);
}

#[test]
fn test_execute_sub_negative_result() {
    // PUSH 5 ; PUSH 100 ; SUB ; EXIT -> -95
    let r = run(vec![0x01, 5, 0x01, 100, 0x03, 0x00]);
    assert_eq!(r, -95);
}

#[test]
fn test_execute_mult() {
    // PUSH 5 ; PUSH 6 ; MULT ; EXIT -> 30
    let r = run(vec![0x01, 5, 0x01, 6, 0x04, 0x00]);
    assert_eq!(r, 30);
}

#[test]
fn test_execute_mult_large() {
    // PUSH 200 ; PUSH 200 ; MULT ; EXIT -> 40000
    let r = run(vec![0x01, 200, 0x01, 200, 0x04, 0x00]);
    assert_eq!(r, 40000);
}

#[test]
fn test_execute_div() {
    // PUSH 20 ; PUSH 4 ; DIV ; EXIT -> 5
    let r = run(vec![0x01, 20, 0x01, 4, 0x05, 0x00]);
    assert_eq!(r, 5);
}

#[test]
fn test_execute_div_truncates_toward_zero() {
    // PUSH 100 ; PUSH 3 ; DIV ; EXIT -> 33 (C truncates)
    let r = run(vec![0x01, 100, 0x01, 3, 0x05, 0x00]);
    assert_eq!(r, 33);
}

#[test]
fn test_execute_div_negative() {
    // PUSH 5 ; PUSH 100 ; SUB ; PUSH 10 ; DIV ; EXIT
    // (5 - 100) / 10 = -95 / 10 = -9 (truncating toward zero in C)
    let r = run(vec![0x01, 5, 0x01, 100, 0x03, 0x01, 10, 0x05, 0x00]);
    assert_eq!(r, -9);
}

#[test]
fn test_execute_dup() {
    // PUSH 7 ; DUP ; ADD ; EXIT -> 14
    let r = run(vec![0x01, 7, 0x0a, 0x02, 0x00]);
    assert_eq!(r, 14);
}

#[test]
fn test_execute_comp_eq_true() {
    // PUSH 5 ; PUSH 5 ; COMP EQ ; EXIT -> 1
    let r = run(vec![0x01, 5, 0x01, 5, 0x06, 0x01, 0x00]);
    assert_eq!(r, 1);
}

#[test]
fn test_execute_comp_eq_false() {
    // PUSH 4 ; PUSH 5 ; COMP EQ ; EXIT -> 0
    let r = run(vec![0x01, 4, 0x01, 5, 0x06, 0x01, 0x00]);
    assert_eq!(r, 0);
}

#[test]
fn test_execute_comp_neq() {
    // 5 != 5 -> 0
    let r = run(vec![0x01, 5, 0x01, 5, 0x06, 0x02, 0x00]);
    assert_eq!(r, 0);
    // 4 != 5 -> 1
    let r = run(vec![0x01, 4, 0x01, 5, 0x06, 0x02, 0x00]);
    assert_eq!(r, 1);
}

#[test]
fn test_execute_comp_lt() {
    // 4 < 5 -> 1
    let r = run(vec![0x01, 4, 0x01, 5, 0x06, 0x03, 0x00]);
    assert_eq!(r, 1);
    // 5 < 5 -> 0
    let r = run(vec![0x01, 5, 0x01, 5, 0x06, 0x03, 0x00]);
    assert_eq!(r, 0);
    // 6 < 5 -> 0
    let r = run(vec![0x01, 6, 0x01, 5, 0x06, 0x03, 0x00]);
    assert_eq!(r, 0);
}

#[test]
fn test_execute_comp_le() {
    // 5 <= 5 -> 1
    let r = run(vec![0x01, 5, 0x01, 5, 0x06, 0x04, 0x00]);
    assert_eq!(r, 1);
    // 6 <= 5 -> 0
    let r = run(vec![0x01, 6, 0x01, 5, 0x06, 0x04, 0x00]);
    assert_eq!(r, 0);
}

#[test]
fn test_execute_comp_gt() {
    // 6 > 5 -> 1
    let r = run(vec![0x01, 6, 0x01, 5, 0x06, 0x05, 0x00]);
    assert_eq!(r, 1);
    // 5 > 5 -> 0
    let r = run(vec![0x01, 5, 0x01, 5, 0x06, 0x05, 0x00]);
    assert_eq!(r, 0);
}

#[test]
fn test_execute_comp_ge() {
    // 5 >= 5 -> 1
    let r = run(vec![0x01, 5, 0x01, 5, 0x06, 0x06, 0x00]);
    assert_eq!(r, 1);
    // 4 >= 5 -> 0
    let r = run(vec![0x01, 4, 0x01, 5, 0x06, 0x06, 0x00]);
    assert_eq!(r, 0);
}

#[test]
fn test_execute_goto_taken() {
    // PUSH 1 PUSH 1 GOTO 8 PUSH 50 EXIT PUSH 99 EXIT
    // After GOTO with top=1, jumps to pc=8 (EXIT). Stack=[1] -> returns 1.
    let r = run(vec![0x01, 1, 0x01, 1, 0x09, 8, 0x01, 50, 0x00, 0x01, 99, 0x00]);
    assert_eq!(r, 1);
}

#[test]
fn test_execute_goto_not_taken() {
    // PUSH 1 PUSH 0 GOTO 99 PUSH 5 EXIT
    // Top is 0 (not 1), so GOTO advances pc and continues normally.
    let r = run(vec![0x01, 1, 0x01, 0, 0x09, 99, 0x01, 5, 0x00]);
    // After PUSH 1 stack=[1], PUSH 0 stack=[1,0], GOTO pops 0, continues.
    // Then PUSH 5 stack=[1,5], EXIT pops 5.
    assert_eq!(r, 5);
}

#[test]
fn test_execute_chained_arithmetic() {
    // PUSH 100 ; PUSH 50 ; PUSH 25 ; ADD ; ADD ; EXIT
    // (100 + (50 + 25)) = 175
    let r = run(vec![0x01, 100, 0x01, 50, 0x01, 25, 0x02, 0x02, 0x00]);
    assert_eq!(r, 175);
}

#[test]
fn test_slothprogram_struct_fields() {
    // Verify the struct has expected fields and pc starts at 0.
    let p = SlothProgram { codes: vec![0x00], pc: 0 };
    assert_eq!(p.codes, vec![0x00]);
    assert_eq!(p.pc, 0);
}

#[test]
fn test_execute_runs_until_exit_in_middle_of_program() {
    // PUSH 7 EXIT then garbage; should stop at EXIT and return 7.
    let r = run(vec![0x01, 7, 0x00, 0xff, 0xff, 0xff, 0xff]);
    assert_eq!(r, 7);
}

#[test]
fn test_ubyte_alias() {
    // The UByte alias should be u8.
    let x: slothvm::UByte = 255;
    assert_eq!(x, 255u8);
}

fn main() {}

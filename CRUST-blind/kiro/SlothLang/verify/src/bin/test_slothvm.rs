use SlothLang::slothvm::{self, SlothProgram};

#[test]
fn test_exit_empty_stack() {
    let mut prog = Some(SlothProgram { codes: vec![0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 0);
}

#[test]
fn test_push_exit() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x05, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 5);
}

#[test]
fn test_add() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x03, 0x01, 0x07, 0x02, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 10);
}

#[test]
fn test_sub() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x0A, 0x01, 0x03, 0x03, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 7);
}

#[test]
fn test_mult() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x04, 0x01, 0x05, 0x04, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 20);
}

#[test]
fn test_div() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x14, 0x01, 0x04, 0x05, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 5);
}

#[test]
fn test_comp_eq_true() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x05, 0x01, 0x05, 0x06, 0x01, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_eq_false() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x05, 0x01, 0x03, 0x06, 0x01, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 0);
}

#[test]
fn test_comp_neq() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x05, 0x01, 0x03, 0x06, 0x02, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_lt_true() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x03, 0x01, 0x05, 0x06, 0x03, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_lt_false() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x05, 0x01, 0x03, 0x06, 0x03, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 0);
}

#[test]
fn test_comp_le_true() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x03, 0x01, 0x05, 0x06, 0x04, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_le_equal() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x05, 0x01, 0x05, 0x06, 0x04, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_gt() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x05, 0x01, 0x03, 0x06, 0x05, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_comp_ge_false() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x03, 0x01, 0x05, 0x06, 0x06, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 0);
}

#[test]
fn test_comp_ge_equal() {
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x05, 0x01, 0x05, 0x06, 0x06, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 1);
}

#[test]
fn test_dup() {
    // PUSH 7, DUP, ADD, EXIT -> 14
    let mut prog = Some(SlothProgram { codes: vec![0x01, 0x07, 0x0A, 0x02, 0x00], pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 14);
}

#[test]
fn test_goto_true() {
    // PUSH 1, GOTO 8, PUSH 99, EXIT, (padding), PUSH 42, EXIT
    let mut prog = Some(SlothProgram {
        codes: vec![0x01, 0x01, 0x09, 0x08, 0x01, 0x63, 0x00, 0x00, 0x01, 0x2A, 0x00],
        pc: 0,
    });
    assert_eq!(slothvm::execute(&mut prog), 42);
}

#[test]
fn test_goto_false() {
    // PUSH 0, GOTO 8, PUSH 99, EXIT
    let mut prog = Some(SlothProgram {
        codes: vec![0x01, 0x00, 0x09, 0x08, 0x01, 0x63, 0x00, 0x00],
        pc: 0,
    });
    assert_eq!(slothvm::execute(&mut prog), 99);
}

#[test]
fn test_count_program() {
    // The Count.sloth bytecodes from ground truth
    let codes: Vec<u8> = vec![
        0x01, 0x01, 0x0a, 0x08, 0x01, 0x01, 0x0a, 0x08, 0x02,
        0x01, 0x01, 0x02, 0x0a, 0x01, 0x0b, 0x03, 0x01, 0x01,
        0x01, 0x01, 0x03, 0x06, 0x03, 0x09, 0x02, 0x00,
    ];
    let mut prog = Some(SlothProgram { codes, pc: 0 });
    assert_eq!(slothvm::execute(&mut prog), 11);
}

fn main() {}

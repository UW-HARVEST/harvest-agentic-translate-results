use emlang::data::{Data, DataType};
use emlang::em::{Em, EmType, Program, DATA_STDERR, DATA_STDOUT, DEFAULT_PROGRAM_CAP};

#[test]
fn test_constants() {
    assert_eq!(DEFAULT_PROGRAM_CAP, 256);
    assert_eq!(DATA_STDOUT, 1);
    assert_eq!(DATA_STDERR, 2);
}

#[test]
fn test_em_new() {
    let em = Em::new(EmType::Pop);
    assert_eq!(em.em_type, EmType::Pop);
    assert_eq!(em.row, 0);
    assert_eq!(em.col, 0);
    assert_eq!(em.path, "");
    assert_eq!(em.r#ref, 0);
    assert_eq!(em.ran, false);
}

#[test]
fn test_em_new_with_data() {
    let d = Data::new_int(42);
    let em = Em::new_with_data(EmType::Push, d);
    assert_eq!(em.em_type, EmType::Push);
    assert_eq!(em.data.dtype, DataType::Int);
}

#[test]
fn test_em_type_display() {
    assert_eq!(format!("{}", EmType::Push), "push");
    assert_eq!(format!("{}", EmType::Pop), "pop");
    assert_eq!(format!("{}", EmType::Add), "add");
    assert_eq!(format!("{}", EmType::Sub), "sub");
    assert_eq!(format!("{}", EmType::Mul), "mul");
    assert_eq!(format!("{}", EmType::Div), "div");
    assert_eq!(format!("{}", EmType::Grt), "grt");
    assert_eq!(format!("{}", EmType::Less), "less");
    assert_eq!(format!("{}", EmType::Equ), "equ");
    assert_eq!(format!("{}", EmType::Nequ), "nequ");
    assert_eq!(format!("{}", EmType::PrintBegin), "print_begin");
    assert_eq!(format!("{}", EmType::PrintEnd), "print_end");
    assert_eq!(format!("{}", EmType::IfBegin), "if_begin");
    assert_eq!(format!("{}", EmType::IfEnd), "if_end");
    assert_eq!(format!("{}", EmType::LoopBegin), "loop_begin");
    assert_eq!(format!("{}", EmType::LoopEnd), "loop_end");
    assert_eq!(format!("{}", EmType::Exit), "exit");
    assert_eq!(format!("{}", EmType::Dup), "dup");
    assert_eq!(format!("{}", EmType::Swap), "swap");
}

#[test]
fn test_program_new() {
    let p = Program::new(8);
    assert_eq!(p.cap, 8);
    assert_eq!(p.size, 0);
}

#[test]
fn test_program_push() {
    let mut p = Program::new(2);
    p.push(Em::new(EmType::Pop));
    assert_eq!(p.size, 1);
    p.push(Em::new(EmType::Add));
    assert_eq!(p.size, 2);
    // Push beyond cap; Vec capacity grows
    p.push(Em::new(EmType::Sub));
    assert_eq!(p.size, 3);
    assert_eq!(p.ems[0].em_type, EmType::Pop);
    assert_eq!(p.ems[1].em_type, EmType::Add);
    assert_eq!(p.ems[2].em_type, EmType::Sub);
}

fn main() {}

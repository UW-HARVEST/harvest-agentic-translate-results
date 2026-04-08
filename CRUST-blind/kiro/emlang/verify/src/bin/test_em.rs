use emlang::data::{Data, DataValue};
use emlang::em::{Em, EmType, Program, DEFAULT_PROGRAM_CAP, DATA_STDOUT, DATA_STDERR};

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
fn test_em_new() {
    let e = Em::new(EmType::Pop);
    assert_eq!(e.em_type, EmType::Pop);
    assert_eq!(e.row, 0);
    assert_eq!(e.col, 0);
    assert_eq!(e.r#ref, 0);
    assert_eq!(e.ran, false);
    assert_eq!(e.path, "");
}

#[test]
fn test_em_new_with_data() {
    let e = Em::new_with_data(EmType::Push, Data::new_int(42));
    assert_eq!(e.em_type, EmType::Push);
    match e.data.value { DataValue::Int(v) => assert_eq!(v, 42), _ => panic!() }
    assert_eq!(e.row, 0);
    assert_eq!(e.col, 0);
    assert_eq!(e.r#ref, 0);
    assert_eq!(e.ran, false);
}

#[test]
fn test_program_new() {
    let p = Program::new(4);
    assert_eq!(p.cap, 4);
    assert_eq!(p.size, 0);
}

#[test]
fn test_program_push() {
    let mut p = Program::new(4);
    p.push(Em::new(EmType::Pop));
    p.push(Em::new(EmType::Add));
    assert_eq!(p.size, 2);
    assert_eq!(p.ems[0].em_type, EmType::Pop);
    assert_eq!(p.ems[1].em_type, EmType::Add);
}

#[test]
fn test_constants() {
    assert_eq!(DEFAULT_PROGRAM_CAP, 256);
    assert_eq!(DATA_STDOUT, 1);
    assert_eq!(DATA_STDERR, 2);
}

#[test]
fn test_em_display_push_int() {
    let mut e = Em::new_with_data(EmType::Push, Data::new_int(42));
    e.path = "test.eml".to_string();
    e.row = 1;
    e.col = 1;
    let s = format!("{}", e);
    assert!(s.contains("<push"));
    assert!(s.contains("42"));
    assert!(s.contains("test.eml:1:1>"));
}

#[test]
fn test_em_display_print_end_stdout() {
    let mut e = Em::new_with_data(EmType::PrintEnd, Data::new_int(DATA_STDOUT as i64));
    e.path = "t.eml".to_string();
    let s = format!("{}", e);
    assert!(s.contains("stdout"));
}

#[test]
fn test_em_display_print_end_stderr() {
    let mut e = Em::new_with_data(EmType::PrintEnd, Data::new_int(DATA_STDERR as i64));
    e.path = "t.eml".to_string();
    let s = format!("{}", e);
    assert!(s.contains("stderr"));
}

fn main() {}

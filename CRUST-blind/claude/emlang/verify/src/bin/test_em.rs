use emlang::data::{Data, DataType, DataValue};
use emlang::em::{Em, EmType, Program, DEFAULT_PROGRAM_CAP, DATA_STDOUT, DATA_STDERR};

#[test]
fn test_em_type_display() {
    // C: em_type_to_cstr names
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
fn test_em_constants() {
    assert_eq!(DEFAULT_PROGRAM_CAP, 256);
    assert_eq!(DATA_STDOUT, 1);
    assert_eq!(DATA_STDERR, 2);
}

#[test]
fn test_em_new() {
    // C: em_new(type) -> {type=type, ...zero}
    let em = Em::new(EmType::Pop);
    assert_eq!(em.em_type, EmType::Pop);
    // default data is zero-int
    assert_eq!(em.data.dtype, DataType::Int);
    match &em.data.value {
        DataValue::Int(i) => assert_eq!(*i, 0),
        _ => panic!("expected Int"),
    }
    assert_eq!(em.path, "");
    assert_eq!(em.row, 0);
    assert_eq!(em.col, 0);
    assert_eq!(em.r#ref, 0);
    assert_eq!(em.ran, false);
}

#[test]
fn test_em_new_with_data() {
    // C: em_new_with_data(EM_PUSH, data_new_int(7))
    let em = Em::new_with_data(EmType::Push, Data::new_int(7));
    assert_eq!(em.em_type, EmType::Push);
    assert_eq!(em.data.dtype, DataType::Int);
    match &em.data.value {
        DataValue::Int(i) => assert_eq!(*i, 7),
        _ => panic!("expected Int"),
    }
    assert_eq!(em.row, 0);
    assert_eq!(em.col, 0);
    assert_eq!(em.r#ref, 0);
    assert_eq!(em.ran, false);
}

#[test]
fn test_em_new_with_str() {
    let em = Em::new_with_data(EmType::Push, Data::new_str("foo".to_string()));
    assert_eq!(em.em_type, EmType::Push);
    assert_eq!(em.data.dtype, DataType::Str);
    match &em.data.value {
        DataValue::Str(s) => assert_eq!(s, "foo"),
        _ => panic!("expected Str"),
    }
}

#[test]
fn test_em_display_push_int() {
    // C: em_fprintf(EM_PUSH, data=42) -> "<push 42 :0:0>\n"
    let em = Em::new_with_data(EmType::Push, Data::new_int(42));
    let s = format!("{}", em);
    assert_eq!(s, "<push 42 :0:0>\n");
}

#[test]
fn test_em_display_push_str() {
    let mut em = Em::new_with_data(EmType::Push, Data::new_str("hi".to_string()));
    em.path = "foo.eml".to_string();
    em.row = 3;
    em.col = 5;
    let s = format!("{}", em);
    assert_eq!(s, "<push hi foo.eml:3:5>\n");
}

#[test]
fn test_em_display_pop() {
    let mut em = Em::new(EmType::Pop);
    em.row = 1;
    em.col = 2;
    let s = format!("{}", em);
    assert_eq!(s, "<pop :1:2>\n");
}

#[test]
fn test_em_display_print_end_stdout() {
    let mut em = Em::new_with_data(EmType::PrintEnd, Data::new_int(DATA_STDOUT as i64));
    em.row = 1;
    em.col = 1;
    let s = format!("{}", em);
    assert_eq!(s, "<print_end stdout :1:1>\n");
}

#[test]
fn test_em_display_print_end_stderr() {
    let mut em = Em::new_with_data(EmType::PrintEnd, Data::new_int(DATA_STDERR as i64));
    em.row = 2;
    em.col = 3;
    let s = format!("{}", em);
    assert_eq!(s, "<print_end stderr :2:3>\n");
}

#[test]
fn test_em_display_print_begin_with_ref() {
    let mut em = Em::new(EmType::PrintBegin);
    em.r#ref = 5;
    em.row = 1;
    em.col = 1;
    let s = format!("{}", em);
    assert_eq!(s, "<print_begin ref: 5 :1:1>\n");
}

#[test]
fn test_em_display_if_begin_with_ref() {
    let mut em = Em::new(EmType::IfBegin);
    em.r#ref = 7;
    em.row = 4;
    em.col = 2;
    let s = format!("{}", em);
    assert_eq!(s, "<if_begin ref: 7 :4:2>\n");
}

#[test]
fn test_em_display_add() {
    let mut em = Em::new(EmType::Add);
    em.row = 1;
    em.col = 5;
    let s = format!("{}", em);
    assert_eq!(s, "<add :1:5>\n");
}

#[test]
fn test_program_new() {
    // C: program_new(256) -> {cap=256, size=0, ems!=NULL}
    let p = Program::new(256);
    assert_eq!(p.cap, 256);
    assert_eq!(p.size, 0);
    assert_eq!(p.ems.len(), 0);
}

#[test]
fn test_program_push() {
    // C: program_push grows
    let mut p = Program::new(2);
    assert_eq!(p.size, 0);
    p.push(Em::new(EmType::Push));
    assert_eq!(p.size, 1);
    p.push(Em::new(EmType::Pop));
    assert_eq!(p.size, 2);
    // Triggers reallocation
    p.push(Em::new(EmType::Add));
    assert_eq!(p.size, 3);
    assert_eq!(p.ems.len(), 3);
    assert_eq!(p.ems[0].em_type, EmType::Push);
    assert_eq!(p.ems[1].em_type, EmType::Pop);
    assert_eq!(p.ems[2].em_type, EmType::Add);
}

#[test]
fn test_em_type_eq() {
    assert_eq!(EmType::Push, EmType::Push);
    assert_ne!(EmType::Push, EmType::Pop);
}

fn main() {}

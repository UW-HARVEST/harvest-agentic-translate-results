use emlang::data::{Data, DataType, DataValue};

#[test]
fn test_data_type_display() {
    assert_eq!(format!("{}", DataType::Int), "int");
    assert_eq!(format!("{}", DataType::Str), "str");
}

#[test]
fn test_data_new_int_default() {
    // C: data_new(DATA_INT) -> {type=0, int_=0}
    let d = Data::new(DataType::Int);
    assert_eq!(d.dtype, DataType::Int);
    match &d.value {
        DataValue::Int(i) => assert_eq!(*i, 0),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_data_new_str_default() {
    let d = Data::new(DataType::Str);
    assert_eq!(d.dtype, DataType::Str);
    match &d.value {
        DataValue::Str(s) => assert_eq!(s, ""),
        _ => panic!("expected Str"),
    }
}

#[test]
fn test_data_new_int() {
    // C: data_new_int(42) -> {type=0, int_=42}
    let d = Data::new_int(42);
    assert_eq!(d.dtype, DataType::Int);
    match &d.value {
        DataValue::Int(i) => assert_eq!(*i, 42),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_data_new_int_negative() {
    let d = Data::new_int(-1234);
    assert_eq!(d.dtype, DataType::Int);
    match &d.value {
        DataValue::Int(i) => assert_eq!(*i, -1234),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_data_new_int_max() {
    let d = Data::new_int(i64::MAX);
    assert_eq!(d.dtype, DataType::Int);
    match &d.value {
        DataValue::Int(i) => assert_eq!(*i, i64::MAX),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_data_new_str() {
    // C: data_new_str("hello") -> {type=1, str="hello"}
    let d = Data::new_str("hello".to_string());
    assert_eq!(d.dtype, DataType::Str);
    match &d.value {
        DataValue::Str(s) => assert_eq!(s, "hello"),
        _ => panic!("expected Str"),
    }
}

#[test]
fn test_data_display_int() {
    // C: data_fprintf(int=42) -> "42"
    let d = Data::new_int(42);
    assert_eq!(format!("{}", d), "42");
}

#[test]
fn test_data_display_int_negative() {
    let d = Data::new_int(-3);
    assert_eq!(format!("{}", d), "-3");
}

#[test]
fn test_data_display_str() {
    // C: data_fprintf(str="hello") -> "hello"
    let d = Data::new_str("hello".to_string());
    assert_eq!(format!("{}", d), "hello");
}

#[test]
fn test_data_display_str_empty() {
    let d = Data::new_str("".to_string());
    assert_eq!(format!("{}", d), "");
}

#[test]
fn test_data_display_int_truncation_to_i32() {
    // The C code uses (int)data->as.int_ -- which truncates. Match that.
    // value larger than i32::MAX -> truncated.
    // E.g., 0x100000001 -> 1
    let d = Data::new_int(0x1_0000_0001);
    assert_eq!(format!("{}", d), "1");
}

#[test]
fn test_data_type_eq() {
    assert_eq!(DataType::Int, DataType::Int);
    assert_eq!(DataType::Str, DataType::Str);
    assert_ne!(DataType::Int, DataType::Str);
}

fn main() {}

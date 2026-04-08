use recordManager::tables::*;
use recordManager::rm_serializer;

#[test]
fn test_string_to_value_int() {
    let v = string_to_value("i10");
    assert!(matches!(v.dt, DataType::DtInt));
    assert!(matches!(v.v, ValueUnion::IntV(10)));
}

#[test]
fn test_string_to_value_int_zero() {
    let v = string_to_value("i0");
    assert!(matches!(v.dt, DataType::DtInt));
    assert!(matches!(v.v, ValueUnion::IntV(0)));
}

#[test]
fn test_string_to_value_int_negative() {
    let v = string_to_value("i-1");
    assert!(matches!(v.dt, DataType::DtInt));
    assert!(matches!(v.v, ValueUnion::IntV(-1)));
}

#[test]
fn test_string_to_value_float() {
    let v = string_to_value("f3.14");
    assert!(matches!(v.dt, DataType::DtFloat));
    if let ValueUnion::FloatV(f) = v.v {
        assert!((f - 3.14f32).abs() < 0.001);
    } else {
        panic!("expected FloatV");
    }
}

#[test]
fn test_string_to_value_string() {
    let v = string_to_value("stest");
    assert!(matches!(v.dt, DataType::DtString));
    if let ValueUnion::StringV(s) = &v.v {
        assert_eq!(s, "test");
    } else {
        panic!("expected StringV");
    }
}

#[test]
fn test_string_to_value_bool_true() {
    let v = string_to_value("bt");
    assert!(matches!(v.dt, DataType::DtBool));
    assert!(matches!(v.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_string_to_value_bool_true_long() {
    let v = string_to_value("btrue");
    assert!(matches!(v.dt, DataType::DtBool));
    assert!(matches!(v.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_string_to_value_bool_false() {
    let v = string_to_value("bf");
    assert!(matches!(v.dt, DataType::DtBool));
    assert!(matches!(v.v, ValueUnion::BoolV(false)));
}

#[test]
fn test_make_string_value_int() {
    let v = Value { dt: DataType::DtInt, v: ValueUnion::IntV(42) };
    assert_eq!(make_string_value(&v), "42");
}

#[test]
fn test_make_string_value_bool() {
    let v_true = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let v_false = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    assert_eq!(make_string_value(&v_true), "true");
    assert_eq!(make_string_value(&v_false), "false");
}

#[test]
fn test_make_string_value_string() {
    let v = Value { dt: DataType::DtString, v: ValueUnion::StringV("hello".to_string()) };
    assert_eq!(make_string_value(&v), "hello");
}

#[test]
fn test_make_value_int() {
    let v = make_value("INT", "42");
    assert!(matches!(v.dt, DataType::DtInt));
    assert!(matches!(v.v, ValueUnion::IntV(42)));
}

#[test]
fn test_make_value_float() {
    let v = make_value("FLOAT", "3.14");
    assert!(matches!(v.dt, DataType::DtFloat));
    if let ValueUnion::FloatV(f) = v.v {
        assert!((f - 3.14f32).abs() < 0.001);
    } else {
        panic!("expected FloatV");
    }
}

#[test]
fn test_make_value_string() {
    let v = make_value("STRING", "hello");
    assert!(matches!(v.dt, DataType::DtString));
    if let ValueUnion::StringV(s) = &v.v {
        assert_eq!(s, "hello");
    } else {
        panic!("expected StringV");
    }
}

#[test]
fn test_make_value_bool() {
    let v = make_value("BOOL", "true");
    assert!(matches!(v.dt, DataType::DtBool));
    assert!(matches!(v.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_serialize_schema() {
    let schema = Schema {
        num_attr: 3,
        attr_names: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        data_types: vec![DataType::DtInt, DataType::DtString, DataType::DtInt],
        type_length: vec![0, 4, 0],
        key_attrs: vec![0],
        key_size: 1,
    };
    let s = serialize_schema(&schema);
    assert_eq!(s, "Schema with <3> attributes (a: INT, b: STRING[4], c: INT) with keys: (a)\n");
}

#[test]
fn test_serialize_value_int() {
    let v = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    assert_eq!(serialize_value(&v), "10");
}

#[test]
fn test_serialize_value_string() {
    let v = Value { dt: DataType::DtString, v: ValueUnion::StringV("Hello World".to_string()) };
    assert_eq!(serialize_value(&v), "Hello World");
}

#[test]
fn test_serialize_value_bool() {
    let v_true = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let v_false = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    assert_eq!(serialize_value(&v_true), "true");
    assert_eq!(serialize_value(&v_false), "false");
}

#[test]
fn test_serialize_value_float() {
    // C ground truth: serializeValue(stringToValue("f5.3")) = "5.300000"
    let v = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(5.3) };
    assert_eq!(serialize_value(&v), "5.300000");
}

#[test]
fn test_serialize_value_float_zero() {
    let v = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(0.0) };
    assert_eq!(serialize_value(&v), "0.000000");
}

#[test]
fn test_make_string_value_float() {
    let v = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(1.5) };
    assert_eq!(make_string_value(&v), "1.500000");
}

#[test]
fn test_datatype_display() {
    assert_eq!(format!("{}", DataType::DtInt), "0");
    assert_eq!(format!("{}", DataType::DtString), "1");
    assert_eq!(format!("{}", DataType::DtFloat), "2");
    assert_eq!(format!("{}", DataType::DtBool), "3");
}

fn main() {}

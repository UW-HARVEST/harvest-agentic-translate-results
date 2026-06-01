use recordManager::rm_serializer::{
    serialize_value, string_to_value, serialize_schema, attr_offset, serialize_record,
    serialize_attr,
};
use recordManager::tables::{DataType, Value, ValueUnion, Schema, Record, RID};
use recordManager::dberror::RC;

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
fn test_serialize_value_bool_true() {
    let v = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    assert_eq!(serialize_value(&v), "true");
}

#[test]
fn test_serialize_value_bool_false() {
    let v = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    assert_eq!(serialize_value(&v), "false");
}

#[test]
fn test_serialize_value_float() {
    let v = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(5.3) };
    assert_eq!(serialize_value(&v), "5.300000");
}

#[test]
fn test_string_to_value_int() {
    let v = string_to_value("i10");
    assert!(matches!(v.dt, DataType::DtInt));
    assert!(matches!(v.v, ValueUnion::IntV(10)));
}

#[test]
fn test_string_to_value_string() {
    let v = string_to_value("sHello World");
    assert!(matches!(v.dt, DataType::DtString));
    if let ValueUnion::StringV(s) = v.v {
        assert_eq!(s, "Hello World");
    } else {
        panic!("not string");
    }
}

#[test]
fn test_string_to_value_bt() {
    let v = string_to_value("bt");
    assert!(matches!(v.dt, DataType::DtBool));
    assert!(matches!(v.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_string_to_value_bf() {
    let v = string_to_value("bf");
    assert!(matches!(v.dt, DataType::DtBool));
    assert!(matches!(v.v, ValueUnion::BoolV(false)));
}

#[test]
fn test_string_to_value_btrue() {
    // C parses byte at index 1: 't' -> true
    let v = string_to_value("btrue");
    assert!(matches!(v.dt, DataType::DtBool));
    assert!(matches!(v.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_string_to_value_float() {
    let v = string_to_value("f5.3");
    assert!(matches!(v.dt, DataType::DtFloat));
    if let ValueUnion::FloatV(f) = v.v {
        assert!((f - 5.3_f32).abs() < 1e-5);
    } else {
        panic!("not float");
    }
}

#[test]
fn test_attr_offset() {
    // Schema: a INT, b STRING(4), c INT
    let schema = Schema {
        num_attr: 3,
        attr_names: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        data_types: vec![DataType::DtInt, DataType::DtString, DataType::DtInt],
        type_length: vec![0, 4, 0],
        key_attrs: vec![0],
        key_size: 1,
    };
    let mut off = -1i32;
    let rc = attr_offset(&schema, 0, &mut off);
    assert!(rc == RC::Ok);
    assert_eq!(off, 0);
    let mut off = -1i32;
    let rc = attr_offset(&schema, 1, &mut off);
    assert!(rc == RC::Ok);
    assert_eq!(off, 4);
    let mut off = -1i32;
    let rc = attr_offset(&schema, 2, &mut off);
    assert!(rc == RC::Ok);
    assert_eq!(off, 8);
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
    // Expected from C: "Schema with <3> attributes (a: INT, b: STRING[4], c: INT) with keys: (a)\n"
    let s = serialize_schema(&schema);
    assert_eq!(s, "Schema with <3> attributes (a: INT, b: STRING[4], c: INT) with keys: (a)\n");
}

#[test]
fn test_serialize_record_and_attr() {
    // Build a record matching {1, "aaaa", 3}: int(4)+string(4)+int(4) = 12 bytes
    let mut bytes = Vec::with_capacity(12);
    bytes.extend_from_slice(&1i32.to_ne_bytes());
    bytes.extend_from_slice(b"aaaa");
    bytes.extend_from_slice(&3i32.to_ne_bytes());
    let data: String = bytes.iter().map(|&b| b as char).collect();
    let record = Record {
        id: RID { page: 0, slot: 0 },
        data,
    };
    let schema = Schema {
        num_attr: 3,
        attr_names: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        data_types: vec![DataType::DtInt, DataType::DtString, DataType::DtInt],
        type_length: vec![0, 4, 0],
        key_attrs: vec![0],
        key_size: 1,
    };
    assert_eq!(serialize_attr(&record, &schema, 0), "a:1");
    assert_eq!(serialize_attr(&record, &schema, 1), "b:aaaa");
    assert_eq!(serialize_attr(&record, &schema, 2), "c:3");
    // serialize_record format: "[page-slot] (a:1b:aaaa,c:3,)" — note C appends "," after each non-first attr
    let s = serialize_record(&record, &schema);
    assert_eq!(s, "[0-0] (a:1b:aaaa,c:3,)");
}

fn main() {}

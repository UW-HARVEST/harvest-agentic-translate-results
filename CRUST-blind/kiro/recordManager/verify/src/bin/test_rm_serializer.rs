use recordManager::rm_serializer;
use recordManager::tables::{self, DataType, Value, ValueUnion, Schema, Record, RID};
use recordManager::dberror::RC;

fn test_schema() -> Schema {
    Schema {
        num_attr: 3,
        attr_names: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        data_types: vec![DataType::DtInt, DataType::DtString, DataType::DtInt],
        type_length: vec![0, 4, 0],
        key_attrs: vec![0],
        key_size: 1,
    }
}

fn make_test_record(a: i32, b: &str, c: i32) -> Record {
    let rec_size = 12usize;
    let mut data_bytes = vec![0u8; rec_size + 1];
    data_bytes[0..4].copy_from_slice(&a.to_le_bytes());
    let b_bytes = b.as_bytes();
    for i in 0..4 {
        data_bytes[4 + i] = if i < b_bytes.len() { b_bytes[i] } else { 0 };
    }
    data_bytes[8..12].copy_from_slice(&c.to_le_bytes());
    let data: String = data_bytes.iter().map(|&b| b as char).collect();
    Record { id: RID { page: 0, slot: 0 }, data }
}

#[test]
fn test_attr_offset() {
    let schema = test_schema();
    let mut result = 0i32;
    assert_eq!(rm_serializer::attr_offset(&schema, 0, &mut result), RC::Ok);
    assert_eq!(result, 0);
    assert_eq!(rm_serializer::attr_offset(&schema, 1, &mut result), RC::Ok);
    assert_eq!(result, 4);
    assert_eq!(rm_serializer::attr_offset(&schema, 2, &mut result), RC::Ok);
    assert_eq!(result, 8);
}

#[test]
fn test_serialize_schema() {
    let schema = test_schema();
    let s = rm_serializer::serialize_schema(&schema);
    assert_eq!(s, "Schema with <3> attributes (a: INT, b: STRING[4], c: INT) with keys: (a)\n");
}

#[test]
fn test_serialize_record_basic() {
    let schema = test_schema();
    let r = make_test_record(1, "aaaa", 3);
    let s = rm_serializer::serialize_record(&r, &schema);
    assert_eq!(s, "[0-0] (a:1b:aaaa,c:3,)");
}

#[test]
fn test_serialize_record_other_values() {
    let schema = test_schema();
    let mut r = make_test_record(42, "test", 99);
    r.id.page = 5;
    r.id.slot = 3;
    let s = rm_serializer::serialize_record(&r, &schema);
    assert_eq!(s, "[5-3] (a:42b:test,c:99,)");
}

#[test]
fn test_serialize_attr_int() {
    let schema = test_schema();
    let r = make_test_record(1, "aaaa", 3);
    assert_eq!(rm_serializer::serialize_attr(&r, &schema, 0), "a:1");
    assert_eq!(rm_serializer::serialize_attr(&r, &schema, 2), "c:3");
}

#[test]
fn test_serialize_attr_string() {
    let schema = test_schema();
    let r = make_test_record(1, "aaaa", 3);
    assert_eq!(rm_serializer::serialize_attr(&r, &schema, 1), "b:aaaa");
}

#[test]
fn test_serialize_value_int() {
    let v = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    assert_eq!(rm_serializer::serialize_value(&v), "10");
}

#[test]
fn test_serialize_value_int_negative() {
    let v = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    assert_eq!(rm_serializer::serialize_value(&v), "-1");
}

#[test]
fn test_serialize_value_string() {
    let v = Value { dt: DataType::DtString, v: ValueUnion::StringV("Hello World".to_string()) };
    assert_eq!(rm_serializer::serialize_value(&v), "Hello World");
}

#[test]
fn test_serialize_value_bool() {
    let v_true = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let v_false = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    assert_eq!(rm_serializer::serialize_value(&v_true), "true");
    assert_eq!(rm_serializer::serialize_value(&v_false), "false");
}

#[test]
fn test_serialize_value_float() {
    // C ground truth: serializeValue(stringToValue("f5.3")) = "5.300000"
    let v = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(5.3) };
    assert_eq!(rm_serializer::serialize_value(&v), "5.300000");
}

#[test]
fn test_serialize_value_float_zero() {
    // C ground truth: "0.000000"
    let v = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(0.0) };
    assert_eq!(rm_serializer::serialize_value(&v), "0.000000");
}

#[test]
fn test_string_to_value_via_serializer() {
    let v = rm_serializer::string_to_value("i10");
    assert!(matches!(v.dt, DataType::DtInt));
    if let ValueUnion::IntV(i) = v.v { assert_eq!(i, 10); } else { panic!("expected IntV"); }
}

fn main() {}

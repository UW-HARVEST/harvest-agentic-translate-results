use recordManager::rm_serializer;
use recordManager::tables::*;
use recordManager::record_mgr;
use recordManager::dberror::RC;

// ---- attr_offset tests ----

#[test]
fn test_attr_offset_first() {
    let s = record_mgr::create_schema(
        2, vec!["a".into(), "b".into()],
        vec![DataType::DtInt, DataType::DtString], vec![0, 10], 1, vec![0],
    );
    let mut result = 0;
    rm_serializer::attr_offset(&s, 0, &mut result);
    assert_eq!(result, 0);
}

#[test]
fn test_attr_offset_after_int() {
    let s = record_mgr::create_schema(
        2, vec!["a".into(), "b".into()],
        vec![DataType::DtInt, DataType::DtString], vec![0, 10], 1, vec![0],
    );
    let mut result = 0;
    rm_serializer::attr_offset(&s, 1, &mut result);
    assert_eq!(result, 4); // sizeof(int) = 4
}

#[test]
fn test_attr_offset_after_string() {
    let s = record_mgr::create_schema(
        2, vec!["a".into(), "b".into()],
        vec![DataType::DtString, DataType::DtInt], vec![10, 0], 1, vec![0],
    );
    let mut result = 0;
    rm_serializer::attr_offset(&s, 1, &mut result);
    assert_eq!(result, 10);
}

#[test]
fn test_attr_offset_after_float() {
    // rm_serializer::attr_offset correctly uses sizeof(float)=4
    let s = record_mgr::create_schema(
        2, vec!["a".into(), "b".into()],
        vec![DataType::DtFloat, DataType::DtInt], vec![0, 0], 1, vec![0],
    );
    let mut result = 0;
    rm_serializer::attr_offset(&s, 1, &mut result);
    assert_eq!(result, 4);
}

#[test]
fn test_attr_offset_after_bool() {
    // sizeof(bool) = sizeof(short) = 2
    let s = record_mgr::create_schema(
        2, vec!["a".into(), "b".into()],
        vec![DataType::DtBool, DataType::DtInt], vec![0, 0], 1, vec![0],
    );
    let mut result = 0;
    rm_serializer::attr_offset(&s, 1, &mut result);
    assert_eq!(result, 2);
}

#[test]
fn test_attr_offset_val() {
    let s = record_mgr::create_schema(
        3, vec!["a".into(), "b".into(), "c".into()],
        vec![DataType::DtInt, DataType::DtString, DataType::DtBool],
        vec![0, 5, 0], 1, vec![0],
    );
    assert_eq!(rm_serializer::attr_offset_val(&s, 0), 0);
    assert_eq!(rm_serializer::attr_offset_val(&s, 1), 4);
    assert_eq!(rm_serializer::attr_offset_val(&s, 2), 9);
}

// ---- serialize_schema (delegates to tables) ----

#[test]
fn test_rm_serialize_schema() {
    let s = record_mgr::create_schema(
        2, vec!["id".into(), "name".into()],
        vec![DataType::DtInt, DataType::DtString], vec![0, 20], 1, vec![0],
    );
    let result = rm_serializer::serialize_schema(&s);
    assert_eq!(result, "Schema with <2> attributes (id: INT, name: STRING[20]) with keys: (id)\n");
}

// ---- serialize_value (delegates to tables) ----

#[test]
fn test_rm_serialize_value_int() {
    let v = Value { dt: DataType::DtInt, v: ValueUnion::IntV(42) };
    assert_eq!(rm_serializer::serialize_value(&v), "42");
}

#[test]
fn test_rm_serialize_value_string() {
    let v = Value { dt: DataType::DtString, v: ValueUnion::StringV("hello".into()) };
    assert_eq!(rm_serializer::serialize_value(&v), "hello");
}

#[test]
fn test_rm_serialize_value_bool() {
    let v = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    assert_eq!(rm_serializer::serialize_value(&v), "true");
    let v2 = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    assert_eq!(rm_serializer::serialize_value(&v2), "false");
}

// ---- string_to_value (delegates to tables) ----

#[test]
fn test_rm_string_to_value() {
    let v = rm_serializer::string_to_value("i99");
    assert!(matches!(v.dt, DataType::DtInt));
    assert!(matches!(v.v, ValueUnion::IntV(99)));
}

#[test]
fn test_rm_string_to_value_bool() {
    let v = rm_serializer::string_to_value("bt");
    assert!(matches!(v.dt, DataType::DtBool));
    assert!(matches!(v.v, ValueUnion::BoolV(true)));
}

fn main() {}

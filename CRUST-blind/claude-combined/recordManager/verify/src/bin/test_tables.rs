use recordManager::tables::{
    DataType, Value, ValueUnion, Record, RID, Schema,
    string_to_value, serialize_value, serialize_schema, serialize_record, serialize_attr,
    make_value, make_string_value,
};

#[test]
fn test_data_type_display() {
    assert_eq!(format!("{}", DataType::DtInt), "0");
    assert_eq!(format!("{}", DataType::DtString), "1");
    assert_eq!(format!("{}", DataType::DtFloat), "2");
    assert_eq!(format!("{}", DataType::DtBool), "3");
}

#[test]
fn test_string_to_value_via_tables_module() {
    let v = string_to_value("i42");
    assert!(matches!(v.dt, DataType::DtInt));
    assert!(matches!(v.v, ValueUnion::IntV(42)));
}

#[test]
fn test_serialize_value_via_tables_module() {
    let v = Value { dt: DataType::DtInt, v: ValueUnion::IntV(7) };
    assert_eq!(serialize_value(&v), "7");
}

#[test]
fn test_make_value_int() {
    let v = make_value("DT_INT", "10");
    assert!(matches!(v.dt, DataType::DtInt));
    assert!(matches!(v.v, ValueUnion::IntV(10)));
}

#[test]
fn test_make_string_value_returns_string() {
    let v = Value { dt: DataType::DtString, v: ValueUnion::StringV("hi".to_string()) };
    assert_eq!(make_string_value(&v), "hi");
}

#[test]
fn test_serialize_schema_via_tables_module() {
    let schema = Schema {
        num_attr: 2,
        attr_names: vec!["x".to_string(), "y".to_string()],
        data_types: vec![DataType::DtInt, DataType::DtFloat],
        type_length: vec![0, 0],
        key_attrs: vec![0],
        key_size: 1,
    };
    let s = serialize_schema(&schema);
    assert_eq!(s, "Schema with <2> attributes (x: INT, y: FLOAT) with keys: (x)\n");
}

#[test]
fn test_serialize_record_int_attr_via_tables_module() {
    let mut bytes = Vec::with_capacity(4);
    bytes.extend_from_slice(&5i32.to_ne_bytes());
    let data: String = bytes.iter().map(|&b| b as char).collect();
    let record = Record {
        id: RID { page: 1, slot: 2 },
        data,
    };
    let schema = Schema {
        num_attr: 1,
        attr_names: vec!["a".to_string()],
        data_types: vec![DataType::DtInt],
        type_length: vec![0],
        key_attrs: vec![0],
        key_size: 1,
    };
    assert_eq!(serialize_attr(&record, &schema, 0), "a:5");
    assert_eq!(serialize_record(&record, &schema), "[1-2] (a:5)");
}

fn main() {}

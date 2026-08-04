use recordManager::tables::{
    make_string_value, make_value, serialize_attr, serialize_record, serialize_schema,
    serialize_table_content, serialize_table_info, serialize_value, string_to_value, DataType,
    Record, Schema, Value, ValueUnion, RID,
};

fn make_test_schema() -> Schema {
    Schema {
        num_attr: 3,
        attr_names: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        data_types: vec![DataType::DtInt, DataType::DtString, DataType::DtInt],
        type_length: vec![0, 4, 0],
        key_attrs: vec![0],
        key_size: 1,
    }
}

#[test]
fn test_datatype_codes() {
    assert_eq!(DataType::DtInt as i32, 0);
    assert_eq!(DataType::DtString as i32, 1);
    assert_eq!(DataType::DtFloat as i32, 2);
    assert_eq!(DataType::DtBool as i32, 3);
}

#[test]
fn test_make_value_int() {
    let v = make_value("DT_INT", "42");
    assert!(matches!(v.dt, DataType::DtInt));
    if let ValueUnion::IntV(i) = v.v {
        assert_eq!(i, 42);
    } else {
        panic!("not int");
    }
    let v2 = make_value("i", "7");
    assert!(matches!(v2.dt, DataType::DtInt));
    if let ValueUnion::IntV(i) = v2.v {
        assert_eq!(i, 7);
    } else {
        panic!("not int");
    }
}

#[test]
fn test_make_value_float() {
    let v = make_value("DT_FLOAT", "3.14");
    assert!(matches!(v.dt, DataType::DtFloat));
    if let ValueUnion::FloatV(f) = v.v {
        assert!((f - 3.14).abs() < 1e-5);
    } else {
        panic!("not float");
    }
}

#[test]
fn test_make_value_string() {
    let v = make_value("DT_STRING", "hello");
    assert!(matches!(v.dt, DataType::DtString));
    if let ValueUnion::StringV(s) = v.v {
        assert_eq!(s, "hello");
    } else {
        panic!("not string");
    }
}

#[test]
fn test_make_value_bool() {
    let v = make_value("DT_BOOL", "true");
    assert!(matches!(v.dt, DataType::DtBool));
    if let ValueUnion::BoolV(b) = v.v {
        assert!(b);
    }

    let v2 = make_value("b", "false");
    if let ValueUnion::BoolV(b) = v2.v {
        assert!(!b);
    }
}

#[test]
fn test_make_string_value() {
    let v = Value {
        dt: DataType::DtString,
        v: ValueUnion::StringV("hello".to_string()),
    };
    let s = make_string_value(&v);
    assert_eq!(s, "hello");
}

#[test]
fn test_string_to_value_int() {
    let v = string_to_value("i42");
    assert!(matches!(v.dt, DataType::DtInt));
    if let ValueUnion::IntV(i) = v.v {
        assert_eq!(i, 42);
    } else {
        panic!("not int");
    }
}

#[test]
fn test_string_to_value_float() {
    let v = string_to_value("f5.3");
    assert!(matches!(v.dt, DataType::DtFloat));
    if let ValueUnion::FloatV(f) = v.v {
        assert!((f - 5.3).abs() < 1e-3);
    } else {
        panic!("not float");
    }
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
fn test_string_to_value_bool() {
    let v = string_to_value("bt");
    if let ValueUnion::BoolV(b) = v.v {
        assert!(b);
    } else {
        panic!("not bool");
    }
    let v2 = string_to_value("btrue");
    if let ValueUnion::BoolV(b) = v2.v {
        assert!(b);
    }
    let v3 = string_to_value("bf");
    if let ValueUnion::BoolV(b) = v3.v {
        assert!(!b);
    }
}

#[test]
fn test_serialize_value() {
    // C output expectations from running C code
    let v = Value {
        dt: DataType::DtInt,
        v: ValueUnion::IntV(42),
    };
    assert_eq!(serialize_value(&v), "42");

    let v = Value {
        dt: DataType::DtFloat,
        v: ValueUnion::FloatV(3.14),
    };
    assert_eq!(serialize_value(&v), "3.140000");

    let v = Value {
        dt: DataType::DtString,
        v: ValueUnion::StringV("Hello".to_string()),
    };
    assert_eq!(serialize_value(&v), "Hello");

    let v = Value {
        dt: DataType::DtBool,
        v: ValueUnion::BoolV(true),
    };
    assert_eq!(serialize_value(&v), "true");

    let v = Value {
        dt: DataType::DtBool,
        v: ValueUnion::BoolV(false),
    };
    assert_eq!(serialize_value(&v), "false");
}

#[test]
fn test_serialize_schema() {
    let s = make_test_schema();
    // C output: "Schema with <3> attributes (a: INT, b: STRING[4], c: INT) with keys: (a)\n"
    let out = serialize_schema(&s);
    assert_eq!(
        out,
        "Schema with <3> attributes (a: INT, b: STRING[4], c: INT) with keys: (a)\n"
    );
}

#[test]
fn test_serialize_record_and_attr() {
    // build a record with attr a=1 (int), b="aaaa" (string len 4), c=3 (int)
    let schema = make_test_schema();
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&1i32.to_ne_bytes());
    data.extend_from_slice(b"aaaa");
    data.extend_from_slice(&3i32.to_ne_bytes());
    let r = Record {
        id: RID { page: 0, slot: 0 },
        data: unsafe { String::from_utf8_unchecked(data) },
    };
    // C output for serializeRecord: "[0-0] (a:1b:aaaa,c:3,)"
    let out = serialize_record(&r, &schema);
    assert_eq!(out, "[0-0] (a:1b:aaaa,c:3,)");

    // Test serialize_attr individually
    assert_eq!(serialize_attr(&r, &schema, 0), "a:1");
    assert_eq!(serialize_attr(&r, &schema, 1), "b:aaaa");
    assert_eq!(serialize_attr(&r, &schema, 2), "c:3");
}

#[test]
fn test_serialize_table_info() {
    use recordManager::record_mgr::{
        close_table, create_table, delete_table, init_record_manager, open_table,
        shutdown_record_manager,
    };
    use recordManager::tables::RM_TableData;
    let schema = make_test_schema();
    let _ = init_record_manager(None);
    let _ = create_table("test_serialize_info", &schema);
    let mut rel = RM_TableData {
        name: String::new(),
        schema: Schema {
            num_attr: 0,
            attr_names: vec![],
            data_types: vec![],
            type_length: vec![],
            key_attrs: vec![],
            key_size: 0,
        },
        mgmt_data: None,
    };
    let _ = open_table(&mut rel, "test_serialize_info");
    let info = serialize_table_info(&rel);
    // C output for empty table:
    // "TABLE <test_serialize_info> with <0> tuples:\nSchema with <3> attributes (a: INT, b: STRING[4], c: INT) with keys: (a)\n"
    assert!(info.starts_with("TABLE <test_serialize_info> with <0> tuples:\n"));
    assert!(info.contains("Schema with <3> attributes (a: INT, b: STRING[4], c: INT) with keys: (a)"));
    let _ = close_table(&mut rel);
    let _ = delete_table("test_serialize_info");
    let _ = shutdown_record_manager();
}

#[test]
fn test_serialize_table_content_empty() {
    use recordManager::record_mgr::{
        close_table, create_table, delete_table, init_record_manager, open_table,
        shutdown_record_manager,
    };
    use recordManager::tables::RM_TableData;
    let schema = make_test_schema();
    let _ = init_record_manager(None);
    let _ = create_table("test_serialize_content", &schema);
    let mut rel = RM_TableData {
        name: String::new(),
        schema: Schema {
            num_attr: 0,
            attr_names: vec![],
            data_types: vec![],
            type_length: vec![],
            key_attrs: vec![],
            key_size: 0,
        },
        mgmt_data: None,
    };
    let _ = open_table(&mut rel, "test_serialize_content");
    let content = serialize_table_content(&rel);
    // empty table -> just the header attributes
    assert_eq!(content, "a, b, c");
    let _ = close_table(&mut rel);
    let _ = delete_table("test_serialize_content");
    let _ = shutdown_record_manager();
}

fn main() {}

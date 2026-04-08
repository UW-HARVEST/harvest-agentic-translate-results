use recordManager::tables::*;
use recordManager::record_mgr;
use recordManager::rm_serializer;

// ---- getRecordSize tests ----

fn make_schema(types: Vec<DataType>, lengths: Vec<i32>) -> Schema {
    let n = types.len() as i32;
    let names: Vec<String> = (0..n).map(|i| format!("a{}", i)).collect();
    record_mgr::create_schema(n, names, types, lengths, 1, vec![0])
}

#[test]
fn test_get_record_size_int() {
    // C: sizeof(int)=4, no padding needed => 4
    let s = make_schema(vec![DataType::DtInt], vec![0]);
    assert_eq!(record_mgr::get_record_size(&s), 4);
}

#[test]
fn test_get_record_size_bool() {
    // C: sizeof(bool)=2, pad to 4 => 4
    let s = make_schema(vec![DataType::DtBool], vec![0]);
    assert_eq!(record_mgr::get_record_size(&s), 4);
}

#[test]
fn test_get_record_size_string5() {
    // C: 5 chars, pad to 8
    let s = make_schema(vec![DataType::DtString], vec![5]);
    assert_eq!(record_mgr::get_record_size(&s), 8);
}

#[test]
fn test_get_record_size_string10() {
    // C: 10 chars, pad to 12
    let s = make_schema(vec![DataType::DtString], vec![10]);
    assert_eq!(record_mgr::get_record_size(&s), 12);
}

#[test]
fn test_get_record_size_two_ints() {
    // C: 4+4=8, no padding => 8
    let s = make_schema(vec![DataType::DtInt, DataType::DtInt], vec![0, 0]);
    assert_eq!(record_mgr::get_record_size(&s), 8);
}

#[test]
fn test_get_record_size_mixed() {
    // C: INT(4)+STRING[4](4)+FLOAT(4)+BOOL(2)=14, pad to 16
    let s = make_schema(
        vec![DataType::DtInt, DataType::DtString, DataType::DtFloat, DataType::DtBool],
        vec![0, 4, 0, 0],
    );
    assert_eq!(record_mgr::get_record_size(&s), 16);
}

#[test]
fn test_get_record_size_float() {
    // C: sizeof(float)=4, no padding => 4
    let s = make_schema(vec![DataType::DtFloat], vec![0]);
    assert_eq!(record_mgr::get_record_size(&s), 4);
}

// ---- get_attr_pos tests ----

#[test]
fn test_get_attr_pos_first() {
    let s = make_schema(vec![DataType::DtInt, DataType::DtString], vec![0, 4]);
    assert_eq!(record_mgr::get_attr_pos(&s, 0), 0);
}

#[test]
fn test_get_attr_pos_after_int() {
    let s = make_schema(vec![DataType::DtInt, DataType::DtInt], vec![0, 0]);
    assert_eq!(record_mgr::get_attr_pos(&s, 1), 4);
}

#[test]
fn test_get_attr_pos_after_string() {
    let s = make_schema(vec![DataType::DtString, DataType::DtInt], vec![10, 0]);
    assert_eq!(record_mgr::get_attr_pos(&s, 1), 10);
}

#[test]
fn test_get_attr_pos_after_bool() {
    let s = make_schema(vec![DataType::DtBool, DataType::DtInt], vec![0, 0]);
    assert_eq!(record_mgr::get_attr_pos(&s, 1), 2);
}

// ---- createSchema tests ----

#[test]
fn test_create_schema() {
    let s = record_mgr::create_schema(
        3,
        vec!["a".into(), "b".into(), "c".into()],
        vec![DataType::DtInt, DataType::DtString, DataType::DtBool],
        vec![0, 10, 0],
        1,
        vec![0],
    );
    assert_eq!(s.num_attr, 3);
    assert_eq!(s.attr_names.len(), 3);
    assert_eq!(s.attr_names[0], "a");
    assert_eq!(s.attr_names[1], "b");
    assert_eq!(s.attr_names[2], "c");
    assert_eq!(s.key_size, 1);
    assert_eq!(s.key_attrs[0], 0);
    assert_eq!(s.type_length[1], 10);
}

// ---- createRecord / freeRecord tests ----

#[test]
fn test_create_record() {
    let s = make_schema(vec![DataType::DtInt], vec![0]);
    let mut rec: Option<Record> = None;
    let rc = record_mgr::create_record(&mut rec, &s);
    assert_eq!(rc, recordManager::dberror::RC::Ok);
    assert!(rec.is_some());
    let r = rec.unwrap();
    // Record data should be at least getRecordSize bytes
    assert!(r.data.len() >= record_mgr::get_record_size(&s) as usize);
}

// ---- set_attr / get_attr tests ----

#[test]
fn test_set_get_attr_int() {
    let s = make_schema(vec![DataType::DtInt, DataType::DtInt], vec![0, 0]);
    let mut rec: Option<Record> = None;
    record_mgr::create_record(&mut rec, &s);
    let mut r = rec.unwrap();

    let val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(42) };
    record_mgr::set_attr(&mut r, &s, 0, &val);
    let val2 = Value { dt: DataType::DtInt, v: ValueUnion::IntV(99) };
    record_mgr::set_attr(&mut r, &s, 1, &val2);

    let mut out = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    record_mgr::get_attr(&r, &s, 0, &mut out);
    match out.v {
        ValueUnion::IntV(v) => assert_eq!(v, 42),
        _ => panic!("expected IntV"),
    }

    record_mgr::get_attr(&r, &s, 1, &mut out);
    match out.v {
        ValueUnion::IntV(v) => assert_eq!(v, 99),
        _ => panic!("expected IntV"),
    }
}

#[test]
fn test_set_get_attr_string() {
    let s = make_schema(vec![DataType::DtString], vec![10]);
    let mut rec: Option<Record> = None;
    record_mgr::create_record(&mut rec, &s);
    let mut r = rec.unwrap();

    let val = Value { dt: DataType::DtString, v: ValueUnion::StringV("hello".into()) };
    record_mgr::set_attr(&mut r, &s, 0, &val);

    let mut out = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    record_mgr::get_attr(&r, &s, 0, &mut out);
    match &out.v {
        ValueUnion::StringV(s) => assert_eq!(s, "hello"),
        _ => panic!("expected StringV"),
    }
}

#[test]
fn test_set_get_attr_bool() {
    let s = make_schema(vec![DataType::DtBool], vec![0]);
    let mut rec: Option<Record> = None;
    record_mgr::create_record(&mut rec, &s);
    let mut r = rec.unwrap();

    let val = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    record_mgr::set_attr(&mut r, &s, 0, &val);

    let mut out = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    record_mgr::get_attr(&r, &s, 0, &mut out);
    match out.v {
        ValueUnion::BoolV(b) => assert!(b),
        _ => panic!("expected BoolV"),
    }
}

#[test]
fn test_set_get_attr_float() {
    let s = make_schema(vec![DataType::DtFloat], vec![0]);
    let mut rec: Option<Record> = None;
    record_mgr::create_record(&mut rec, &s);
    let mut r = rec.unwrap();

    let val = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(3.14) };
    record_mgr::set_attr(&mut r, &s, 0, &val);

    let mut out = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    record_mgr::get_attr(&r, &s, 0, &mut out);
    match out.v {
        ValueUnion::FloatV(f) => assert!((f - 3.14).abs() < 0.001),
        _ => panic!("expected FloatV"),
    }
}

// ---- stringToValue tests ----

#[test]
fn test_string_to_value_int() {
    let v = string_to_value("i42");
    assert!(matches!(v.dt, DataType::DtInt));
    assert!(matches!(v.v, ValueUnion::IntV(42)));
}

#[test]
fn test_string_to_value_float() {
    let v = string_to_value("f3.14");
    assert!(matches!(v.dt, DataType::DtFloat));
    if let ValueUnion::FloatV(f) = v.v {
        assert!((f - 3.14).abs() < 0.01);
    } else {
        panic!("expected FloatV");
    }
}

#[test]
fn test_string_to_value_string() {
    let v = string_to_value("shello");
    assert!(matches!(v.dt, DataType::DtString));
    if let ValueUnion::StringV(s) = &v.v {
        assert_eq!(s, "hello");
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
    // C: val[1] == 't' => TRUE, so "btrue" is also true
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
fn test_string_to_value_unknown_prefix() {
    // C: default case => DT_INT, intV=-1
    let v = string_to_value("x123");
    assert!(matches!(v.dt, DataType::DtInt));
    assert!(matches!(v.v, ValueUnion::IntV(-1)));
}

#[test]
fn test_string_to_value_empty() {
    // C: empty string => default => DT_INT, intV=-1
    let v = string_to_value("");
    assert!(matches!(v.dt, DataType::DtInt));
    assert!(matches!(v.v, ValueUnion::IntV(-1)));
}

// ---- serializeValue tests ----

#[test]
fn test_serialize_value_int() {
    let v = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    assert_eq!(serialize_value(&v), "10");
}

#[test]
fn test_serialize_value_string() {
    let v = Value { dt: DataType::DtString, v: ValueUnion::StringV("Hello World".into()) };
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
fn test_serialize_value_int_negative() {
    let v = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    assert_eq!(serialize_value(&v), "-1");
}

#[test]
fn test_serialize_value_int_zero() {
    let v = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    assert_eq!(serialize_value(&v), "0");
}

// ---- serializeSchema tests ----

#[test]
fn test_serialize_schema_single_int() {
    let s = record_mgr::create_schema(
        1, vec!["a".into()], vec![DataType::DtInt], vec![0], 1, vec![0],
    );
    let result = serialize_schema(&s);
    assert_eq!(result, "Schema with <1> attributes (a: INT) with keys: (a)\n");
}

#[test]
fn test_serialize_schema_mixed() {
    let s = record_mgr::create_schema(
        4,
        vec!["a".into(), "b".into(), "c".into(), "d".into()],
        vec![DataType::DtInt, DataType::DtString, DataType::DtFloat, DataType::DtBool],
        vec![0, 4, 0, 0],
        1,
        vec![0],
    );
    let result = serialize_schema(&s);
    assert_eq!(
        result,
        "Schema with <4> attributes (a: INT, b: STRING[4], c: FLOAT, d: BOOL) with keys: (a)\n"
    );
}

#[test]
fn test_serialize_schema_multiple_keys() {
    let s = record_mgr::create_schema(
        2,
        vec!["x".into(), "y".into()],
        vec![DataType::DtInt, DataType::DtInt],
        vec![0, 0],
        2,
        vec![0, 1],
    );
    let result = serialize_schema(&s);
    assert_eq!(
        result,
        "Schema with <2> attributes (x: INT, y: INT) with keys: (x, y)\n"
    );
}

// ---- roundtrip: stringToValue -> serializeValue ----

#[test]
fn test_roundtrip_int() {
    let v = string_to_value("i10");
    assert_eq!(serialize_value(&v), "10");
}

#[test]
fn test_roundtrip_string() {
    let v = string_to_value("sHello World");
    assert_eq!(serialize_value(&v), "Hello World");
}

#[test]
fn test_roundtrip_bool_true() {
    let v = string_to_value("bt");
    assert_eq!(serialize_value(&v), "true");
}

// ---- freeSchema (no-op in Rust, just check it returns Ok) ----

#[test]
fn test_free_schema() {
    let mut s = make_schema(vec![DataType::DtInt], vec![0]);
    assert_eq!(record_mgr::free_schema(&mut s), recordManager::dberror::RC::Ok);
}

// ---- initRecordManager / shutdownRecordManager ----

#[test]
fn test_init_shutdown_record_manager() {
    assert_eq!(record_mgr::init_record_manager(None), recordManager::dberror::RC::Ok);
    assert_eq!(record_mgr::shutdown_record_manager(), recordManager::dberror::RC::Ok);
}

fn main() {}

use recordManager::record_mgr::*;
use recordManager::tables::*;
use recordManager::dberror::RC;

fn test_schema() -> Schema {
    create_schema(
        3,
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        vec![DataType::DtInt, DataType::DtString, DataType::DtInt],
        vec![0, 4, 0],
        1,
        vec![0],
    )
}

fn make_test_record(schema: &Schema, a: i32, b: &str, c: i32) -> Record {
    let mut rec = None;
    create_record(&mut rec, schema);
    let mut r = rec.unwrap();
    set_attr(&mut r, schema, 0, &Value { dt: DataType::DtInt, v: ValueUnion::IntV(a) });
    set_attr(&mut r, schema, 1, &Value { dt: DataType::DtString, v: ValueUnion::StringV(b.to_string()) });
    set_attr(&mut r, schema, 2, &Value { dt: DataType::DtInt, v: ValueUnion::IntV(c) });
    r
}

#[test]
fn test_get_record_size() {
    let schema = test_schema();
    // C ground truth: 12
    assert_eq!(get_record_size(&schema), 12);
}

#[test]
fn test_get_record_size_with_bool() {
    let schema = create_schema(
        2,
        vec!["x".to_string(), "y".to_string()],
        vec![DataType::DtInt, DataType::DtBool],
        vec![0, 0],
        1,
        vec![0],
    );
    // C ground truth: 8 (4 for int + 2 for bool + 2 padding)
    assert_eq!(get_record_size(&schema), 8);
}

#[test]
fn test_create_schema() {
    let schema = test_schema();
    assert_eq!(schema.num_attr, 3);
    assert_eq!(schema.attr_names, vec!["a", "b", "c"]);
    assert_eq!(schema.key_size, 1);
    assert_eq!(schema.key_attrs, vec![0]);
}

#[test]
fn test_create_record() {
    let schema = test_schema();
    let mut rec = None;
    let rc = create_record(&mut rec, &schema);
    assert_eq!(rc, RC::Ok);
    assert!(rec.is_some());
    let r = rec.unwrap();
    assert_eq!(r.data.len(), 13); // rec_size(12) + 1
}

#[test]
fn test_set_and_get_attr_int() {
    let schema = test_schema();
    let mut r = make_test_record(&schema, 1, "aaaa", 3);
    let mut val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    get_attr(&r, &schema, 0, &mut val);
    assert!(matches!(val.dt, DataType::DtInt));
    if let ValueUnion::IntV(v) = val.v { assert_eq!(v, 1); } else { panic!("expected IntV"); }
}

#[test]
fn test_set_and_get_attr_string() {
    let schema = test_schema();
    let r = make_test_record(&schema, 1, "aaaa", 3);
    let mut val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    get_attr(&r, &schema, 1, &mut val);
    assert!(matches!(val.dt, DataType::DtString));
    if let ValueUnion::StringV(s) = &val.v { assert_eq!(s, "aaaa"); } else { panic!("expected StringV"); }
}

#[test]
fn test_set_and_get_attr_int_third() {
    let schema = test_schema();
    let r = make_test_record(&schema, 1, "aaaa", 3);
    let mut val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    get_attr(&r, &schema, 2, &mut val);
    if let ValueUnion::IntV(v) = val.v { assert_eq!(v, 3); } else { panic!("expected IntV"); }
}

#[test]
fn test_modify_attr() {
    let schema = test_schema();
    let mut r = make_test_record(&schema, 1, "aaaa", 3);
    set_attr(&mut r, &schema, 2, &Value { dt: DataType::DtInt, v: ValueUnion::IntV(4) });
    let mut val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    get_attr(&r, &schema, 2, &mut val);
    if let ValueUnion::IntV(v) = val.v { assert_eq!(v, 4); } else { panic!("expected IntV"); }
}

#[test]
fn test_get_attr_pos() {
    let schema = test_schema();
    // C ground truth: 0, 4, 8
    assert_eq!(get_attr_pos(&schema, 0), 0);
    assert_eq!(get_attr_pos(&schema, 1), 4);
    assert_eq!(get_attr_pos(&schema, 2), 8);
}

#[test]
fn test_free_schema() {
    let mut schema = test_schema();
    let rc = free_schema(&mut schema);
    assert_eq!(rc, RC::Ok);
}

#[test]
fn test_free_record() {
    let schema = test_schema();
    let mut r = make_test_record(&schema, 1, "aaaa", 3);
    let rc = free_record(&mut r);
    assert_eq!(rc, RC::Ok);
    assert_eq!(r.data.len(), 0);
}

#[test]
fn test_init_and_shutdown_record_manager() {
    let rc = init_record_manager(None);
    assert_eq!(rc, RC::Ok);
    let rc = shutdown_record_manager();
    assert_eq!(rc, RC::Ok);
}

fn main() {}

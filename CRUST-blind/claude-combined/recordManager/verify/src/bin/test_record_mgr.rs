use recordManager::dberror::RC;
use recordManager::record_mgr::{
    init_record_manager, shutdown_record_manager,
    create_schema, get_record_size, get_attr_pos, free_schema,
    create_record, free_record, get_attr, set_attr,
    create_table, open_table, close_table, delete_table,
    get_num_tuples, insert_record_mut, get_record, update_record, delete_record,
    start_scan, next, close_scan, RM_ScanHandle,
};
use recordManager::tables::{
    DataType, Value, ValueUnion, Record, RID, Schema, RM_TableData,
};
use recordManager::expr::{Expr, ExprType, ExprUnion, OpType, Operator};
use recordManager::rm_serializer::string_to_value;

fn make_test_schema() -> Schema {
    create_schema(
        3,
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        vec![DataType::DtInt, DataType::DtString, DataType::DtInt],
        vec![0, 4, 0],
        1,
        vec![0],
    )
}

#[test]
fn test_init_shutdown() {
    let rc = init_record_manager(None);
    assert!(rc == RC::Ok);
    let rc = shutdown_record_manager();
    assert!(rc == RC::Ok);
}

#[test]
fn test_create_schema_fields() {
    let s = make_test_schema();
    assert_eq!(s.num_attr, 3);
    assert_eq!(s.attr_names, vec!["a", "b", "c"]);
    assert_eq!(s.type_length, vec![0, 4, 0]);
    assert_eq!(s.key_size, 1);
    assert_eq!(s.key_attrs, vec![0]);
}

#[test]
fn test_get_record_size() {
    let s = make_test_schema();
    // Per C: int(4) + string(4) + int(4) = 12, no padding
    assert_eq!(get_record_size(&s), 12);

    // Schema with bool: bool=1 byte, padded to 4
    let s2 = create_schema(
        1,
        vec!["b".to_string()],
        vec![DataType::DtBool],
        vec![0],
        0,
        vec![],
    );
    assert_eq!(get_record_size(&s2), 4);

    // Schema with bool + int: 1 + 4 = 5, padded to 8
    let s3 = create_schema(
        2,
        vec!["b".to_string(), "i".to_string()],
        vec![DataType::DtBool, DataType::DtInt],
        vec![0, 0],
        0,
        vec![],
    );
    assert_eq!(get_record_size(&s3), 8);
}

#[test]
fn test_get_attr_pos() {
    let s = make_test_schema();
    assert_eq!(get_attr_pos(&s, 0), 0);
    assert_eq!(get_attr_pos(&s, 1), 4);
    assert_eq!(get_attr_pos(&s, 2), 8);
}

#[test]
fn test_free_schema() {
    let mut s = make_test_schema();
    let rc = free_schema(&mut s);
    assert!(rc == RC::Ok);
}

#[test]
fn test_create_record_size() {
    let s = make_test_schema();
    let mut rec_opt: Option<Record> = None;
    let rc = create_record(&mut rec_opt, &s);
    assert!(rc == RC::Ok);
    let rec = rec_opt.unwrap();
    assert_eq!(rec.data.chars().count(), 12);
    assert_eq!(rec.id.page, 0);
    assert_eq!(rec.id.slot, 0);
}

#[test]
fn test_set_get_attr_int() {
    let s = make_test_schema();
    let mut rec_opt: Option<Record> = None;
    let _ = create_record(&mut rec_opt, &s);
    let mut rec = rec_opt.unwrap();
    let v = Value { dt: DataType::DtInt, v: ValueUnion::IntV(42) };
    let rc = set_attr(&mut rec, &s, 0, &v);
    assert!(rc == RC::Ok);
    let mut got = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    let rc = get_attr(&rec, &s, 0, &mut got);
    assert!(rc == RC::Ok);
    assert!(matches!(got.dt, DataType::DtInt));
    assert!(matches!(got.v, ValueUnion::IntV(42)));
}

#[test]
fn test_set_get_attr_string() {
    let s = make_test_schema();
    let mut rec_opt: Option<Record> = None;
    let _ = create_record(&mut rec_opt, &s);
    let mut rec = rec_opt.unwrap();
    let v = Value { dt: DataType::DtString, v: ValueUnion::StringV("aaaa".to_string()) };
    let rc = set_attr(&mut rec, &s, 1, &v);
    assert!(rc == RC::Ok);
    let mut got = Value { dt: DataType::DtString, v: ValueUnion::StringV(String::new()) };
    let rc = get_attr(&rec, &s, 1, &mut got);
    assert!(rc == RC::Ok);
    if let ValueUnion::StringV(s) = got.v {
        assert_eq!(s, "aaaa");
    } else {
        panic!("not string");
    }
}

#[test]
fn test_free_record() {
    let s = make_test_schema();
    let mut rec_opt: Option<Record> = None;
    let _ = create_record(&mut rec_opt, &s);
    let mut rec = rec_opt.unwrap();
    let rc = free_record(&mut rec);
    assert!(rc == RC::Ok);
}

fn from_test_record(s: &Schema, a: i32, b: &str, c: i32) -> Record {
    let mut rec_opt: Option<Record> = None;
    let _ = create_record(&mut rec_opt, s);
    let mut rec = rec_opt.unwrap();
    let _ = set_attr(&mut rec, s, 0, &Value { dt: DataType::DtInt, v: ValueUnion::IntV(a) });
    let _ = set_attr(&mut rec, s, 1, &Value { dt: DataType::DtString, v: ValueUnion::StringV(b.to_string()) });
    let _ = set_attr(&mut rec, s, 2, &Value { dt: DataType::DtInt, v: ValueUnion::IntV(c) });
    rec
}

fn empty_table_data() -> RM_TableData {
    RM_TableData {
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
    }
}

#[test]
fn test_create_open_close_delete_table() {
    let path = "/tmp/rm_test_basic.bin";
    let _ = delete_table(path);
    let s = make_test_schema();
    let rc = create_table(path, &s);
    assert!(rc == RC::Ok);
    let mut table = empty_table_data();
    let rc = open_table(&mut table, path);
    assert!(rc == RC::Ok);
    assert_eq!(table.name, path);
    assert_eq!(table.schema.num_attr, 3);
    assert_eq!(table.schema.attr_names, vec!["a", "b", "c"]);
    assert_eq!(table.schema.type_length, vec![0, 4, 0]);
    assert_eq!(table.schema.key_size, 1);
    assert_eq!(get_num_tuples(&table), 0);

    let rc = close_table(&mut table);
    assert!(rc == RC::Ok);
    let rc = delete_table(path);
    assert!(rc == RC::Ok);
}

#[test]
fn test_insert_get_record() {
    let path = "/tmp/rm_test_insert.bin";
    let _ = delete_table(path);
    let s = make_test_schema();
    let _ = create_table(path, &s);
    let mut table = empty_table_data();
    let _ = open_table(&mut table, path);

    let mut rec = from_test_record(&s, 1, "aaaa", 3);
    let rc = insert_record_mut(&mut table, &mut rec);
    assert!(rc == RC::Ok);
    assert_eq!(rec.id.page, 1);
    assert_eq!(rec.id.slot, 0);
    let rid = rec.id.clone();
    assert_eq!(get_num_tuples(&table), 1);

    let mut rec_opt: Option<Record> = None;
    let _ = create_record(&mut rec_opt, &s);
    let mut got = rec_opt.unwrap();
    let rc = get_record(&table, &rid, &mut got);
    assert!(rc == RC::Ok);
    let mut a_val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    let _ = get_attr(&got, &s, 0, &mut a_val);
    assert!(matches!(a_val.v, ValueUnion::IntV(1)));
    let mut b_val = Value { dt: DataType::DtString, v: ValueUnion::StringV(String::new()) };
    let _ = get_attr(&got, &s, 1, &mut b_val);
    if let ValueUnion::StringV(s) = b_val.v {
        assert_eq!(s, "aaaa");
    } else {
        panic!("not string");
    }
    let mut c_val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    let _ = get_attr(&got, &s, 2, &mut c_val);
    assert!(matches!(c_val.v, ValueUnion::IntV(3)));
    assert_eq!(got.id.page, rid.page);
    assert_eq!(got.id.slot, rid.slot);

    let _ = close_table(&mut table);
    let _ = delete_table(path);
}

#[test]
fn test_update_record() {
    let path = "/tmp/rm_test_update.bin";
    let _ = delete_table(path);
    let s = make_test_schema();
    let _ = create_table(path, &s);
    let mut table = empty_table_data();
    let _ = open_table(&mut table, path);

    let mut rec = from_test_record(&s, 1, "aaaa", 3);
    let _ = insert_record_mut(&mut table, &mut rec);
    let rid = rec.id.clone();

    let mut new_rec = from_test_record(&s, 9, "iiii", 6);
    new_rec.id = rid.clone();
    let rc = update_record(&mut table, &new_rec);
    assert!(rc == RC::Ok);

    let mut rec_opt: Option<Record> = None;
    let _ = create_record(&mut rec_opt, &s);
    let mut got = rec_opt.unwrap();
    let _ = get_record(&table, &rid, &mut got);
    let mut a_val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    let _ = get_attr(&got, &s, 0, &mut a_val);
    assert!(matches!(a_val.v, ValueUnion::IntV(9)));
    let mut c_val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    let _ = get_attr(&got, &s, 2, &mut c_val);
    assert!(matches!(c_val.v, ValueUnion::IntV(6)));

    let _ = close_table(&mut table);
    let _ = delete_table(path);
}

#[test]
fn test_delete_record() {
    let path = "/tmp/rm_test_delete.bin";
    let _ = delete_table(path);
    let s = make_test_schema();
    let _ = create_table(path, &s);
    let mut table = empty_table_data();
    let _ = open_table(&mut table, path);

    let mut rec = from_test_record(&s, 1, "aaaa", 3);
    let _ = insert_record_mut(&mut table, &mut rec);
    let rid = rec.id.clone();

    let rc = delete_record(&mut table, &rid);
    assert!(rc == RC::Ok);

    let mut rec_opt: Option<Record> = None;
    let _ = create_record(&mut rec_opt, &s);
    let mut got = rec_opt.unwrap();
    let rc = get_record(&table, &rid, &mut got);
    assert!(rc == RC::RecordNotFound);

    let _ = close_table(&mut table);
    let _ = delete_table(path);
}

#[test]
fn test_close_open_persistence() {
    let path = "/tmp/rm_test_persist.bin";
    let _ = delete_table(path);
    let s = make_test_schema();
    let _ = create_table(path, &s);
    let mut table = empty_table_data();
    let _ = open_table(&mut table, path);

    let mut rec = from_test_record(&s, 7, "zzzz", 11);
    let _ = insert_record_mut(&mut table, &mut rec);
    let rid = rec.id.clone();
    let _ = close_table(&mut table);

    let mut table = empty_table_data();
    let _ = open_table(&mut table, path);
    assert_eq!(get_num_tuples(&table), 1);
    let mut rec_opt: Option<Record> = None;
    let _ = create_record(&mut rec_opt, &s);
    let mut got = rec_opt.unwrap();
    let rc = get_record(&table, &rid, &mut got);
    assert!(rc == RC::Ok);
    let mut a_val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    let _ = get_attr(&got, &s, 0, &mut a_val);
    assert!(matches!(a_val.v, ValueUnion::IntV(7)));

    let _ = close_table(&mut table);
    let _ = delete_table(path);
}

#[test]
fn test_scan_with_filter() {
    let path = "/tmp/rm_test_scan.bin";
    let _ = delete_table(path);
    let s = make_test_schema();
    let _ = create_table(path, &s);
    let mut table = empty_table_data();
    let _ = open_table(&mut table, path);

    let inserts = [
        (1, "aaaa", 3),
        (2, "bbbb", 2),
        (3, "cccc", 1),
        (4, "dddd", 3),
        (5, "eeee", 5),
        (6, "ffff", 1),
    ];
    for (a, b, c) in inserts.iter() {
        let mut rec = from_test_record(&s, *a, b, *c);
        let _ = insert_record_mut(&mut table, &mut rec);
    }
    assert_eq!(get_num_tuples(&table), 6);

    // condition: c == 1
    let cond = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpCompEqual,
            args: vec![
                Expr { expr_type: ExprType::ExprConst, expr: ExprUnion::Cons(string_to_value("i1")) },
                Expr { expr_type: ExprType::ExprAttrRef, expr: ExprUnion::AttrRef(2) },
            ],
        })),
    };

    let mut scan = RM_ScanHandle {
        rel: empty_table_data(),
        mgmt_data: None,
    };
    let rc = start_scan(&table, &mut scan, &cond);
    assert!(rc == RC::Ok);

    let mut count = 0;
    let mut rec_opt: Option<Record> = None;
    let _ = create_record(&mut rec_opt, &s);
    let mut r = rec_opt.unwrap();
    let mut found_a3 = false;
    let mut found_a6 = false;
    loop {
        let rc = next(&mut scan, &mut r);
        if rc == RC::RmNoMoreTuples { break; }
        assert!(rc == RC::Ok);
        count += 1;
        let mut a_val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
        let _ = get_attr(&r, &s, 0, &mut a_val);
        if let ValueUnion::IntV(a) = a_val.v {
            if a == 3 { found_a3 = true; }
            if a == 6 { found_a6 = true; }
        }
    }
    assert_eq!(count, 2);
    assert!(found_a3);
    assert!(found_a6);
    let rc = close_scan(&mut scan);
    assert!(rc == RC::Ok);

    let _ = close_table(&mut table);
    let _ = delete_table(path);
}

#[test]
fn test_insert_many_records() {
    let path = "/tmp/rm_test_many.bin";
    let _ = delete_table(path);
    let s = make_test_schema();
    let _ = create_table(path, &s);
    let mut table = empty_table_data();
    let _ = open_table(&mut table, path);

    let n = 200;
    let mut rids = Vec::with_capacity(n);
    for i in 0..n {
        let mut rec = from_test_record(&s, i as i32, "aaaa", (i % 5) as i32);
        let _ = insert_record_mut(&mut table, &mut rec);
        rids.push(rec.id.clone());
    }
    assert_eq!(get_num_tuples(&table), n as i32);

    // Verify all
    let mut rec_opt: Option<Record> = None;
    let _ = create_record(&mut rec_opt, &s);
    let mut got = rec_opt.unwrap();
    for i in 0..n {
        let rc = get_record(&table, &rids[i], &mut got);
        assert!(rc == RC::Ok);
        let mut a_val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
        let _ = get_attr(&got, &s, 0, &mut a_val);
        if let ValueUnion::IntV(a) = a_val.v {
            assert_eq!(a, i as i32);
        }
    }

    let _ = close_table(&mut table);
    let _ = delete_table(path);
}

#[test]
fn test_get_record_invalid_slot() {
    let path = "/tmp/rm_test_invalid.bin";
    let _ = delete_table(path);
    let s = make_test_schema();
    let _ = create_table(path, &s);
    let mut table = empty_table_data();
    let _ = open_table(&mut table, path);

    let mut rec_opt: Option<Record> = None;
    let _ = create_record(&mut rec_opt, &s);
    let mut got = rec_opt.unwrap();
    let invalid = RID { page: 1, slot: 1_000_000 };
    let rc = get_record(&table, &invalid, &mut got);
    assert!(rc == RC::RecordNotFound);

    let _ = close_table(&mut table);
    let _ = delete_table(path);
}

fn main() {}

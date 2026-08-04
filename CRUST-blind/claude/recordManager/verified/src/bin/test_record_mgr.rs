use recordManager::dberror::RC;
use recordManager::expr::{Expr, ExprType, ExprUnion, OpType, Operator};
use recordManager::record_mgr::{
    close_scan, close_table, create_record, create_schema, create_table, delete_record,
    delete_table, free_record, free_schema, get_attr, get_attr_pos, get_num_tuples, get_record,
    get_record_size, init_record_manager, insert_record, next, open_table, set_attr,
    shutdown_record_manager, start_scan, update_record, RM_ScanHandle,
};
use recordManager::tables::{DataType, Record, Schema, Value, ValueUnion, RID, RM_TableData};

fn make_int(i: i32) -> Value {
    Value {
        dt: DataType::DtInt,
        v: ValueUnion::IntV(i),
    }
}
fn make_str_val(s: &str) -> Value {
    Value {
        dt: DataType::DtString,
        v: ValueUnion::StringV(s.to_string()),
    }
}

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

fn empty_table() -> RM_TableData {
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

fn build_test_record(schema: &Schema, a: i32, b: &str, c: i32) -> Record {
    let mut record_opt: Option<Record> = None;
    let _ = create_record(&mut record_opt, schema);
    let mut r = record_opt.unwrap();
    let _ = set_attr(&mut r, schema, 0, &make_int(a));
    let _ = set_attr(&mut r, schema, 1, &make_str_val(b));
    let _ = set_attr(&mut r, schema, 2, &make_int(c));
    r
}

#[test]
fn test_get_record_size() {
    let s = test_schema();
    // INT (4) + STRING(4) + INT (4) = 12, padded = 12
    assert_eq!(get_record_size(&s), 12);
}

#[test]
fn test_get_record_size_with_padding() {
    // INT(4) + BOOL(1) = 5, padded to 8
    let s = create_schema(
        2,
        vec!["x".to_string(), "y".to_string()],
        vec![DataType::DtInt, DataType::DtBool],
        vec![0, 0],
        0,
        vec![],
    );
    assert_eq!(get_record_size(&s), 8);
}

#[test]
fn test_get_attr_pos() {
    let s = test_schema();
    assert_eq!(get_attr_pos(&s, 0), 0);
    assert_eq!(get_attr_pos(&s, 1), 4);
    assert_eq!(get_attr_pos(&s, 2), 8);
}

#[test]
fn test_create_schema() {
    let s = test_schema();
    assert_eq!(s.num_attr, 3);
    assert_eq!(s.key_size, 1);
    assert_eq!(s.attr_names[0], "a");
    assert_eq!(s.attr_names[1], "b");
    assert_eq!(s.attr_names[2], "c");
    assert!(matches!(s.data_types[0], DataType::DtInt));
    assert!(matches!(s.data_types[1], DataType::DtString));
    assert!(matches!(s.data_types[2], DataType::DtInt));
    assert_eq!(s.type_length, vec![0, 4, 0]);
    assert_eq!(s.key_attrs, vec![0]);
}

#[test]
fn test_create_record_and_attrs() {
    let s = test_schema();
    let mut record: Option<Record> = None;
    assert_eq!(create_record(&mut record, &s), RC::Ok);
    let mut r = record.unwrap();
    // Initially zeroes -> reading int yields 0
    let mut v = make_int(-1);
    assert_eq!(get_attr(&r, &s, 0, &mut v), RC::Ok);
    if let ValueUnion::IntV(i) = v.v {
        assert_eq!(i, 0);
    }

    // Set attrs
    let _ = set_attr(&mut r, &s, 0, &make_int(42));
    let _ = set_attr(&mut r, &s, 1, &make_str_val("test"));
    let _ = set_attr(&mut r, &s, 2, &make_int(7));

    let mut v = make_int(-1);
    assert_eq!(get_attr(&r, &s, 0, &mut v), RC::Ok);
    if let ValueUnion::IntV(i) = v.v {
        assert_eq!(i, 42);
    } else {
        panic!("not int");
    }

    let mut v = make_int(-1);
    assert_eq!(get_attr(&r, &s, 1, &mut v), RC::Ok);
    if let ValueUnion::StringV(st) = v.v {
        assert_eq!(st, "test");
    } else {
        panic!("not string");
    }

    let mut v = make_int(-1);
    assert_eq!(get_attr(&r, &s, 2, &mut v), RC::Ok);
    if let ValueUnion::IntV(i) = v.v {
        assert_eq!(i, 7);
    } else {
        panic!("not int");
    }

    let _ = free_record(&mut r);
}

#[test]
fn test_init_and_shutdown_record_manager() {
    assert_eq!(init_record_manager(None), RC::Ok);
    assert_eq!(shutdown_record_manager(), RC::Ok);
}

#[test]
fn test_create_open_close_delete_table() {
    let s = test_schema();
    let _ = init_record_manager(None);
    assert_eq!(create_table("test_rm_create", &s), RC::Ok);

    let mut rel = empty_table();
    assert_eq!(open_table(&mut rel, "test_rm_create"), RC::Ok);
    assert_eq!(rel.name, "test_rm_create");
    // Schema preserved
    assert_eq!(rel.schema.num_attr, 3);
    assert_eq!(rel.schema.attr_names[0], "a");
    assert_eq!(rel.schema.attr_names[1], "b");
    assert_eq!(rel.schema.attr_names[2], "c");

    // No tuples yet
    assert_eq!(get_num_tuples(&rel), 0);

    assert_eq!(close_table(&mut rel), RC::Ok);
    assert_eq!(delete_table("test_rm_create"), RC::Ok);
    let _ = shutdown_record_manager();
}

#[test]
fn test_delete_table_empty_name() {
    assert_eq!(delete_table(""), RC::InvalidHeader);
}

#[test]
fn test_create_table_empty_name() {
    assert_eq!(create_table("", &test_schema()), RC::GeneralError);
}

#[test]
fn test_insert_and_get_record() {
    let s = test_schema();
    let _ = init_record_manager(None);
    let _ = delete_table("test_rm_insert");
    assert_eq!(create_table("test_rm_insert", &s), RC::Ok);
    let mut rel = empty_table();
    assert_eq!(open_table(&mut rel, "test_rm_insert"), RC::Ok);

    let r = build_test_record(&rel.schema, 1, "aaaa", 3);
    assert_eq!(insert_record(&mut rel, &r), RC::Ok);

    // After insert, the record's id is set
    let id = r.id.clone();
    assert_eq!(id.page, 1);
    assert_eq!(id.slot, 0);

    // get_record should return the same data
    let mut got: Option<Record> = None;
    let _ = create_record(&mut got, &rel.schema);
    let mut g = got.unwrap();
    let rc = get_record(&rel, &id, &mut g);
    assert_eq!(rc, RC::Ok);

    // Verify attrs
    let mut v = make_int(-1);
    assert_eq!(get_attr(&g, &rel.schema, 0, &mut v), RC::Ok);
    if let ValueUnion::IntV(i) = v.v {
        assert_eq!(i, 1);
    }
    let mut v = make_int(-1);
    assert_eq!(get_attr(&g, &rel.schema, 1, &mut v), RC::Ok);
    if let ValueUnion::StringV(s) = v.v {
        assert_eq!(s, "aaaa");
    }
    let mut v = make_int(-1);
    assert_eq!(get_attr(&g, &rel.schema, 2, &mut v), RC::Ok);
    if let ValueUnion::IntV(i) = v.v {
        assert_eq!(i, 3);
    }

    assert_eq!(get_num_tuples(&rel), 1);
    assert_eq!(close_table(&mut rel), RC::Ok);
    assert_eq!(delete_table("test_rm_insert"), RC::Ok);
    let _ = shutdown_record_manager();
}

#[test]
fn test_update_record() {
    let s = test_schema();
    let _ = init_record_manager(None);
    let _ = delete_table("test_rm_update");
    assert_eq!(create_table("test_rm_update", &s), RC::Ok);
    let mut rel = empty_table();
    assert_eq!(open_table(&mut rel, "test_rm_update"), RC::Ok);

    let r = build_test_record(&rel.schema, 5, "bbbb", 6);
    assert_eq!(insert_record(&mut rel, &r), RC::Ok);
    let id = r.id.clone();

    // Update with new values
    let mut update = build_test_record(&rel.schema, 99, "zzzz", 100);
    update.id = id.clone();
    assert_eq!(update_record(&mut rel, &update), RC::Ok);

    let mut got: Option<Record> = None;
    let _ = create_record(&mut got, &rel.schema);
    let mut g = got.unwrap();
    assert_eq!(get_record(&rel, &id, &mut g), RC::Ok);

    let mut v = make_int(-1);
    assert_eq!(get_attr(&g, &rel.schema, 0, &mut v), RC::Ok);
    if let ValueUnion::IntV(i) = v.v {
        assert_eq!(i, 99);
    }
    let mut v = make_int(-1);
    assert_eq!(get_attr(&g, &rel.schema, 1, &mut v), RC::Ok);
    if let ValueUnion::StringV(st) = v.v {
        assert_eq!(st, "zzzz");
    }
    let mut v = make_int(-1);
    assert_eq!(get_attr(&g, &rel.schema, 2, &mut v), RC::Ok);
    if let ValueUnion::IntV(i) = v.v {
        assert_eq!(i, 100);
    }

    let _ = close_table(&mut rel);
    let _ = delete_table("test_rm_update");
    let _ = shutdown_record_manager();
}

#[test]
fn test_delete_record() {
    let s = test_schema();
    let _ = init_record_manager(None);
    let _ = delete_table("test_rm_delete");
    assert_eq!(create_table("test_rm_delete", &s), RC::Ok);
    let mut rel = empty_table();
    assert_eq!(open_table(&mut rel, "test_rm_delete"), RC::Ok);

    let r = build_test_record(&rel.schema, 7, "dddd", 9);
    assert_eq!(insert_record(&mut rel, &r), RC::Ok);
    let id = r.id.clone();
    assert_eq!(get_num_tuples(&rel), 1);

    assert_eq!(delete_record(&mut rel, &id), RC::Ok);
    assert_eq!(get_num_tuples(&rel), 0);

    // Now retrieving the deleted record returns RecordNotFound
    let mut got: Option<Record> = None;
    let _ = create_record(&mut got, &rel.schema);
    let mut g = got.unwrap();
    assert_eq!(get_record(&rel, &id, &mut g), RC::RecordNotFound);

    let _ = close_table(&mut rel);
    let _ = delete_table("test_rm_delete");
    let _ = shutdown_record_manager();
}

#[test]
fn test_delete_record_invalid_slot() {
    let s = test_schema();
    let _ = init_record_manager(None);
    let _ = delete_table("test_rm_delinv");
    assert_eq!(create_table("test_rm_delinv", &s), RC::Ok);
    let mut rel = empty_table();
    assert_eq!(open_table(&mut rel, "test_rm_delinv"), RC::Ok);

    let bad_id = RID {
        page: 1,
        slot: 99999,
    };
    assert_eq!(delete_record(&mut rel, &bad_id), RC::RecordNotFound);

    let _ = close_table(&mut rel);
    let _ = delete_table("test_rm_delinv");
    let _ = shutdown_record_manager();
}

#[test]
fn test_scan_with_condition() {
    let s = test_schema();
    let _ = init_record_manager(None);
    let _ = delete_table("test_rm_scan");
    assert_eq!(create_table("test_rm_scan", &s), RC::Ok);
    let mut rel = empty_table();
    assert_eq!(open_table(&mut rel, "test_rm_scan"), RC::Ok);

    // Insert records with c=1, c=2, c=1
    let mut r1 = build_test_record(&rel.schema, 1, "aaaa", 1);
    let _ = insert_record(&mut rel, &r1);
    let mut r2 = build_test_record(&rel.schema, 2, "bbbb", 2);
    let _ = insert_record(&mut rel, &r2);
    let mut r3 = build_test_record(&rel.schema, 3, "cccc", 1);
    let _ = insert_record(&mut rel, &r3);
    drop(r1);
    drop(r2);
    drop(r3);

    // Scan: c == 1 -> 2 records
    let left = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(1)),
    };
    let right = Expr {
        expr_type: ExprType::ExprAttrRef,
        expr: ExprUnion::AttrRef(2),
    };
    let cond = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpCompEqual,
            args: vec![left, right],
        })),
    };

    let mut scan = RM_ScanHandle {
        rel: empty_table(),
        mgmt_data: None,
    };
    assert_eq!(start_scan(&rel, &mut scan, &cond), RC::Ok);
    let mut got: Option<Record> = None;
    let _ = create_record(&mut got, &rel.schema);
    let mut g = got.unwrap();

    let mut count = 0;
    loop {
        let rc = next(&mut scan, &mut g);
        if rc == RC::RmNoMoreTuples {
            break;
        }
        assert_eq!(rc, RC::Ok);
        count += 1;
    }
    assert_eq!(count, 2);
    assert_eq!(close_scan(&mut scan), RC::Ok);

    let _ = close_table(&mut rel);
    let _ = delete_table("test_rm_scan");
    let _ = shutdown_record_manager();
}

#[test]
fn test_get_num_tuples_no_mgmt() {
    let rel = empty_table();
    assert_eq!(get_num_tuples(&rel), -1);
}

#[test]
fn test_free_schema_returns_ok() {
    let mut s = test_schema();
    assert_eq!(free_schema(&mut s), RC::Ok);
}

fn main() {}

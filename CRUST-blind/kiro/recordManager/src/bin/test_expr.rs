use recordManager::dberror::RC;
use recordManager::tables::*;
use recordManager::expr::*;

fn make_int(v: i32) -> Value {
    Value { dt: DataType::DtInt, v: ValueUnion::IntV(v) }
}
fn make_float(v: f32) -> Value {
    Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(v) }
}
fn make_bool(v: bool) -> Value {
    Value { dt: DataType::DtBool, v: ValueUnion::BoolV(v) }
}
fn make_string(v: &str) -> Value {
    Value { dt: DataType::DtString, v: ValueUnion::StringV(v.to_string()) }
}
fn get_bool_val(v: &Value) -> bool {
    match &v.v { ValueUnion::BoolV(b) => *b, _ => panic!("expected BoolV") }
}

// ---- valueEquals tests ----

#[test]
fn test_value_equals_int_equal() {
    let mut result = make_int(0);
    let rc = value_equals(&make_int(10), &make_int(10), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_value_equals_int_not_equal() {
    let mut result = make_int(0);
    let rc = value_equals(&make_int(9), &make_int(10), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(!get_bool_val(&result));
}

#[test]
fn test_value_equals_string_equal() {
    let mut result = make_int(0);
    let rc = value_equals(&make_string("Hello World"), &make_string("Hello World"), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_value_equals_string_not_equal() {
    let mut result = make_int(0);
    let rc = value_equals(&make_string("abc"), &make_string("def"), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(!get_bool_val(&result));
}

#[test]
fn test_value_equals_float_equal() {
    let mut result = make_int(0);
    let rc = value_equals(&make_float(3.14), &make_float(3.14), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_value_equals_bool_equal() {
    let mut result = make_int(0);
    let rc = value_equals(&make_bool(true), &make_bool(true), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_value_equals_bool_not_equal() {
    let mut result = make_int(0);
    let rc = value_equals(&make_bool(true), &make_bool(false), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(!get_bool_val(&result));
}

#[test]
fn test_value_equals_different_types() {
    let mut result = make_int(0);
    let rc = value_equals(&make_int(5), &make_float(5.0), &mut result);
    assert_eq!(rc, RC::RmCompareValueOfDifferentDatatype);
}

// ---- valueSmaller tests ----

#[test]
fn test_value_smaller_int_true() {
    let mut result = make_int(0);
    let rc = value_smaller(&make_int(3), &make_int(10), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_value_smaller_int_false() {
    let mut result = make_int(0);
    let rc = value_smaller(&make_int(10), &make_int(3), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(!get_bool_val(&result));
}

#[test]
fn test_value_smaller_int_equal() {
    let mut result = make_int(0);
    let rc = value_smaller(&make_int(5), &make_int(5), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(!get_bool_val(&result));
}

#[test]
fn test_value_smaller_float_true() {
    let mut result = make_int(0);
    let rc = value_smaller(&make_float(5.0), &make_float(6.5), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_value_smaller_string_true() {
    let mut result = make_int(0);
    let rc = value_smaller(&make_string("abc"), &make_string("def"), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_value_smaller_different_types() {
    let mut result = make_int(0);
    let rc = value_smaller(&make_int(5), &make_float(5.0), &mut result);
    assert_eq!(rc, RC::RmCompareValueOfDifferentDatatype);
}

// Note: valueSmaller for DT_BOOL crashes in C (fall-through to strcmp).
// The Rust code handles it correctly with match. We test the Rust behavior.
#[test]
fn test_value_smaller_bool() {
    let mut result = make_int(0);
    let rc = value_smaller(&make_bool(false), &make_bool(true), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

// ---- boolNot tests ----

#[test]
fn test_bool_not_true() {
    let mut result = make_int(0);
    let rc = bool_not(&make_bool(true), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.dt, DataType::DtBool));
    assert!(!get_bool_val(&result));
}

#[test]
fn test_bool_not_false() {
    let mut result = make_int(0);
    let rc = bool_not(&make_bool(false), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.dt, DataType::DtBool));
    assert!(get_bool_val(&result));
}

#[test]
fn test_bool_not_non_bool() {
    let mut result = make_int(0);
    let rc = bool_not(&make_int(5), &mut result);
    assert_eq!(rc, RC::RmBooleanExprArgIsNotBoolean);
}

// ---- boolAnd tests ----

#[test]
fn test_bool_and_true_true() {
    let mut result = make_int(0);
    let rc = bool_and(&make_bool(true), &make_bool(true), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_bool_and_true_false() {
    let mut result = make_int(0);
    let rc = bool_and(&make_bool(true), &make_bool(false), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(!get_bool_val(&result));
}

#[test]
fn test_bool_and_false_false() {
    let mut result = make_int(0);
    let rc = bool_and(&make_bool(false), &make_bool(false), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(!get_bool_val(&result));
}

#[test]
fn test_bool_and_non_bool() {
    let mut result = make_int(0);
    let rc = bool_and(&make_int(5), &make_bool(true), &mut result);
    assert_eq!(rc, RC::RmBooleanExprArgIsNotBoolean);
}

// ---- boolOr tests ----

#[test]
fn test_bool_or_true_false() {
    let mut result = make_int(0);
    let rc = bool_or(&make_bool(true), &make_bool(false), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_bool_or_false_false() {
    let mut result = make_int(0);
    let rc = bool_or(&make_bool(false), &make_bool(false), &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(!get_bool_val(&result));
}

#[test]
fn test_bool_or_non_bool() {
    let mut result = make_int(0);
    let rc = bool_or(&make_int(5), &make_bool(true), &mut result);
    assert_eq!(rc, RC::RmBooleanExprArgIsNotBoolean);
}

// ---- evalExpr tests ----

#[test]
fn test_eval_expr_const_int() {
    let expr = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(10)),
    };
    let record = Record { id: RID { page: 0, slot: 0 }, data: String::new() };
    let schema = recordManager::record_mgr::create_schema(
        1, vec!["a".into()], vec![DataType::DtInt], vec![0], 1, vec![0],
    );
    let mut result = make_int(0);
    let rc = eval_expr(&record, &schema, &expr, &mut result);
    assert_eq!(rc, RC::Ok);
    match result.v {
        ValueUnion::IntV(v) => assert_eq!(v, 10),
        _ => panic!("expected IntV"),
    }
}

#[test]
fn test_eval_expr_const_bool() {
    let expr = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_bool(true)),
    };
    let record = Record { id: RID { page: 0, slot: 0 }, data: String::new() };
    let schema = recordManager::record_mgr::create_schema(
        1, vec!["a".into()], vec![DataType::DtInt], vec![0], 1, vec![0],
    );
    let mut result = make_int(0);
    let rc = eval_expr(&record, &schema, &expr, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_eval_expr_comp_smaller() {
    let left = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(10)),
    };
    let right = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(20)),
    };
    let op = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpCompSmaller,
            args: vec![left, right],
        })),
    };
    let record = Record { id: RID { page: 0, slot: 0 }, data: String::new() };
    let schema = recordManager::record_mgr::create_schema(
        1, vec!["a".into()], vec![DataType::DtInt], vec![0], 1, vec![0],
    );
    let mut result = make_int(0);
    let rc = eval_expr(&record, &schema, &op, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_eval_expr_bool_and() {
    // (10 < 20) AND true => true
    let left = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(10)),
    };
    let right = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(20)),
    };
    let smaller = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpCompSmaller,
            args: vec![left, right],
        })),
    };
    let bool_const = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_bool(true)),
    };
    let and_expr = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpBoolAnd,
            args: vec![smaller, bool_const],
        })),
    };
    let record = Record { id: RID { page: 0, slot: 0 }, data: String::new() };
    let schema = recordManager::record_mgr::create_schema(
        1, vec!["a".into()], vec![DataType::DtInt], vec![0], 1, vec![0],
    );
    let mut result = make_int(0);
    let rc = eval_expr(&record, &schema, &and_expr, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_eval_expr_bool_not() {
    let inner = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_bool(false)),
    };
    let not_expr = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpBoolNot,
            args: vec![inner],
        })),
    };
    let record = Record { id: RID { page: 0, slot: 0 }, data: String::new() };
    let schema = recordManager::record_mgr::create_schema(
        1, vec!["a".into()], vec![DataType::DtInt], vec![0], 1, vec![0],
    );
    let mut result = make_int(0);
    let rc = eval_expr(&record, &schema, &not_expr, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

#[test]
fn test_eval_expr_comp_equal() {
    let left = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(10)),
    };
    let right = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(10)),
    };
    let op = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpCompEqual,
            args: vec![left, right],
        })),
    };
    let record = Record { id: RID { page: 0, slot: 0 }, data: String::new() };
    let schema = recordManager::record_mgr::create_schema(
        1, vec!["a".into()], vec![DataType::DtInt], vec![0], 1, vec![0],
    );
    let mut result = make_int(0);
    let rc = eval_expr(&record, &schema, &op, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(get_bool_val(&result));
}

// ---- free_expr / free_val (no-ops in Rust) ----

#[test]
fn test_free_expr() {
    let mut expr = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(10)),
    };
    assert_eq!(free_expr(&mut expr), RC::Ok);
}

#[test]
fn test_free_val() {
    let mut val = make_int(42);
    free_val(&mut val); // should not panic
}

fn main() {}

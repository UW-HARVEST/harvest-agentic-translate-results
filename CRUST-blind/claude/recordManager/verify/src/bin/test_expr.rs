use recordManager::dberror::RC;
use recordManager::expr::{
    bool_and, bool_not, bool_or, eval_expr, free_expr, free_val, value_equals, value_smaller, Expr,
    ExprType, ExprUnion, OpType, Operator,
};
use recordManager::tables::{DataType, Record, Schema, Value, ValueUnion, RID};

fn make_int(i: i32) -> Value {
    Value {
        dt: DataType::DtInt,
        v: ValueUnion::IntV(i),
    }
}
fn make_float(f: f32) -> Value {
    Value {
        dt: DataType::DtFloat,
        v: ValueUnion::FloatV(f),
    }
}
fn make_str(s: &str) -> Value {
    Value {
        dt: DataType::DtString,
        v: ValueUnion::StringV(s.to_string()),
    }
}
fn make_bool(b: bool) -> Value {
    Value {
        dt: DataType::DtBool,
        v: ValueUnion::BoolV(b),
    }
}
fn make_int_result() -> Value {
    Value {
        dt: DataType::DtInt,
        v: ValueUnion::IntV(-1),
    }
}

fn assert_bool_value(v: &Value, expected: bool) {
    match (&v.dt, &v.v) {
        (DataType::DtBool, ValueUnion::BoolV(b)) => assert_eq!(*b, expected),
        _ => panic!("expected bool value, got {:?}", v),
    }
}

#[test]
fn test_value_equals_int() {
    let mut result = make_int_result();
    let rc = value_equals(&make_int(10), &make_int(10), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, true);

    let mut result = make_int_result();
    let rc = value_equals(&make_int(9), &make_int(10), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, false);
}

#[test]
fn test_value_equals_string() {
    let mut result = make_int_result();
    let rc = value_equals(&make_str("Hello World"), &make_str("Hello World"), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, true);

    let mut result = make_int_result();
    let rc = value_equals(&make_str("Hello Worl"), &make_str("Hello World"), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, false);

    let mut result = make_int_result();
    let rc = value_equals(&make_str("Hello Worl"), &make_str("Hello Wor"), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, false);
}

#[test]
fn test_value_equals_different_types() {
    let mut result = make_int_result();
    let rc = value_equals(&make_int(10), &make_float(10.0), &mut result);
    assert_eq!(rc, RC::RmCompareValueOfDifferentDatatype);
}

#[test]
fn test_value_smaller_int() {
    let mut result = make_int_result();
    let rc = value_smaller(&make_int(3), &make_int(10), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, true);

    let mut result = make_int_result();
    let rc = value_smaller(&make_int(10), &make_int(3), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, false);
}

#[test]
fn test_value_smaller_float() {
    let mut result = make_int_result();
    let rc = value_smaller(&make_float(5.0), &make_float(6.5), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, true);
}

#[test]
fn test_value_smaller_different_types() {
    let mut result = make_int_result();
    let rc = value_smaller(&make_int(10), &make_str("hello"), &mut result);
    assert_eq!(rc, RC::RmCompareValueOfDifferentDatatype);
}

#[test]
fn test_bool_and() {
    let mut result = make_int_result();
    let rc = bool_and(&make_bool(true), &make_bool(true), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, true);

    let mut result = make_int_result();
    let rc = bool_and(&make_bool(true), &make_bool(false), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, false);

    let mut result = make_int_result();
    let rc = bool_and(&make_bool(false), &make_bool(true), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, false);
}

#[test]
fn test_bool_and_non_bool() {
    let mut result = make_int_result();
    let rc = bool_and(&make_int(1), &make_bool(true), &mut result);
    assert_eq!(rc, RC::RmBooleanExprArgIsNotBoolean);
}

#[test]
fn test_bool_or() {
    let mut result = make_int_result();
    let rc = bool_or(&make_bool(true), &make_bool(false), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, true);

    let mut result = make_int_result();
    let rc = bool_or(&make_bool(false), &make_bool(false), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, false);
}

#[test]
fn test_bool_or_non_bool() {
    let mut result = make_int_result();
    let rc = bool_or(&make_str("hi"), &make_bool(true), &mut result);
    assert_eq!(rc, RC::RmBooleanExprArgIsNotBoolean);
}

#[test]
fn test_bool_not() {
    let mut result = make_int_result();
    let rc = bool_not(&make_bool(false), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, true);

    let mut result = make_int_result();
    let rc = bool_not(&make_bool(true), &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, false);
}

#[test]
fn test_bool_not_non_bool() {
    let mut result = make_int_result();
    let rc = bool_not(&make_int(1), &mut result);
    assert_eq!(rc, RC::RmBooleanExprArgIsNotBoolean);
}

#[test]
fn test_eval_expr_const() {
    let mut record = Record {
        id: RID { page: 0, slot: 0 },
        data: String::new(),
    };
    let schema = Schema {
        num_attr: 0,
        attr_names: vec![],
        data_types: vec![],
        type_length: vec![],
        key_attrs: vec![],
        key_size: 0,
    };
    let expr = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(10)),
    };
    let mut result = make_int_result();
    let rc = eval_expr(&mut record, &schema, &expr, &mut result);
    assert_eq!(rc, RC::Ok);
    if let ValueUnion::IntV(i) = result.v {
        assert_eq!(i, 10);
    } else {
        panic!("not int");
    }
}

#[test]
fn test_eval_expr_op_smaller() {
    let mut record = Record {
        id: RID { page: 0, slot: 0 },
        data: String::new(),
    };
    let schema = Schema {
        num_attr: 0,
        attr_names: vec![],
        data_types: vec![],
        type_length: vec![],
        key_attrs: vec![],
        key_size: 0,
    };

    let l = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(10)),
    };
    let r = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(20)),
    };
    let op_expr = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpCompSmaller,
            args: vec![l, r],
        })),
    };
    let mut result = make_int_result();
    let rc = eval_expr(&mut record, &schema, &op_expr, &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, true);
}

#[test]
fn test_eval_expr_op_and_complex() {
    let mut record = Record {
        id: RID { page: 0, slot: 0 },
        data: String::new(),
    };
    let schema = Schema {
        num_attr: 0,
        attr_names: vec![],
        data_types: vec![],
        type_length: vec![],
        key_attrs: vec![],
        key_size: 0,
    };

    // (10 < 20) AND true => true
    let l = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(10)),
    };
    let r = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(20)),
    };
    let smaller = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpCompSmaller,
            args: vec![l, r],
        })),
    };
    let cnst_true = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_bool(true)),
    };
    let and_expr = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpBoolAnd,
            args: vec![smaller, cnst_true],
        })),
    };
    let mut result = make_int_result();
    let rc = eval_expr(&mut record, &schema, &and_expr, &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, true);
}

#[test]
fn test_eval_expr_not() {
    let mut record = Record {
        id: RID { page: 0, slot: 0 },
        data: String::new(),
    };
    let schema = Schema {
        num_attr: 0,
        attr_names: vec![],
        data_types: vec![],
        type_length: vec![],
        key_attrs: vec![],
        key_size: 0,
    };
    let cnst = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_bool(false)),
    };
    let not_expr = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpBoolNot,
            args: vec![cnst],
        })),
    };
    let mut result = make_int_result();
    let rc = eval_expr(&mut record, &schema, &not_expr, &mut result);
    assert_eq!(rc, RC::Ok);
    assert_bool_value(&result, true);
}

#[test]
fn test_free_expr_and_val_noop() {
    let mut e = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(make_int(5)),
    };
    assert_eq!(free_expr(&mut e), RC::Ok);
    let mut v = make_int(0);
    free_val(&mut v);
}

fn main() {}

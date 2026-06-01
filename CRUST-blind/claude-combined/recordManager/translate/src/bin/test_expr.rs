use recordManager::dberror::RC;
use recordManager::expr::{
    value_equals, value_smaller, bool_not, bool_and, bool_or, eval_expr,
    Expr, ExprType, ExprUnion, Operator, OpType,
};
use recordManager::tables::{DataType, Value, ValueUnion, Record, Schema, RID};
use recordManager::rm_serializer::string_to_value;

fn dummy_record() -> Record {
    Record {
        id: RID { page: 0, slot: 0 },
        data: String::new(),
    }
}

fn dummy_schema() -> Schema {
    Schema {
        num_attr: 0,
        attr_names: vec![],
        data_types: vec![],
        type_length: vec![],
        key_attrs: vec![],
        key_size: 0,
    }
}

#[test]
fn test_value_equals_int_equal() {
    let l = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let r = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let rc = value_equals(&l, &r, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.dt, DataType::DtBool));
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_value_equals_int_unequal() {
    let l = Value { dt: DataType::DtInt, v: ValueUnion::IntV(9) };
    let r = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let rc = value_equals(&l, &r, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(false)));
}

#[test]
fn test_value_equals_string_equal() {
    let l = Value { dt: DataType::DtString, v: ValueUnion::StringV("Hello World".to_string()) };
    let r = Value { dt: DataType::DtString, v: ValueUnion::StringV("Hello World".to_string()) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let rc = value_equals(&l, &r, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_value_equals_string_unequal() {
    let l = Value { dt: DataType::DtString, v: ValueUnion::StringV("Hello Worl".to_string()) };
    let r = Value { dt: DataType::DtString, v: ValueUnion::StringV("Hello World".to_string()) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let rc = value_equals(&l, &r, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(false)));
}

#[test]
fn test_value_equals_different_dt_returns_error() {
    let l = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let r = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(10.0) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let rc = value_equals(&l, &r, &mut result);
    assert!(rc == RC::RmCompareValueOfDifferentDatatype);
}

#[test]
fn test_value_smaller_int() {
    let l = Value { dt: DataType::DtInt, v: ValueUnion::IntV(3) };
    let r = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let rc = value_smaller(&l, &r, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_value_smaller_float() {
    let l = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(5.0) };
    let r = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(6.5) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let rc = value_smaller(&l, &r, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_bool_and_true_true() {
    let l = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let r = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let rc = bool_and(&l, &r, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_bool_and_true_false() {
    let l = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let r = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let rc = bool_and(&l, &r, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(false)));
}

#[test]
fn test_bool_or() {
    let l = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let r = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let rc = bool_or(&l, &r, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));

    let l = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let r = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let rc = bool_or(&l, &r, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(false)));
}

#[test]
fn test_bool_not() {
    let f = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let rc = bool_not(&f, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_bool_not_non_bool_input() {
    let v = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
    let mut result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let rc = bool_not(&v, &mut result);
    assert!(rc == RC::RmBooleanExprArgIsNotBoolean);
}

#[test]
fn test_eval_expr_const() {
    // EXPR_CONST for stringToValue("i10") -> Int(10)
    let cons_val = string_to_value("i10");
    let expr = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(cons_val),
    };
    let r = dummy_record();
    let s = dummy_schema();
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = eval_expr(&r, &s, &expr, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::IntV(10)));
}

#[test]
fn test_eval_expr_smaller() {
    let l_expr = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(string_to_value("i10")),
    };
    let r_expr = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(string_to_value("i20")),
    };
    let op = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpCompSmaller,
            args: vec![l_expr, r_expr],
        })),
    };
    let r = dummy_record();
    let s = dummy_schema();
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = eval_expr(&r, &s, &op, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_eval_expr_and() {
    // (10 < 20) AND true
    let l_expr = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(string_to_value("i10")),
    };
    let r_expr = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(string_to_value("i20")),
    };
    let smaller = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpCompSmaller,
            args: vec![l_expr, r_expr],
        })),
    };
    let bool_true = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(string_to_value("bt")),
    };
    let and_expr = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpBoolAnd,
            args: vec![smaller, bool_true],
        })),
    };
    let r = dummy_record();
    let s = dummy_schema();
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = eval_expr(&r, &s, &and_expr, &mut result);
    assert!(rc == RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

fn main() {}

use recordManager::expr::*;
use recordManager::tables::*;
use recordManager::dberror::RC;

#[test]
fn test_value_equals_int_same() {
    let left = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let right = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = value_equals(&left, &right, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.dt, DataType::DtBool));
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_value_equals_int_diff() {
    let left = Value { dt: DataType::DtInt, v: ValueUnion::IntV(9) };
    let right = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = value_equals(&left, &right, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.dt, DataType::DtBool));
    assert!(matches!(result.v, ValueUnion::BoolV(false)));
}

#[test]
fn test_value_equals_string_same() {
    let left = Value { dt: DataType::DtString, v: ValueUnion::StringV("Hello World".to_string()) };
    let right = Value { dt: DataType::DtString, v: ValueUnion::StringV("Hello World".to_string()) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = value_equals(&left, &right, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_value_equals_string_diff() {
    let left = Value { dt: DataType::DtString, v: ValueUnion::StringV("Hello Worl".to_string()) };
    let right = Value { dt: DataType::DtString, v: ValueUnion::StringV("Hello World".to_string()) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = value_equals(&left, &right, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(false)));
}

#[test]
fn test_value_equals_different_types() {
    let left = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let right = Value { dt: DataType::DtString, v: ValueUnion::StringV("Hello".to_string()) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = value_equals(&left, &right, &mut result);
    assert_eq!(rc, RC::RmCompareValueOfDifferentDatatype);
}

#[test]
fn test_value_smaller_int() {
    let left = Value { dt: DataType::DtInt, v: ValueUnion::IntV(3) };
    let right = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = value_smaller(&left, &right, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_value_smaller_int_not() {
    let left = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let right = Value { dt: DataType::DtInt, v: ValueUnion::IntV(3) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = value_smaller(&left, &right, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(false)));
}

#[test]
fn test_value_smaller_float() {
    let left = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(5.0) };
    let right = Value { dt: DataType::DtFloat, v: ValueUnion::FloatV(6.5) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = value_smaller(&left, &right, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_bool_not_false() {
    let input = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = bool_not(&input, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.dt, DataType::DtBool));
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_bool_not_true() {
    let input = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = bool_not(&input, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(false)));
}

#[test]
fn test_bool_not_non_bool() {
    let input = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = bool_not(&input, &mut result);
    assert_eq!(rc, RC::RmBooleanExprArgIsNotBoolean);
}

#[test]
fn test_bool_and_tt() {
    let left = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let right = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = bool_and(&left, &right, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_bool_and_tf() {
    let left = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let right = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = bool_and(&left, &right, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(false)));
}

#[test]
fn test_bool_and_non_bool() {
    let left = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    let right = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = bool_and(&left, &right, &mut result);
    assert_eq!(rc, RC::RmBooleanExprArgIsNotBoolean);
}

#[test]
fn test_bool_or_tf() {
    let left = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(true) };
    let right = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = bool_or(&left, &right, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_bool_or_ff() {
    let left = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let right = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(false) };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let rc = bool_or(&left, &right, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.v, ValueUnion::BoolV(false)));
}

#[test]
fn test_eval_expr_const_int() {
    let expr = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) }),
    };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let record = recordManager::tables::Record {
        id: recordManager::tables::RID { page: 0, slot: 0 },
        data: String::new(),
    };
    let schema = recordManager::tables::Schema {
        num_attr: 0, attr_names: vec![], data_types: vec![],
        type_length: vec![], key_attrs: vec![], key_size: 0,
    };
    let rc = eval_expr(&record, &schema, &expr, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.v, ValueUnion::IntV(10)));
}

#[test]
fn test_eval_expr_smaller() {
    let left = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) }),
    };
    let right = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(Value { dt: DataType::DtInt, v: ValueUnion::IntV(20) }),
    };
    let op = Expr {
        expr_type: ExprType::ExprOp,
        expr: ExprUnion::Op(Box::new(Operator {
            op_type: OpType::OpCompSmaller,
            args: vec![left, right],
        })),
    };
    let mut result = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
    let record = recordManager::tables::Record {
        id: recordManager::tables::RID { page: 0, slot: 0 },
        data: String::new(),
    };
    let schema = recordManager::tables::Schema {
        num_attr: 0, attr_names: vec![], data_types: vec![],
        type_length: vec![], key_attrs: vec![], key_size: 0,
    };
    let rc = eval_expr(&record, &schema, &op, &mut result);
    assert_eq!(rc, RC::Ok);
    assert!(matches!(result.dt, DataType::DtBool));
    assert!(matches!(result.v, ValueUnion::BoolV(true)));
}

#[test]
fn test_free_expr() {
    let mut expr = Expr {
        expr_type: ExprType::ExprConst,
        expr: ExprUnion::Cons(Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) }),
    };
    let rc = free_expr(&mut expr);
    assert_eq!(rc, RC::Ok);
}

#[test]
fn test_free_val() {
    let mut val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(10) };
    free_val(&mut val);
    // Just ensure it doesn't panic
}

fn main() {}

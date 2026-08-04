use crate::{dberror::RC, tables::{Value, Record, Schema}};
use crate::tables::{DataType, ValueUnion};
#[derive(Debug, Clone)]
pub enum ExprType {
ExprOp,
ExprConst,
ExprAttrRef,
}
#[derive(Debug, Clone)]
pub struct Expr {
pub expr_type: ExprType,
pub expr: ExprUnion,
}
#[derive(Debug, Clone)]
pub enum ExprUnion {
Cons(Value),
AttrRef(i32),
Op(Box<Operator>),
}
#[derive(Debug, Clone)]
pub struct Operator {
pub op_type: OpType,
pub args: Vec<Expr>,
}
#[derive(Debug, Clone)]
pub enum OpType {
OpBoolAnd,
OpBoolOr,
OpBoolNot,
OpCompEqual,
OpCompSmaller,
}
pub fn value_equals(left: &Value, right: &Value, result: &mut Value) -> RC {
    if std::mem::discriminant(&left.dt) != std::mem::discriminant(&right.dt) {
        return RC::RmCompareValueOfDifferentDatatype;
    }

    let equals = match (&left.v, &right.v) {
        (ValueUnion::IntV(l), ValueUnion::IntV(r)) => l == r,
        (ValueUnion::FloatV(l), ValueUnion::FloatV(r)) => l == r,
        (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) => l == r,
        (ValueUnion::StringV(l), ValueUnion::StringV(r)) => l == r,
        _ => return RC::RmCompareValueOfDifferentDatatype,
    };

    *result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(equals) };
    bool_rc(equals)
}
pub fn value_smaller(left: &Value, right: &Value, result: &mut Value) -> RC {
    if std::mem::discriminant(&left.dt) != std::mem::discriminant(&right.dt) {
        return RC::RmCompareValueOfDifferentDatatype;
    }

    let smaller = match (&left.v, &right.v) {
        (ValueUnion::IntV(l), ValueUnion::IntV(r)) => l < r,
        (ValueUnion::FloatV(l), ValueUnion::FloatV(r)) => l < r,
        (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) => (*l as u8) < (*r as u8),
        (ValueUnion::StringV(l), ValueUnion::StringV(r)) => l < r,
        _ => return RC::RmCompareValueOfDifferentDatatype,
    };

    *result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(smaller) };
    bool_rc(smaller)
}
pub fn bool_not(input: &Value, result: &mut Value) -> RC {
    if !matches!(input.dt, DataType::DtBool) {
        return RC::RmBooleanExprArgIsNotBoolean;
    }

    let value = match input.v {
        ValueUnion::BoolV(v) => !v,
        _ => return RC::RmBooleanExprArgIsNotBoolean,
    };
    *result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(value) };
    bool_rc(value)
}
pub fn bool_and(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !matches!(left.dt, DataType::DtBool) || !matches!(right.dt, DataType::DtBool) {
        return RC::RmBooleanExprArgIsNotBoolean;
    }

    let value = match (&left.v, &right.v) {
        (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) => *l && *r,
        _ => return RC::RmBooleanExprArgIsNotBoolean,
    };
    *result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(value) };
    bool_rc(value)
}
pub fn bool_or(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !matches!(left.dt, DataType::DtBool) || !matches!(right.dt, DataType::DtBool) {
        return RC::RmBooleanExprArgIsNotBoolean;
    }

    let value = match (&left.v, &right.v) {
        (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) => *l || *r,
        _ => return RC::RmBooleanExprArgIsNotBoolean,
    };
    *result = Value { dt: DataType::DtBool, v: ValueUnion::BoolV(value) };
    bool_rc(value)
}
pub fn eval_expr(record: &Record, schema: &Schema, expr: &Expr, result: &mut Value) -> RC {
    match &expr.expr {
        ExprUnion::Cons(value) => {
            *result = value.clone();
            RC::Ok
        }
        ExprUnion::AttrRef(attr_num) => crate::record_mgr::get_attr(record, schema, *attr_num, result),
        ExprUnion::Op(op) => {
            let mut left = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
            let left_rc = eval_expr(record, schema, &op.args[0], &mut left);
            if left_rc != RC::Ok {
                return left_rc;
            }

            let op_rc = match op.op_type {
                OpType::OpBoolNot => bool_not(&left, result),
                OpType::OpBoolAnd | OpType::OpBoolOr | OpType::OpCompEqual | OpType::OpCompSmaller => {
                    let mut right = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
                    let right_rc = eval_expr(record, schema, &op.args[1], &mut right);
                    if right_rc != RC::Ok {
                        return right_rc;
                    }

                    match op.op_type {
                        OpType::OpBoolAnd => bool_and(&left, &right, result),
                        OpType::OpBoolOr => bool_or(&left, &right, result),
                        OpType::OpCompEqual => value_equals(&left, &right, result),
                        OpType::OpCompSmaller => value_smaller(&left, &right, result),
                        OpType::OpBoolNot => unreachable!(),
                    }
                }
            };

            match op_rc {
                RC::Error => RC::Ok,
                other => other,
            }
        }
    }
}
pub fn free_expr(expr: &mut Expr) -> RC {
    match &mut expr.expr {
        ExprUnion::Cons(value) => free_val(value),
        ExprUnion::AttrRef(_) => {}
        ExprUnion::Op(op) => {
            for arg in &mut op.args {
                let _ = free_expr(arg);
            }
            op.args.clear();
        }
    }
    RC::Ok
}
pub fn free_val(val: &mut Value) {
    if matches!(val.dt, DataType::DtString) {
        val.v = ValueUnion::StringV(String::new());
    }
}

fn bool_rc(value: bool) -> RC {
    if value {
        RC::Ok
    } else {
        RC::Error
    }
}

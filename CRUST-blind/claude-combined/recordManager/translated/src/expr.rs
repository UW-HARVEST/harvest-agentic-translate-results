use crate::{dberror::RC, tables::{Value, Record, Schema, DataType, ValueUnion}};

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

fn discriminant(dt: &DataType) -> i32 {
    match dt {
        DataType::DtInt => 0,
        DataType::DtString => 1,
        DataType::DtFloat => 2,
        DataType::DtBool => 3,
    }
}

pub fn value_equals(left: &Value, right: &Value, result: &mut Value) -> RC {
    if discriminant(&left.dt) != discriminant(&right.dt) {
        return RC::RmCompareValueOfDifferentDatatype;
    }
    result.dt = DataType::DtBool;
    let r = match (&left.v, &right.v) {
        (ValueUnion::IntV(a), ValueUnion::IntV(b)) => a == b,
        (ValueUnion::FloatV(a), ValueUnion::FloatV(b)) => a == b,
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => a == b,
        (ValueUnion::StringV(a), ValueUnion::StringV(b)) => a == b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(r);
    RC::Ok
}

pub fn value_smaller(left: &Value, right: &Value, result: &mut Value) -> RC {
    if discriminant(&left.dt) != discriminant(&right.dt) {
        return RC::RmCompareValueOfDifferentDatatype;
    }
    result.dt = DataType::DtBool;
    let r = match (&left.v, &right.v) {
        (ValueUnion::IntV(a), ValueUnion::IntV(b)) => a < b,
        (ValueUnion::FloatV(a), ValueUnion::FloatV(b)) => a < b,
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => !a & b,
        (ValueUnion::StringV(a), ValueUnion::StringV(b)) => a < b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(r);
    RC::Ok
}

pub fn bool_not(input: &Value, result: &mut Value) -> RC {
    if !matches!(input.dt, DataType::DtBool) {
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    result.dt = DataType::DtBool;
    let v = match input.v {
        ValueUnion::BoolV(b) => !b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(v);
    RC::Ok
}

pub fn bool_and(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !matches!(left.dt, DataType::DtBool) || !matches!(right.dt, DataType::DtBool) {
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    let v = match (&left.v, &right.v) {
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => *a && *b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(v);
    RC::Ok
}

pub fn bool_or(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !matches!(left.dt, DataType::DtBool) || !matches!(right.dt, DataType::DtBool) {
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    let v = match (&left.v, &right.v) {
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => *a || *b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(v);
    RC::Ok
}

pub fn eval_expr(record: &Record, schema: &Schema, expr: &Expr, result: &mut Value) -> RC {
    match &expr.expr {
        ExprUnion::Op(op) => {
            let two_args = !matches!(op.op_type, OpType::OpBoolNot);
            let mut l_in = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
            let rc_l = eval_expr(record, schema, &op.args[0], &mut l_in);
            if rc_l != RC::Ok {
                return rc_l;
            }
            let mut r_in = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
            if two_args {
                let rc_r = eval_expr(record, schema, &op.args[1], &mut r_in);
                if rc_r != RC::Ok {
                    return rc_r;
                }
            }
            match op.op_type {
                OpType::OpBoolNot => bool_not(&l_in, result),
                OpType::OpBoolAnd => bool_and(&l_in, &r_in, result),
                OpType::OpBoolOr => bool_or(&l_in, &r_in, result),
                OpType::OpCompEqual => value_equals(&l_in, &r_in, result),
                OpType::OpCompSmaller => value_smaller(&l_in, &r_in, result),
            }
        }
        ExprUnion::Cons(v) => {
            *result = v.clone();
            RC::Ok
        }
        ExprUnion::AttrRef(idx) => {
            crate::record_mgr::get_attr(record, schema, *idx, result)
        }
    }
}

pub fn free_expr(_expr: &mut Expr) -> RC {
    // No-op in Rust: memory will be released on drop.
    RC::Ok
}

pub fn free_val(_val: &mut Value) {
    // No-op in Rust: memory will be released on drop.
}

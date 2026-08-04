use crate::{dberror::RC, tables::{Value, ValueUnion, Record, Schema, DataType}};
use crate::record_mgr::get_attr;

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

fn datatype_index(dt: &DataType) -> i32 {
    match dt {
        DataType::DtInt => 0,
        DataType::DtString => 1,
        DataType::DtFloat => 2,
        DataType::DtBool => 3,
    }
}

pub fn value_equals(left: &Value, right: &Value, result: &mut Value) -> RC {
    if datatype_index(&left.dt) != datatype_index(&right.dt) {
        return RC::RmCompareValueOfDifferentDatatype;
    }
    result.dt = DataType::DtBool;
    let eq = match (&left.v, &right.v) {
        (ValueUnion::IntV(a), ValueUnion::IntV(b)) => a == b,
        (ValueUnion::FloatV(a), ValueUnion::FloatV(b)) => a == b,
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => a == b,
        (ValueUnion::StringV(a), ValueUnion::StringV(b)) => a == b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(eq);
    if eq { RC::Ok } else { RC::Error }
}

pub fn value_smaller(left: &Value, right: &Value, result: &mut Value) -> RC {
    if datatype_index(&left.dt) != datatype_index(&right.dt) {
        return RC::RmCompareValueOfDifferentDatatype;
    }
    result.dt = DataType::DtBool;
    let smaller = match (&left.v, &right.v) {
        (ValueUnion::IntV(a), ValueUnion::IntV(b)) => a < b,
        (ValueUnion::FloatV(a), ValueUnion::FloatV(b)) => a < b,
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => !*a & *b,
        (ValueUnion::StringV(a), ValueUnion::StringV(b)) => a < b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(smaller);
    if smaller { RC::Ok } else { RC::Error }
}

pub fn bool_not(input: &Value, result: &mut Value) -> RC {
    if datatype_index(&input.dt) != datatype_index(&DataType::DtBool) {
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    result.dt = DataType::DtBool;
    let val = match &input.v {
        ValueUnion::BoolV(b) => !*b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(val);
    if val { RC::Ok } else { RC::Error }
}

pub fn bool_and(left: &Value, right: &Value, result: &mut Value) -> RC {
    if datatype_index(&left.dt) != datatype_index(&DataType::DtBool)
        || datatype_index(&right.dt) != datatype_index(&DataType::DtBool)
    {
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    let val = match (&left.v, &right.v) {
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => *a && *b,
        _ => false,
    };
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(val);
    if val { RC::Ok } else { RC::Error }
}

pub fn bool_or(left: &Value, right: &Value, result: &mut Value) -> RC {
    if datatype_index(&left.dt) != datatype_index(&DataType::DtBool)
        || datatype_index(&right.dt) != datatype_index(&DataType::DtBool)
    {
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    let val = match (&left.v, &right.v) {
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => *a || *b,
        _ => false,
    };
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(val);
    if val { RC::Ok } else { RC::Error }
}

pub fn eval_expr(record: &Record, schema: &Schema, expr: &Expr, result: &mut Value) -> RC {
    match &expr.expr {
        ExprUnion::Op(op_box) => {
            let op = op_box.as_ref();
            let two_args = !matches!(op.op_type, OpType::OpBoolNot);
            let mut l_in = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
            let mut r_in = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
            let rc = eval_expr(record, schema, &op.args[0], &mut l_in);
            // For OpCompSmaller etc the eval_expr return doesn't strictly need to be Ok
            // (Const evaluations always return Ok)
            if !matches!(rc, RC::Ok) && matches!(op.args[0].expr_type, ExprType::ExprAttrRef) {
                return rc;
            }
            if two_args {
                let rc2 = eval_expr(record, schema, &op.args[1], &mut r_in);
                if !matches!(rc2, RC::Ok) && matches!(op.args[1].expr_type, ExprType::ExprAttrRef) {
                    return rc2;
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
        ExprUnion::AttrRef(attr) => {
            get_attr(record, schema, *attr, result)
        }
    }
}

pub fn free_expr(_expr: &mut Expr) -> RC {
    // No-op in Rust; values are dropped automatically.
    RC::Ok
}

pub fn free_val(_val: &mut Value) {
    // No-op in Rust; values are dropped automatically.
}

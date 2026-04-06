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

fn values_are_same_type(left: &Value, right: &Value) -> bool {
    std::mem::discriminant(&left.dt) == std::mem::discriminant(&right.dt)
}

pub fn value_equals(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !values_are_same_type(left, right) {
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
    if eq { RC::Ok } else { RC::RmCompareValueOfDifferentDatatype }
}

pub fn value_smaller(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !values_are_same_type(left, right) {
        return RC::RmCompareValueOfDifferentDatatype;
    }
    result.dt = DataType::DtBool;
    let lt = match (&left.v, &right.v) {
        (ValueUnion::IntV(a), ValueUnion::IntV(b)) => a < b,
        (ValueUnion::FloatV(a), ValueUnion::FloatV(b)) => a < b,
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => (*a as i32) < (*b as i32),
        (ValueUnion::StringV(a), ValueUnion::StringV(b)) => a < b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(lt);
    if lt { RC::Ok } else { RC::RmCompareValueOfDifferentDatatype }
}

pub fn bool_not(input: &Value, result: &mut Value) -> RC {
    match &input.v {
        ValueUnion::BoolV(b) => {
            let r = !b;
            result.dt = DataType::DtBool;
            result.v = ValueUnion::BoolV(r);
            if r { RC::Ok } else { RC::RmBooleanExprArgIsNotBoolean }
        }
        _ => RC::RmBooleanExprArgIsNotBoolean,
    }
}

pub fn bool_and(left: &Value, right: &Value, result: &mut Value) -> RC {
    let (lb, rb) = match (&left.v, &right.v) {
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => (*a, *b),
        _ => return RC::RmBooleanExprArgIsNotBoolean,
    };
    let r = lb && rb;
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(r);
    if r { RC::Ok } else { RC::RmBooleanExprArgIsNotBoolean }
}

pub fn bool_or(left: &Value, right: &Value, result: &mut Value) -> RC {
    let (lb, rb) = match (&left.v, &right.v) {
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => (*a, *b),
        _ => return RC::RmBooleanExprArgIsNotBoolean,
    };
    let r = lb || rb;
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(r);
    if r { RC::Ok } else { RC::RmBooleanExprArgIsNotBoolean }
}

fn cpval(result: &mut Value, input: &Value) {
    result.dt = input.dt.clone();
    result.v = input.v.clone();
}

pub fn eval_expr(record: &Record, schema: &Schema, expr: &Expr, result: &mut Value) -> RC {
    match &expr.expr_type {
        ExprType::ExprOp => {
            let op = match &expr.expr {
                ExprUnion::Op(o) => o,
                _ => return RC::Error,
            };
            let two_args = !matches!(op.op_type, OpType::OpBoolNot);
            let mut l_in = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
            eval_expr(record, schema, &op.args[0], &mut l_in);
            let mut r_in = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
            if two_args {
                eval_expr(record, schema, &op.args[1], &mut r_in);
            }
            // Set result to default
            result.dt = DataType::DtBool;
            result.v = ValueUnion::BoolV(false);
            match op.op_type {
                OpType::OpBoolNot => { bool_not(&l_in, result); },
                OpType::OpBoolAnd => { bool_and(&l_in, &r_in, result); },
                OpType::OpBoolOr => { bool_or(&l_in, &r_in, result); },
                OpType::OpCompEqual => { value_equals(&l_in, &r_in, result); },
                OpType::OpCompSmaller => { value_smaller(&l_in, &r_in, result); },
            }
        }
        ExprType::ExprConst => {
            let cons = match &expr.expr {
                ExprUnion::Cons(c) => c,
                _ => return RC::Error,
            };
            cpval(result, cons);
        }
        ExprType::ExprAttrRef => {
            let attr_ref = match &expr.expr {
                ExprUnion::AttrRef(a) => *a,
                _ => return RC::Error,
            };
            let mut val = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
            crate::record_mgr::get_attr(record, schema, attr_ref, &mut val);
            cpval(result, &val);
        }
    }
    RC::Ok
}

pub fn free_expr(_expr: &mut Expr) -> RC {
    // In Rust, memory is managed automatically
    RC::Ok
}

pub fn free_val(_val: &mut Value) {
    // In Rust, memory is managed automatically
}

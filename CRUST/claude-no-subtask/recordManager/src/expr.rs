use crate::{dberror::{RC, throw}, tables::{Value, ValueUnion, DataType, Record, Schema}};

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
    if left.dt != right.dt {
        throw(
            RC::RmCompareValueOfDifferentDatatype,
            "equality comparison only supported for values of the same datatype",
        );
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
    if left.dt != right.dt {
        throw(
            RC::RmCompareValueOfDifferentDatatype,
            "equality comparison only supported for values of the same datatype",
        );
        return RC::RmCompareValueOfDifferentDatatype;
    }
    result.dt = DataType::DtBool;
    let lt = match (&left.v, &right.v) {
        (ValueUnion::IntV(a), ValueUnion::IntV(b)) => a < b,
        (ValueUnion::FloatV(a), ValueUnion::FloatV(b)) => a < b,
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => !a & b,
        (ValueUnion::StringV(a), ValueUnion::StringV(b)) => a < b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(lt);
    if lt { RC::Ok } else { RC::Error }
}

pub fn bool_not(input: &Value, result: &mut Value) -> RC {
    if input.dt != DataType::DtBool {
        throw(
            RC::RmBooleanExprArgIsNotBoolean,
            "boolean NOT requires boolean input",
        );
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    result.dt = DataType::DtBool;
    let val = match &input.v {
        ValueUnion::BoolV(b) => !b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(val);
    RC::Ok
}

pub fn bool_and(left: &Value, right: &Value, result: &mut Value) -> RC {
    if left.dt != DataType::DtBool || right.dt != DataType::DtBool {
        throw(
            RC::RmBooleanExprArgIsNotBoolean,
            "boolean AND requires boolean inputs",
        );
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    result.dt = DataType::DtBool;
    let val = match (&left.v, &right.v) {
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => *a && *b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(val);
    if val { RC::Ok } else { RC::Error }
}

pub fn bool_or(left: &Value, right: &Value, result: &mut Value) -> RC {
    if left.dt != DataType::DtBool || right.dt != DataType::DtBool {
        throw(
            RC::RmBooleanExprArgIsNotBoolean,
            "boolean OR requires boolean inputs",
        );
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    result.dt = DataType::DtBool;
    let val = match (&left.v, &right.v) {
        (ValueUnion::BoolV(a), ValueUnion::BoolV(b)) => *a || *b,
        _ => false,
    };
    result.v = ValueUnion::BoolV(val);
    if val { RC::Ok } else { RC::Error }
}

pub fn eval_expr(record: &Record, schema: &Schema, expr: &Expr, result: &mut Value) -> RC {
    match &expr.expr {
        ExprUnion::Op(op) => {
            let two_args = !matches!(op.op_type, OpType::OpBoolNot);
            let mut l_in = Value {
                dt: DataType::DtInt,
                v: ValueUnion::IntV(-1),
            };
            let mut r_in = Value {
                dt: DataType::DtInt,
                v: ValueUnion::IntV(-1),
            };
            let rc = eval_expr(record, schema, &op.args[0], &mut l_in);
            if rc != RC::Ok {
                return rc;
            }
            if two_args {
                let rc = eval_expr(record, schema, &op.args[1], &mut r_in);
                if rc != RC::Ok {
                    return rc;
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
            result.dt = v.dt.clone();
            result.v = v.v.clone();
            RC::Ok
        }
        ExprUnion::AttrRef(attr) => {
            crate::record_mgr::get_attr(record, schema, *attr, result)
        }
    }
}

pub fn free_expr(_expr: &mut Expr) -> RC {
    // Rust handles memory automatically.
    RC::Ok
}

pub fn free_val(_val: &mut Value) {
    // Rust handles memory automatically.
}

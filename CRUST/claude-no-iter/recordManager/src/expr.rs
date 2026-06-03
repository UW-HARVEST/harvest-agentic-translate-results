use crate::{
    dberror::{throw, RC},
    record_mgr,
    tables::{DataType, Record, Schema, Value, ValueUnion},
};
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

fn dt_eq(a: &DataType, b: &DataType) -> bool {
    matches!(
        (a, b),
        (DataType::DtInt, DataType::DtInt)
            | (DataType::DtFloat, DataType::DtFloat)
            | (DataType::DtBool, DataType::DtBool)
            | (DataType::DtString, DataType::DtString)
    )
}

fn rc_from_code(code: i32) -> RC {
    match code {
        0 => RC::Ok,
        200 => RC::RmCompareValueOfDifferentDatatype,
        201 => RC::RmExprResultIsNotBoolean,
        202 => RC::RmBooleanExprArgIsNotBoolean,
        _ => RC::Error,
    }
}

pub fn value_equals(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !dt_eq(&left.dt, &right.dt) {
        let code = throw(
            RC::RmCompareValueOfDifferentDatatype,
            "equality comparison only supported for values of the same datatype",
        );
        return rc_from_code(code);
    }
    result.dt = DataType::DtBool;
    let b = match (&left.v, &right.v) {
        (ValueUnion::IntV(l), ValueUnion::IntV(r)) => l == r,
        (ValueUnion::FloatV(l), ValueUnion::FloatV(r)) => l == r,
        (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) => l == r,
        (ValueUnion::StringV(l), ValueUnion::StringV(r)) => l == r,
        _ => false,
    };
    result.v = ValueUnion::BoolV(b);
    if b {
        RC::Ok
    } else {
        RC::Error
    }
}

pub fn value_smaller(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !dt_eq(&left.dt, &right.dt) {
        let code = throw(
            RC::RmCompareValueOfDifferentDatatype,
            "equality comparison only supported for values of the same datatype",
        );
        return rc_from_code(code);
    }
    result.dt = DataType::DtBool;
    let b = match (&left.v, &right.v) {
        (ValueUnion::IntV(l), ValueUnion::IntV(r)) => l < r,
        (ValueUnion::FloatV(l), ValueUnion::FloatV(r)) => l < r,
        (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) => !l & r,
        (ValueUnion::StringV(l), ValueUnion::StringV(r)) => l < r,
        _ => false,
    };
    result.v = ValueUnion::BoolV(b);
    if b {
        RC::Ok
    } else {
        RC::Error
    }
}

pub fn bool_not(input: &Value, result: &mut Value) -> RC {
    if !matches!(input.dt, DataType::DtBool) {
        let code = throw(
            RC::RmBooleanExprArgIsNotBoolean,
            "boolean NOT requires boolean input",
        );
        return rc_from_code(code);
    }
    result.dt = DataType::DtBool;
    if let ValueUnion::BoolV(b) = input.v {
        result.v = ValueUnion::BoolV(!b);
    }
    RC::Ok
}

pub fn bool_and(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !matches!(left.dt, DataType::DtBool) || !matches!(right.dt, DataType::DtBool) {
        let code = throw(
            RC::RmBooleanExprArgIsNotBoolean,
            "boolean AND requires boolean inputs",
        );
        return rc_from_code(code);
    }
    let l = match left.v {
        ValueUnion::BoolV(b) => b,
        _ => false,
    };
    let r = match right.v {
        ValueUnion::BoolV(b) => b,
        _ => false,
    };
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(l && r);
    if l && r {
        RC::Ok
    } else {
        RC::Error
    }
}

pub fn bool_or(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !matches!(left.dt, DataType::DtBool) || !matches!(right.dt, DataType::DtBool) {
        let code = throw(
            RC::RmBooleanExprArgIsNotBoolean,
            "boolean OR requires boolean inputs",
        );
        return rc_from_code(code);
    }
    let l = match left.v {
        ValueUnion::BoolV(b) => b,
        _ => false,
    };
    let r = match right.v {
        ValueUnion::BoolV(b) => b,
        _ => false,
    };
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(l || r);
    if l || r {
        RC::Ok
    } else {
        RC::Error
    }
}

pub fn eval_expr(record: &Record, schema: &Schema, expr: &Expr, result: &mut Value) -> RC {
    // Initialize result to default int -1 like the C MAKE_VALUE
    result.dt = DataType::DtInt;
    result.v = ValueUnion::IntV(-1);

    match &expr.expr {
        ExprUnion::Op(op) => {
            let two_args = !matches!(op.op_type, OpType::OpBoolNot);
            let mut l_in = Value {
                dt: DataType::DtInt,
                v: ValueUnion::IntV(0),
            };
            let mut r_in = Value {
                dt: DataType::DtInt,
                v: ValueUnion::IntV(0),
            };
            let rc = eval_expr(record, schema, &op.args[0], &mut l_in);
            if rc != RC::Ok {
                return rc;
            }
            if two_args {
                let rc2 = eval_expr(record, schema, &op.args[1], &mut r_in);
                if rc2 != RC::Ok {
                    return rc2;
                }
            }
            match op.op_type {
                OpType::OpBoolNot => {
                    let rc = bool_not(&l_in, result);
                    if rc != RC::Ok {
                        return rc;
                    }
                }
                OpType::OpBoolAnd => {
                    bool_and(&l_in, &r_in, result);
                }
                OpType::OpBoolOr => {
                    bool_or(&l_in, &r_in, result);
                }
                OpType::OpCompEqual => {
                    let rc = value_equals(&l_in, &r_in, result);
                    if rc != RC::Ok {
                        return rc;
                    }
                }
                OpType::OpCompSmaller => {
                    let rc = value_smaller(&l_in, &r_in, result);
                    if rc != RC::Ok {
                        return rc;
                    }
                }
            }
            RC::Ok
        }
        ExprUnion::Cons(val) => {
            result.dt = val.dt.clone();
            result.v = val.v.clone();
            RC::Ok
        }
        ExprUnion::AttrRef(attr_ref) => record_mgr::get_attr(record, schema, *attr_ref, result),
    }
}

pub fn free_expr(_expr: &mut Expr) -> RC {
    // Rust handles memory cleanup automatically
    RC::Ok
}

pub fn free_val(_val: &mut Value) {
    // Rust handles memory cleanup automatically
}

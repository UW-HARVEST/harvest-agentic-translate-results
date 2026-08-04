use crate::dberror::{throw, RC};
use crate::tables::{
    DataType, Record, Schema, Value, ValueUnion,
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

pub fn value_equals(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !dt_eq(&left.dt, &right.dt) {
        let _ = throw(
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
    RC::Ok
}

pub fn value_smaller(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !dt_eq(&left.dt, &right.dt) {
        let _ = throw(
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
    RC::Ok
}

pub fn bool_not(input: &Value, result: &mut Value) -> RC {
    if !matches!(input.dt, DataType::DtBool) {
        let _ = throw(
            RC::RmBooleanExprArgIsNotBoolean,
            "boolean NOT requires boolean input",
        );
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    result.dt = DataType::DtBool;
    let v = if let ValueUnion::BoolV(b) = input.v { !b } else { false };
    result.v = ValueUnion::BoolV(v);
    RC::Ok
}

pub fn bool_and(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !matches!(left.dt, DataType::DtBool) || !matches!(right.dt, DataType::DtBool) {
        let _ = throw(
            RC::RmBooleanExprArgIsNotBoolean,
            "boolean AND requires boolean inputs",
        );
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    let l = matches!(left.v, ValueUnion::BoolV(true));
    let r = matches!(right.v, ValueUnion::BoolV(true));
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(l && r);
    RC::Ok
}

pub fn bool_or(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !matches!(left.dt, DataType::DtBool) || !matches!(right.dt, DataType::DtBool) {
        let _ = throw(
            RC::RmBooleanExprArgIsNotBoolean,
            "boolean OR requires boolean inputs",
        );
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    let l = matches!(left.v, ValueUnion::BoolV(true));
    let r = matches!(right.v, ValueUnion::BoolV(true));
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(l || r);
    RC::Ok
}

pub fn eval_expr(record: &Record, schema: &Schema, expr: &Expr, result: &mut Value) -> RC {
    match &expr.expr {
        ExprUnion::Cons(v) => {
            *result = v.clone();
            RC::Ok
        }
        ExprUnion::AttrRef(attr) => {
            let mut tmp = Value {
                dt: DataType::DtInt,
                v: ValueUnion::IntV(0),
            };
            let rc = crate::record_mgr::get_attr(record, schema, *attr, &mut tmp);
            if rc != RC::Ok {
                return rc;
            }
            *result = tmp;
            RC::Ok
        }
        ExprUnion::Op(op) => {
            let two_args = !matches!(op.op_type, OpType::OpBoolNot);
            let mut l_in = Value {
                dt: DataType::DtInt,
                v: ValueUnion::IntV(-1),
            };
            let rc = eval_expr(record, schema, &op.args[0], &mut l_in);
            if rc != RC::Ok {
                return rc;
            }
            let mut r_in = Value {
                dt: DataType::DtInt,
                v: ValueUnion::IntV(-1),
            };
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
    }
}

pub fn free_expr(_expr: &mut Expr) -> RC {
    // Rust handles deallocation via Drop.
    RC::Ok
}

pub fn free_val(_val: &mut Value) {
    // Rust handles deallocation via Drop.
}

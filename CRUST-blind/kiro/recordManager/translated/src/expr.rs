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

pub fn value_equals(left: &Value, right: &Value, result: &mut Value) -> RC {
    if std::mem::discriminant(&left.dt) != std::mem::discriminant(&right.dt) {
        return RC::RmCompareValueOfDifferentDatatype;
    }
    result.dt = DataType::DtBool;
    let b = match (&left.v, &right.v) {
        (ValueUnion::IntV(l), ValueUnion::IntV(r)) => *l == *r,
        (ValueUnion::FloatV(l), ValueUnion::FloatV(r)) => *l == *r,
        (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) => *l == *r,
        (ValueUnion::StringV(l), ValueUnion::StringV(r)) => l == r,
        _ => false,
    };
    result.v = ValueUnion::BoolV(b);
    RC::Ok
}

pub fn value_smaller(left: &Value, right: &Value, result: &mut Value) -> RC {
    if std::mem::discriminant(&left.dt) != std::mem::discriminant(&right.dt) {
        return RC::RmCompareValueOfDifferentDatatype;
    }
    result.dt = DataType::DtBool;
    let b = match (&left.v, &right.v) {
        (ValueUnion::IntV(l), ValueUnion::IntV(r)) => *l < *r,
        (ValueUnion::FloatV(l), ValueUnion::FloatV(r)) => *l < *r,
        // C code has fall-through from DT_BOOL to DT_STRING (no break)
        (ValueUnion::BoolV(_l), ValueUnion::BoolV(_r)) => {
            // Fall through to string comparison in C, but since these are bool values
            // the string comparison won't apply. The C code sets boolV from bool < bool
            // then immediately falls through to strcmp which overwrites it.
            // Actually in C: case DT_BOOL sets result then falls through to DT_STRING.
            // DT_STRING does strcmp on stringV pointers which are garbage for bool values.
            // We'll just do the bool comparison since the string case would be UB.
            false // The C fall-through means DT_STRING case runs, comparing string pointers
        }
        (ValueUnion::StringV(l), ValueUnion::StringV(r)) => l < r,
        _ => false,
    };
    result.v = ValueUnion::BoolV(b);
    RC::Ok
}

pub fn bool_not(input: &Value, result: &mut Value) -> RC {
    match &input.v {
        ValueUnion::BoolV(b) => {
            result.dt = DataType::DtBool;
            result.v = ValueUnion::BoolV(!b);
            RC::Ok
        }
        _ => RC::RmBooleanExprArgIsNotBoolean,
    }
}

pub fn bool_and(left: &Value, right: &Value, result: &mut Value) -> RC {
    match (&left.v, &right.v) {
        (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) => {
            result.v = ValueUnion::BoolV(*l && *r);
            RC::Ok
        }
        _ => RC::RmBooleanExprArgIsNotBoolean,
    }
}

pub fn bool_or(left: &Value, right: &Value, result: &mut Value) -> RC {
    match (&left.v, &right.v) {
        (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) => {
            result.v = ValueUnion::BoolV(*l || *r);
            RC::Ok
        }
        _ => RC::RmBooleanExprArgIsNotBoolean,
    }
}

pub fn eval_expr(record: &Record, schema: &Schema, expr: &Expr, result: &mut Value) -> RC {
    // Initialize result like C: MAKE_VALUE(*result, DT_INT, -1)
    result.dt = DataType::DtInt;
    result.v = ValueUnion::IntV(-1);

    match &expr.expr {
        ExprUnion::Op(op) => {
            let two_args = !matches!(op.op_type, OpType::OpBoolNot);
            let mut l_in = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
            let rc = eval_expr(record, schema, &op.args[0], &mut l_in);
            if rc != RC::Ok { return rc; }

            let mut r_in = Value { dt: DataType::DtInt, v: ValueUnion::IntV(-1) };
            if two_args {
                let rc = eval_expr(record, schema, &op.args[1], &mut r_in);
                if rc != RC::Ok { return rc; }
            }

            let rc = match op.op_type {
                OpType::OpBoolNot => bool_not(&l_in, result),
                OpType::OpBoolAnd => bool_and(&l_in, &r_in, result),
                OpType::OpBoolOr => bool_or(&l_in, &r_in, result),
                OpType::OpCompEqual => value_equals(&l_in, &r_in, result),
                OpType::OpCompSmaller => value_smaller(&l_in, &r_in, result),
            };
            if rc != RC::Ok { return rc; }
        }
        ExprUnion::Cons(val) => {
            // CPVAL: copy value
            result.dt = val.dt.clone();
            result.v = val.v.clone();
        }
        ExprUnion::AttrRef(attr_ref) => {
            let rc = crate::record_mgr::get_attr(record, schema, *attr_ref, result);
            if rc != RC::Ok { return rc; }
        }
    }
    RC::Ok
}

pub fn free_expr(_expr: &mut Expr) -> RC {
    // In Rust, memory is managed automatically. Nothing to do.
    RC::Ok
}

pub fn free_val(_val: &mut Value) {
    // In Rust, memory is managed automatically. Nothing to do.
}

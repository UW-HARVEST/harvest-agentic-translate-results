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

fn get_bool(v: &Value) -> bool {
    match &v.v { ValueUnion::BoolV(b) => *b, _ => false }
}

fn get_int(v: &Value) -> i32 {
    match &v.v { ValueUnion::IntV(i) => *i, _ => 0 }
}

fn get_float(v: &Value) -> f32 {
    match &v.v { ValueUnion::FloatV(f) => *f, _ => 0.0 }
}

fn get_string(v: &Value) -> &str {
    match &v.v { ValueUnion::StringV(s) => s.as_str(), _ => "" }
}

fn dt_matches(a: &DataType, b: &DataType) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

pub fn value_equals(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !dt_matches(&left.dt, &right.dt) {
        return RC::RmCompareValueOfDifferentDatatype;
    }
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(match &left.dt {
        DataType::DtInt => get_int(left) == get_int(right),
        DataType::DtFloat => get_float(left) == get_float(right),
        DataType::DtBool => get_bool(left) == get_bool(right),
        DataType::DtString => get_string(left) == get_string(right),
    });
    RC::Ok
}

pub fn value_smaller(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !dt_matches(&left.dt, &right.dt) {
        return RC::RmCompareValueOfDifferentDatatype;
    }
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(match &left.dt {
        DataType::DtInt => get_int(left) < get_int(right),
        DataType::DtFloat => get_float(left) < get_float(right),
        DataType::DtBool => (get_bool(left) as i32) < (get_bool(right) as i32),
        DataType::DtString => get_string(left) < get_string(right),
    });
    RC::Ok
}

pub fn bool_not(input: &Value, result: &mut Value) -> RC {
    if !matches!(input.dt, DataType::DtBool) {
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(!get_bool(input));
    RC::Ok
}

pub fn bool_and(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !matches!(left.dt, DataType::DtBool) || !matches!(right.dt, DataType::DtBool) {
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    result.v = ValueUnion::BoolV(get_bool(left) && get_bool(right));
    RC::Ok
}

pub fn bool_or(left: &Value, right: &Value, result: &mut Value) -> RC {
    if !matches!(left.dt, DataType::DtBool) || !matches!(right.dt, DataType::DtBool) {
        return RC::RmBooleanExprArgIsNotBoolean;
    }
    result.v = ValueUnion::BoolV(get_bool(left) || get_bool(right));
    RC::Ok
}

fn cpval(result: &mut Value, input: &Value) {
    result.dt = input.dt.clone();
    result.v = input.v.clone();
}

pub fn eval_expr(record: &Record, schema: &Schema, expr: &Expr, result: &mut Value) -> RC {
    result.dt = DataType::DtInt;
    result.v = ValueUnion::IntV(-1);

    match &expr.expr_type {
        ExprType::ExprOp => {
            let op = match &expr.expr {
                ExprUnion::Op(o) => o,
                _ => return RC::Error,
            };
            let two_args = !matches!(op.op_type, OpType::OpBoolNot);

            let mut l_in = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
            eval_expr(record, schema, &op.args[0], &mut l_in);

            let mut r_in = Value { dt: DataType::DtInt, v: ValueUnion::IntV(0) };
            if two_args {
                eval_expr(record, schema, &op.args[1], &mut r_in);
            }

            match op.op_type {
                OpType::OpBoolNot => { bool_not(&l_in, result); }
                OpType::OpBoolAnd => { bool_and(&l_in, &r_in, result); }
                OpType::OpBoolOr => { bool_or(&l_in, &r_in, result); }
                OpType::OpCompEqual => { value_equals(&l_in, &r_in, result); }
                OpType::OpCompSmaller => { value_smaller(&l_in, &r_in, result); }
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
            crate::record_mgr::get_attr(record, schema, attr_ref, result);
        }
    }
    RC::Ok
}

pub fn free_expr(expr: &mut Expr) -> RC {
    // In Rust, memory is managed automatically. This is a no-op.
    RC::Ok
}

pub fn free_val(val: &mut Value) {
    // In Rust, memory is managed automatically. This is a no-op.
}

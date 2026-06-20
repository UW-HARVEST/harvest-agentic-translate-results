use crate::{
    dberror::RC,
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

pub fn value_equals(left: &Value, right: &Value, result: &mut Value) -> RC {
    if std::mem::discriminant(&left.dt) != std::mem::discriminant(&right.dt) {
        return RC::RmCompareValueOfDifferentDatatype;
    }

    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(match (&left.v, &right.v) {
        (ValueUnion::IntV(l), ValueUnion::IntV(r)) => l == r,
        (ValueUnion::FloatV(l), ValueUnion::FloatV(r)) => l == r,
        (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) => l == r,
        (ValueUnion::StringV(l), ValueUnion::StringV(r)) => l == r,
        _ => false,
    });
    RC::Ok
}

pub fn value_smaller(left: &Value, right: &Value, result: &mut Value) -> RC {
    if std::mem::discriminant(&left.dt) != std::mem::discriminant(&right.dt) {
        return RC::RmCompareValueOfDifferentDatatype;
    }

    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(match (&left.v, &right.v) {
        (ValueUnion::IntV(l), ValueUnion::IntV(r)) => l < r,
        (ValueUnion::FloatV(l), ValueUnion::FloatV(r)) => l < r,
        (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) => (*l as i32) < (*r as i32),
        (ValueUnion::StringV(l), ValueUnion::StringV(r)) => l < r,
        _ => false,
    });
    RC::Ok
}

pub fn bool_not(input: &Value, result: &mut Value) -> RC {
    let ValueUnion::BoolV(value) = input.v else {
        return RC::RmBooleanExprArgIsNotBoolean;
    };
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(!value);
    RC::Ok
}

pub fn bool_and(left: &Value, right: &Value, result: &mut Value) -> RC {
    let (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) = (&left.v, &right.v) else {
        return RC::RmBooleanExprArgIsNotBoolean;
    };
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(*l && *r);
    RC::Ok
}

pub fn bool_or(left: &Value, right: &Value, result: &mut Value) -> RC {
    let (ValueUnion::BoolV(l), ValueUnion::BoolV(r)) = (&left.v, &right.v) else {
        return RC::RmBooleanExprArgIsNotBoolean;
    };
    result.dt = DataType::DtBool;
    result.v = ValueUnion::BoolV(*l || *r);
    RC::Ok
}

pub fn eval_expr(record: &Record, schema: &Schema, expr: &Expr, result: &mut Value) -> RC {
    match &expr.expr {
        ExprUnion::Op(op) => {
            let mut left = Value {
                dt: DataType::DtInt,
                v: ValueUnion::IntV(-1),
            };
            let rc = eval_expr(record, schema, &op.args[0], &mut left);
            if rc != RC::Ok {
                return rc;
            }

            let mut right = Value {
                dt: DataType::DtInt,
                v: ValueUnion::IntV(-1),
            };
            if !matches!(op.op_type, OpType::OpBoolNot) {
                let rc = eval_expr(record, schema, &op.args[1], &mut right);
                if rc != RC::Ok {
                    return rc;
                }
            }

            match op.op_type {
                OpType::OpBoolNot => bool_not(&left, result),
                OpType::OpBoolAnd => bool_and(&left, &right, result),
                OpType::OpBoolOr => bool_or(&left, &right, result),
                OpType::OpCompEqual => value_equals(&left, &right, result),
                OpType::OpCompSmaller => value_smaller(&left, &right, result),
            }
        }
        ExprUnion::Cons(value) => {
            *result = value.clone();
            RC::Ok
        }
        ExprUnion::AttrRef(attr_num) => crate::record_mgr::get_attr(record, schema, *attr_num, result),
    }
}

pub fn free_expr(_expr: &mut Expr) -> RC {
    RC::Ok
}

pub fn free_val(_val: &mut Value) {}

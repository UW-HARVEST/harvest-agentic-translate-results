use std::ffi::c_int;

#[derive(Clone, Copy)]
pub(crate) enum Operation {
    Add,
    Sub,
    Mul,
}

pub(crate) const OPERATION: Operation = if cfg!(feature = "add") {
    Operation::Add
} else if cfg!(feature = "sub") {
    Operation::Sub
} else if cfg!(feature = "mul") {
    Operation::Mul
} else {
    Operation::Add
};

pub(crate) const REPEAT: c_int = if cfg!(feature = "0") {
    0
} else if cfg!(feature = "1") {
    1
} else if cfg!(feature = "2") {
    2
} else if cfg!(feature = "3") {
    3
} else if cfg!(feature = "4") {
    4
} else if cfg!(feature = "5") {
    5
} else if cfg!(feature = "6") {
    6
} else if cfg!(feature = "7") {
    7
} else {
    5
};

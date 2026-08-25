use std::ffi::c_int;

#[cfg(any(
    all(feature = "add", feature = "sub"),
    all(feature = "add", feature = "mul"),
    all(feature = "sub", feature = "mul"),
))]
compile_error!("features `add`, `sub`, and `mul` are mutually exclusive");

#[cfg(any(
    all(feature = "0", feature = "1"),
    all(feature = "0", feature = "2"),
    all(feature = "0", feature = "3"),
    all(feature = "0", feature = "4"),
    all(feature = "0", feature = "5"),
    all(feature = "0", feature = "6"),
    all(feature = "0", feature = "7"),
    all(feature = "1", feature = "2"),
    all(feature = "1", feature = "3"),
    all(feature = "1", feature = "4"),
    all(feature = "1", feature = "5"),
    all(feature = "1", feature = "6"),
    all(feature = "1", feature = "7"),
    all(feature = "2", feature = "3"),
    all(feature = "2", feature = "4"),
    all(feature = "2", feature = "5"),
    all(feature = "2", feature = "6"),
    all(feature = "2", feature = "7"),
    all(feature = "3", feature = "4"),
    all(feature = "3", feature = "5"),
    all(feature = "3", feature = "6"),
    all(feature = "3", feature = "7"),
    all(feature = "4", feature = "5"),
    all(feature = "4", feature = "6"),
    all(feature = "4", feature = "7"),
    all(feature = "5", feature = "6"),
    all(feature = "5", feature = "7"),
    all(feature = "6", feature = "7"),
))]
compile_error!("features `0` through `7` are mutually exclusive");

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

use std::ffi::c_int;

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum Operation {
    Add,
    Sub,
    Mul,
}

// Alternate values override the C defaults when Cargo's additive features
// leave both the default and an explicitly requested value enabled.
#[cfg(feature = "mul")]
pub const OP: Operation = Operation::Mul;
#[cfg(all(not(feature = "mul"), feature = "sub"))]
pub const OP: Operation = Operation::Sub;
#[cfg(all(not(feature = "mul"), not(feature = "sub")))]
pub const OP: Operation = Operation::Add;

#[cfg(feature = "7")]
pub const REPEAT: c_int = 7;
#[cfg(all(not(feature = "7"), feature = "6"))]
pub const REPEAT: c_int = 6;
#[cfg(all(not(feature = "7"), not(feature = "6"), feature = "4"))]
pub const REPEAT: c_int = 4;
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "4"),
    feature = "3"
))]
pub const REPEAT: c_int = 3;
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "4"),
    not(feature = "3"),
    feature = "2"
))]
pub const REPEAT: c_int = 2;
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "4"),
    not(feature = "3"),
    not(feature = "2"),
    feature = "1"
))]
pub const REPEAT: c_int = 1;
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "4"),
    not(feature = "3"),
    not(feature = "2"),
    not(feature = "1"),
    feature = "0"
))]
pub const REPEAT: c_int = 0;
#[cfg(not(any(
    feature = "7",
    feature = "6",
    feature = "4",
    feature = "3",
    feature = "2",
    feature = "1",
    feature = "0"
)))]
pub const REPEAT: c_int = 5;

impl Operation {
    pub const fn name_c(self) -> *const i8 {
        match self {
            Self::Add => c"add".as_ptr(),
            Self::Sub => c"sub".as_ptr(),
            Self::Mul => c"mul".as_ptr(),
        }
    }

    pub fn apply(self, a: c_int, b: c_int) -> c_int {
        match self {
            Self::Add => a.wrapping_add(b),
            Self::Sub => a.wrapping_sub(b),
            Self::Mul => a.wrapping_mul(b),
        }
    }

    pub const fn initial(self) -> c_int {
        match self {
            Self::Add | Self::Sub => 0,
            Self::Mul => 1,
        }
    }

    pub fn step(self, accumulator: c_int, index: c_int) -> c_int {
        match self {
            Self::Add => accumulator.wrapping_add(index),
            Self::Sub => accumulator.wrapping_sub(index),
            Self::Mul => accumulator.wrapping_mul(index.wrapping_add(1)),
        }
    }
}

pub fn run_unrolled(operation: Operation, repeat: c_int) -> c_int {
    let mut accumulator = operation.initial();
    for index in 0..repeat {
        accumulator = operation.step(accumulator, index);
    }
    accumulator
}

pub fn generated_accumulator(operation: Operation, n: c_int) -> c_int {
    if (0..=6).contains(&n) {
        run_unrolled(operation, n)
    } else {
        operation.initial()
    }
}

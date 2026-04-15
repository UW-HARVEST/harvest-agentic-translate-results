pub fn op_add(a: i32, b: i32) -> i32 { a + b }
pub fn op_sub(a: i32, b: i32) -> i32 { a - b }
pub fn op_mul(a: i32, b: i32) -> i32 { a * b }

pub const INIT_ADD: i32 = 0;
pub const INIT_SUB: i32 = 0;
pub const INIT_MUL: i32 = 1;

pub fn step_add(acc: &mut i32, i: i32) { *acc += i; }
pub fn step_sub(acc: &mut i32, i: i32) { *acc -= i; }
pub fn step_mul(acc: &mut i32, i: i32) { *acc *= i + 1; }

pub const REPEAT: i32 = 5;

macro_rules! define_accum {
    ($func_name:ident, $init:expr, $step:ident) => {
        pub fn $func_name(n: i32) -> i32 {
            let mut acc = $init;
            match n {
                0 => {}
                1 => { $step(&mut acc, 0); }
                2 => { $step(&mut acc, 0); $step(&mut acc, 1); }
                3 => { $step(&mut acc, 0); $step(&mut acc, 1); $step(&mut acc, 2); }
                4 => { $step(&mut acc, 0); $step(&mut acc, 1); $step(&mut acc, 2); $step(&mut acc, 3); }
                5 => { $step(&mut acc, 0); $step(&mut acc, 1); $step(&mut acc, 2); $step(&mut acc, 3); $step(&mut acc, 4); }
                6 => { $step(&mut acc, 0); $step(&mut acc, 1); $step(&mut acc, 2); $step(&mut acc, 3); $step(&mut acc, 4); $step(&mut acc, 5); }
                _ => {}
            }
            acc
        }
    };
}

define_accum!(accum_add, INIT_ADD, step_add);

pub const G_OP: fn(i32, i32) -> i32 = op_add;
pub const G_OP_NAME: &str = "add";

pub fn helper_call(a: i32, b: i32) -> i32 {
    let r = op_add(a, b);
    let mut acc = INIT_ADD;
    step_add(&mut acc, 0);
    step_add(&mut acc, 1);
    step_add(&mut acc, 2);
    step_add(&mut acc, 3);
    step_add(&mut acc, 4);
    println!("helper.call={} helper.acc={}", r, acc);
    r + acc
}

pub fn helper_ptr(a: i32, b: i32) -> i32 {
    let fp: fn(i32, i32) -> i32 = op_add;
    let r = fp(a, b);
    println!("helper.ptr={}", r);
    r
}

pub fn use_generated(n: i32) -> i32 {
    let r = accum_add(n);
    println!("gen.acc={}", r);
    r
}

pub const OP_NAME: &str = "add";
pub const REPEAT: i32 = 5;

pub fn op_add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn op_sub(a: i32, b: i32) -> i32 {
    a - b
}

pub fn op_mul(a: i32, b: i32) -> i32 {
    a * b
}

pub fn op_fn(a: i32, b: i32) -> i32 {
    op_add(a, b)
}

pub fn init_for_op() -> i32 {
    0
}

fn step_op(acc: &mut i32, i: i32) {
    *acc += i;
}

fn rep0(_acc: &mut i32) {}

fn rep1(acc: &mut i32) {
    step_op(acc, 0);
}

fn rep2(acc: &mut i32) {
    rep1(acc);
    step_op(acc, 1);
}

fn rep3(acc: &mut i32) {
    rep2(acc);
    step_op(acc, 2);
}

fn rep4(acc: &mut i32) {
    rep3(acc);
    step_op(acc, 3);
}

fn rep5(acc: &mut i32) {
    rep4(acc);
    step_op(acc, 4);
}

fn rep6(acc: &mut i32) {
    rep5(acc);
    step_op(acc, 5);
}

fn rep7(acc: &mut i32) {
    rep6(acc);
    step_op(acc, 6);
}

pub fn run_loop(acc: &mut i32, n: i32) {
    match n {
        0 => rep0(acc),
        1 => rep1(acc),
        2 => rep2(acc),
        3 => rep3(acc),
        4 => rep4(acc),
        5 => rep5(acc),
        6 => rep6(acc),
        7 => rep7(acc),
        _ => {}
    }
}

pub fn accum_op(n: i32) -> i32 {
    let mut acc = init_for_op();
    match n {
        0 => rep0(&mut acc),
        1 => rep1(&mut acc),
        2 => rep2(&mut acc),
        3 => rep3(&mut acc),
        4 => rep4(&mut acc),
        5 => rep5(&mut acc),
        6 => rep6(&mut acc),
        _ => {}
    }
    acc
}

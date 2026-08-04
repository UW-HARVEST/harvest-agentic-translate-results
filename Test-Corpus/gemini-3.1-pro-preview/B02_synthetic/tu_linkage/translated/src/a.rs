use std::sync::atomic::{AtomicI32, Ordering};

static STATE_A: AtomicI32 = AtomicI32::new(0);

fn target(code: i32) -> i32 {
    let mut state = STATE_A.load(Ordering::Relaxed);
    if code < 0 {
        return if (state & 1) != 0 { 6 } else { 5 };
    }
    state = state ^ (code << 1);
    STATE_A.store(state, Ordering::Relaxed);
    let k = ((code >> 2) ^ state) & 7;
    match k {
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 1,
        4 => 3,
        5 | 6 => 5,
        _ => 7,
    }
}

fn a_bias_call(x: i32) -> i32 {
    target((x ^ 0x55) + 7)
}

fn wrap(x: i32) -> i32 {
    target(x - 5)
}

pub fn call_a_once(x: i32) -> i32 {
    let a = target(x);
    let b = wrap(a);
    let c = target(b ^ 3);
    let d = a_bias_call(b);
    a ^ (b << 1) ^ (c << 2) ^ (d << 3)
}

pub fn process_a_stream(xs: &[i32]) -> i32 {
    let mut acc: usize = 0;
    for &v in xs {
        for j in 0..3 {
            let t = target(v + j);
            if (t & 1) == 0 {
                acc = acc.wrapping_add(t as usize);
                continue;
            }
            acc ^= (t << j) as usize;
            if t == 5 {
                break;
            }
        }
    }
    let mut acc_i64 = acc as i64;
    if acc_i64 > 0x7fffffff {
        acc_i64 = 0x7fffffff;
    }
    if acc_i64 < -0x80000000 {
        acc_i64 = -0x80000000;
    }
    acc_i64 as i32
}

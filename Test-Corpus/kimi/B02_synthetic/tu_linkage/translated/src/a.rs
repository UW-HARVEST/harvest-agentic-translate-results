use std::sync::atomic::{AtomicI32, Ordering};

static STATE_A: AtomicI32 = AtomicI32::new(0);

fn a_bias_call<F: Fn(i32) -> i32>(fp: F, x: i32) -> i32 {
    fp((x ^ 0x55) + 7)
}

fn target_a(code: i32) -> i32 {
    if code < 0 {
        let state = STATE_A.load(Ordering::SeqCst);
        return if (state & 1) != 0 { 6 } else { 5 };
    }
    let mut state = STATE_A.load(Ordering::SeqCst);
    state = state ^ (code << 1);
    STATE_A.store(state, Ordering::SeqCst);
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

fn wrap(x: i32) -> i32 {
    target_a(x - 5)
}

fn a_mac_call<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 {
    a_bias_call(f, x)
}

pub fn call_a_once(x: i32) -> i32 {
    let a = target_a(x);
    let b = wrap(a);
    let c = target_a(b ^ 3);
    let d = a_mac_call(&target_a, b);
    a ^ (b << 1) ^ (c << 2) ^ (d << 3)
}

pub fn process_a_stream(xs: &[i32]) -> i32 {
    let mut acc: i64 = 0;
    for i in 0..xs.len() {
        let v = xs[i];
        for j in 0..3 {
            let t = target_a(v + j);
            if (t & 1) == 0 {
                acc += t as i64;
                continue;
            }
            acc ^= (t << j) as i64;
            if t == 5 {
                break;
            }
        }
    }
    if acc > 0x7fffffff {
        acc = 0x7fffffff;
    }
    if acc < -0x80000000 {
        acc = -0x80000000;
    }
    acc as i32
}

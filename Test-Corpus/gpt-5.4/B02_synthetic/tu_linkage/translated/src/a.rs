use std::sync::atomic::{AtomicI32, Ordering};

static STATE_A: AtomicI32 = AtomicI32::new(0);

fn a_bias_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    fp((x ^ 0x55) + 7)
}

fn target(code: i32) -> i32 {
    if code < 0 {
        return if (STATE_A.load(Ordering::Relaxed) & 1) != 0 { 6 } else { 5 };
    }
    let new_state = STATE_A.load(Ordering::Relaxed) ^ (code << 1);
    STATE_A.store(new_state, Ordering::Relaxed);
    let k = ((code >> 2) ^ new_state) & 7;
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
    target(x - 5)
}

pub fn call_a_once(x: i32) -> i32 {
    let fp: fn(i32) -> i32 = target;
    let a = fp(x);
    let b = wrap(a);
    let c = target(b ^ 3);
    let d = a_bias_call(target, b);
    a ^ (b << 1) ^ (c << 2) ^ (d << 3)
}

pub fn process_a_stream(xs: &[i32]) -> i32 {
    let mut acc: i64 = 0;
    for &v in xs {
        for j in 0..3 {
            let t = target(v + j);
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
    if acc > 0x7fffffff_i64 {
        acc = 0x7fffffff_i64;
    }
    if acc < -0x80000000_i64 {
        acc = -0x80000000_i64;
    }
    acc as i32
}

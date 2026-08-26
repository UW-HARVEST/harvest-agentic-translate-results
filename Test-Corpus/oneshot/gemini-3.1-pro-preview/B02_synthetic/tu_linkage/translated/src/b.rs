use std::sync::atomic::{AtomicI32, Ordering};

static FLIPFLOP: AtomicI32 = AtomicI32::new(0);

fn target(code: i32) -> i32 {
    let mut ff = FLIPFLOP.load(Ordering::Relaxed);
    ff ^= 1;
    FLIPFLOP.store(ff, Ordering::Relaxed);
    if code < 0 {
        return if ff != 0 { 2 } else { 6 };
    }
    let z = (code ^ (if ff != 0 { 0x7f } else { 0x1f })) % 8;
    match z {
        0 | 7 => 4,
        1 | 2 => 3,
        3 => 1,
        4 => 0,
        5 => 5,
        _ => 7,
    }
}

fn b_twist_call(x: i32) -> i32 {
    target(((x + 9) ^ 0x2222) - 17)
}

fn w2(x: i32) -> i32 {
    target(x + 9)
}

pub fn call_b_once(x: i32) -> i32 {
    let a = target(x);
    let b = w2(a);
    let c = b_twist_call(a);
    let d = target(c ^ x);
    (a << 1) ^ (b << 2) ^ (c << 3) ^ (d << 4)
}

pub fn process_b_stream(xs: &[i32]) -> i32 {
    let mut acc: i32 = 1;
    for &v in xs {
        let mut iter = 0;
        while { iter += 1; iter <= 4 } {
            let t = target(v - iter);
            if t == 6 {
                acc = acc.wrapping_sub(t);
                break;
            }
            if t == 3 {
                continue;
            }
            acc = acc.wrapping_mul(3) ^ t;
        }
    }
    acc
}

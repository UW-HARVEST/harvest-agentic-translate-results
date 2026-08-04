use std::sync::atomic::{AtomicI32, Ordering};

static FLIPFLOP: AtomicI32 = AtomicI32::new(0);

fn b_twist_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    fp(((x + 9) ^ 0x2222) - 17)
}

fn target(code: i32) -> i32 {
    let new_flip = FLIPFLOP.fetch_xor(1, Ordering::Relaxed) ^ 1;
    if code < 0 {
        return if new_flip != 0 { 2 } else { 6 };
    }
    let z = (code ^ if new_flip != 0 { 0x7f } else { 0x1f }) % 8;
    if z == 0 || z == 7 {
        return 4;
    }
    if z == 1 || z == 2 {
        return 3;
    }
    if z == 3 {
        return 1;
    }
    if z == 4 {
        return 0;
    }
    if z == 5 {
        return 5;
    }
    7
}

fn w2(x: i32) -> i32 {
    target(x + 9)
}

pub fn call_b_once(x: i32) -> i32 {
    let fp: fn(i32) -> i32 = target;
    let a = target(x);
    let b = w2(a);
    let c = b_twist_call(target, a);
    let d = fp(c ^ x);
    (a << 1) ^ (b << 2) ^ (c << 3) ^ (d << 4)
}

pub fn process_b_stream(xs: &[i32]) -> i32 {
    let mut acc = 1i32;
    for &v in xs {
        let mut iter = 0;
        while {
            iter += 1;
            iter <= 4
        } {
            let t = target(v - iter);
            if t == 6 {
                acc -= t;
                break;
            }
            if t == 3 {
                continue;
            }
            acc = (acc * 3) ^ t;
        }
    }
    acc
}

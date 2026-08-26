use std::sync::atomic::{AtomicI32, Ordering};

static FLIPFLOP: AtomicI32 = AtomicI32::new(0);

fn b_twist_call<F: Fn(i32) -> i32>(fp: F, x: i32) -> i32 {
    fp(((x + 9) ^ 0x2222) - 17)
}

fn target_b(code: i32) -> i32 {
    let ff = FLIPFLOP.load(Ordering::SeqCst);
    FLIPFLOP.store(ff ^ 1, Ordering::SeqCst);
    let flip = (ff ^ 1) != 0;
    if code < 0 {
        return if flip { 2 } else { 6 };
    }
    let z = (code ^ if flip { 0x7f } else { 0x1f }) % 8;
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
    target_b(x + 9)
}

fn b_mac_call<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 {
    b_twist_call(f, x)
}

pub fn call_b_once(x: i32) -> i32 {
    let a = target_b(x);
    let b = w2(a);
    let c = b_mac_call(&target_b, a);
    let d = target_b(c ^ x);
    (a << 1) ^ (b << 2) ^ (c << 3) ^ (d << 4)
}

pub fn process_b_stream(xs: &[i32]) -> i32 {
    let mut acc: i32 = 1;
    for i in 0..xs.len() {
        let v = xs[i];
        let mut iter = 0;
        while iter < 4 {
            iter += 1;
            let t = target_b(v - iter);
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

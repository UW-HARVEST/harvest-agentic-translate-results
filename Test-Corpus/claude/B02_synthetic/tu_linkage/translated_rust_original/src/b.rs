// Translation of c_src/src/b.c

use std::cell::Cell;

thread_local! {
    static FLIPFLOP: Cell<i32> = const { Cell::new(0) };
}

fn b_twist_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    // C: fp(((x + 9) ^ 0x2222) - 17)
    fp(((x.wrapping_add(9)) ^ 0x2222).wrapping_sub(17))
}

fn b_target(code: i32) -> i32 {
    let new_flip = FLIPFLOP.with(|f| {
        let v = f.get() ^ 1;
        f.set(v);
        v
    });
    if code < 0 {
        return if new_flip != 0 { 2 } else { 6 };
    }
    let mask = if new_flip != 0 { 0x7f } else { 0x1f };
    // C: int z = (code ^ mask) % 8;  -- signed mod can be negative if (code^mask) is negative.
    let z = (code ^ mask) % 8;
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
    b_target(x.wrapping_add(9))
}

pub fn call_b_once(x: i32) -> i32 {
    let fp: fn(i32) -> i32 = b_target;
    let a = b_target(x);
    let b = w2(a);
    // B_MAC_CALL(F,X) = b_twist_call(F,X)
    let c = b_twist_call(b_target, a);
    let d = fp(c ^ x);
    (a.wrapping_shl(1)) ^ (b.wrapping_shl(2)) ^ (c.wrapping_shl(3)) ^ (d.wrapping_shl(4))
}

pub fn process_b_stream(xs: &[i32]) -> i32 {
    let mut acc: i32 = 1;
    for &v in xs {
        let mut iter: i32 = 0;
        loop {
            iter += 1;
            if iter > 4 {
                break;
            }
            let t = b_target(v.wrapping_sub(iter));
            if t == 6 {
                acc = acc.wrapping_sub(t);
                break;
            }
            if t == 3 {
                continue;
            }
            acc = (acc.wrapping_mul(3)) ^ t;
        }
    }
    acc
}

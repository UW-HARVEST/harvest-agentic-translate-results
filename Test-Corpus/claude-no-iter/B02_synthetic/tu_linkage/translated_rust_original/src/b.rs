// Translated from c_src/src/b.c

use std::cell::Cell;

thread_local! {
    static FLIPFLOP: Cell<i32> = Cell::new(0);
}

fn b_twist_call<F: Fn(i32) -> i32>(fp: F, x: i32) -> i32 {
    // C: fp(((x + 9) ^ 0x2222) - 17)
    fp(((x.wrapping_add(9)) ^ 0x2222).wrapping_sub(17))
}

fn target_b(code: i32) -> i32 {
    let new_ff = FLIPFLOP.with(|c| {
        let v = c.get() ^ 1;
        c.set(v);
        v
    });
    if code < 0 {
        return if new_ff != 0 { 2 } else { 6 };
    }
    let mask = if new_ff != 0 { 0x7f } else { 0x1f };
    // C: int z = (code ^ mask) % 8;
    // Note: C's % preserves sign of dividend; the result can be negative if (code ^ mask) is negative.
    let xored = code ^ mask;
    let z = xored % 8;
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
    target_b(x.wrapping_add(9))
}

pub fn call_b_once(x: i32) -> i32 {
    let a = target_b(x);
    let b = w2(a);
    // B_MAC_CALL(F,X) = b_twist_call(F, X)
    let c = b_twist_call(target_b, a);
    let d = target_b(c ^ x);
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
            let t = target_b(v.wrapping_sub(iter));
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

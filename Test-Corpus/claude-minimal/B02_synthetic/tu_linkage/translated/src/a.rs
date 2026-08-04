// Translated from c_src/src/a.c

use std::cell::Cell;

thread_local! {
    static STATE_A: Cell<i32> = Cell::new(0);
}

#[inline]
fn a_bias_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    // (x ^ 0x55) + 7  with C-style wrapping
    fp((x ^ 0x55).wrapping_add(7))
}

fn a_target(code: i32) -> i32 {
    if code < 0 {
        let s = STATE_A.with(|s| s.get());
        return if s & 1 != 0 { 6 } else { 5 };
    }
    let new_state = STATE_A.with(|s| {
        let cur = s.get();
        let ns = cur ^ (code.wrapping_shl(1));
        s.set(ns);
        ns
    });
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

#[inline]
fn wrap(x: i32) -> i32 {
    a_target(x.wrapping_sub(5))
}

// A_MAC_CALL(F, X) -> a_bias_call((F), (X))
#[inline]
fn a_mac_call(f: fn(i32) -> i32, x: i32) -> i32 {
    a_bias_call(f, x)
}

pub fn call_a_once(x: i32) -> i32 {
    let fp: fn(i32) -> i32 = a_target;
    let a = fp(x);
    let b = wrap(a);
    let c = a_target(b ^ 3);
    let d = a_mac_call(a_target, b);
    a ^ (b.wrapping_shl(1)) ^ (c.wrapping_shl(2)) ^ (d.wrapping_shl(3))
}

pub fn process_a_stream(xs: &[i32]) -> i32 {
    // The original C uses size_t acc but assigns potentially-negative results.
    // Use i64 to mirror the comparison/clamp logic, then cast at the end.
    let mut acc: i64 = 0;
    for &v in xs.iter() {
        for j in 0..3i32 {
            let t = a_target(v.wrapping_add(j));
            if (t & 1) == 0 {
                acc = acc.wrapping_add(t as i64);
                continue;
            }
            acc ^= (t.wrapping_shl(j as u32)) as i64;
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

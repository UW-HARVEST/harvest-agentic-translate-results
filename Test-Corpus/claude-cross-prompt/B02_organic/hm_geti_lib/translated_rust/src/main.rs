// Rust translation of c_src/src/lib.c
//
// The original C source is a library implementing the stb_ds hash map. It
// exposes a single public symbol `hm_geti(int num)` which exercises a series
// of internal assertions on the hash map. The library performs no I/O, so the
// executable produces no stdout output for valid inputs (matching the C
// behavior, which would only produce output if assertions failed).
//
// To make this an executable, we read a single integer `num` from stdin
// (matching scanf("%d", ...) behavior, which reads across whitespace including
// newlines) and call the equivalent of `hm_geti(num)`.

use std::collections::HashMap;
use std::io::Read;

/// Equivalent of stb_ds `hm_geti` test routine. Uses a Rust HashMap with an
/// explicit "default" value (-2) to mirror the `hmdefault(intmap, -2)` call in
/// the C version. All assertions must hold for successful execution.
fn hm_geti(num: i32) {
    // intmap: i32 -> i32. Default value is -2 once `hmdefault` is set.
    let mut intmap: HashMap<i32, i32> = HashMap::new();
    let default_val: i32 = -2;

    // Before any insert, key 1 is absent (hmgeti returns -1).
    let i = 1i32;
    assert!(!intmap.contains_key(&i)); // hmgeti(intmap, 1) == -1

    // hmdefault(intmap, -2)
    // After hmdefault, hmgeti still returns -1 for missing keys but hmget
    // returns the default value.
    assert!(!intmap.contains_key(&i)); // hmgeti(intmap, 1) == -1
    let v = *intmap.get(&i).unwrap_or(&default_val);
    assert_eq!(v, -2); // hmget(intmap, 1) == -2

    // for (i = 0; i < num; i += 2) hmput(intmap, i, i*5);
    let mut i = 0i32;
    while i < num {
        intmap.insert(i, i.wrapping_mul(5));
        i += 2;
    }

    // Verify all values: even keys -> i*5, odd keys -> default (-2).
    let mut i = 0i32;
    while i < num {
        let got = *intmap.get(&i).unwrap_or(&default_val);
        if i & 1 != 0 {
            assert_eq!(got, -2);
        } else {
            assert_eq!(got, i.wrapping_mul(5));
        }
        // hmget_ts variant — same semantics for our purposes.
        let got_ts = *intmap.get(&i).unwrap_or(&default_val);
        if i & 1 != 0 {
            assert_eq!(got_ts, -2);
        } else {
            assert_eq!(got_ts, i.wrapping_mul(5));
        }
        i += 1;
    }

    // Re-insert even keys with new values: i*3.
    let mut i = 0i32;
    while i < num {
        intmap.insert(i, i.wrapping_mul(3));
        i += 2;
    }

    // Verify: even keys -> i*3, odd keys -> default (-2).
    let mut i = 0i32;
    while i < num {
        let got = *intmap.get(&i).unwrap_or(&default_val);
        if i & 1 != 0 {
            assert_eq!(got, -2);
        } else {
            assert_eq!(got, i.wrapping_mul(3));
        }
        i += 1;
    }

    // for (i = 2; i < num; i += 4) hmdel(intmap, i);
    let mut i = 2i32;
    while i < num {
        intmap.remove(&i);
        i += 4;
    }

    // Verify: keys with (i & 3) != 0 -> default; keys with (i & 3) == 0 -> i*3.
    let mut i = 0i32;
    while i < num {
        let got = *intmap.get(&i).unwrap_or(&default_val);
        if i & 3 != 0 {
            assert_eq!(got, -2);
        } else {
            assert_eq!(got, i.wrapping_mul(3));
        }
        i += 1;
    }

    // Delete every key.
    let mut i = 0i32;
    while i < num {
        intmap.remove(&i);
        i += 1;
    }

    // Now every key returns default.
    let mut i = 0i32;
    while i < num {
        let got = *intmap.get(&i).unwrap_or(&default_val);
        assert_eq!(got, -2);
        i += 1;
    }

    // hmfree(intmap) — re-init.
    intmap.clear();

    // Re-populate even keys, then free again.
    let mut i = 0i32;
    while i < num {
        intmap.insert(i, i.wrapping_mul(3));
        i += 2;
    }
    intmap.clear();
}

/// Read an integer from stdin in scanf("%d", ...) style: skip leading
/// whitespace (including newlines) and parse an optional sign followed by
/// decimal digits.
fn scanf_int() -> Option<i32> {
    let mut buf = String::new();
    if std::io::stdin().read_to_string(&mut buf).is_err() {
        return None;
    }
    let bytes = buf.as_bytes();
    let mut idx = 0usize;
    // Skip leading whitespace.
    while idx < bytes.len() && (bytes[idx] as char).is_whitespace() {
        idx += 1;
    }
    if idx >= bytes.len() {
        return None;
    }
    let start = idx;
    if bytes[idx] == b'-' || bytes[idx] == b'+' {
        idx += 1;
    }
    let digit_start = idx;
    while idx < bytes.len() && (bytes[idx] as char).is_ascii_digit() {
        idx += 1;
    }
    if idx == digit_start {
        return None;
    }
    let s = std::str::from_utf8(&bytes[start..idx]).ok()?;
    s.parse::<i64>().ok().map(|v| v as i32)
}

fn main() {
    let num = scanf_int().unwrap_or(0);
    hm_geti(num);
}

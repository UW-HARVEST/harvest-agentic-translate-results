//! Phase B — additional hardening for the value-dependent code paths that a
//! single hand-picked input would miss (rows C27/C41/C43/C60/C61/C81/C83).
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_int;
use std::fmt::Write as _;

/// `case_insensitive_strcmp` calls `tolower()` on every byte, so object lookups
/// must be exercised with the whole 1..255 byte range, not just ASCII.
#[test]
fn extra_object_lookup_all_byte_values() {
    diff("extra: object lookup over every byte value", |api| unsafe {
        let mut log = String::new();
        // keys made of a single byte, in both cases where applicable
        let obj = (api.cJSON_CreateObject)();
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for b in 1u8..=255 {
            keys.push(vec![b]);
        }
        for (i, k) in keys.iter().enumerate() {
            let kb = CBuf::new(k);
            (api.cJSON_AddItemToObject)(obj, kb.ptr(), (api.cJSON_CreateNumber)(i as f64));
        }
        for b in 1u8..=255 {
            let probe = CBuf::new(&[b]);
            let ins = (api.cJSON_GetObjectItem)(obj, probe.ptr());
            let sen = (api.cJSON_GetObjectItemCaseSensitive)(obj, probe.ptr());
            let _ = writeln!(
                log,
                "byte {b:3}: insens={:?} sens={:?}",
                (!ins.is_null()).then(|| (*ins).valueint),
                (!sen.is_null()).then(|| (*sen).valueint)
            );
        }
        // multi-byte keys with a random mix, probed with random variants
        let mut rng = Rng::new(0x5EED_0BEE_F00D_0001);
        let mut owned: Vec<Vec<u8>> = Vec::new();
        let obj2 = (api.cJSON_CreateObject)();
        for i in 0..60 {
            let k = rng.ascii(6);
            owned.push(k.clone());
            let kb = CBuf::new(&k);
            (api.cJSON_AddItemToObject)(obj2, kb.ptr(), (api.cJSON_CreateNumber)(i as f64));
        }
        for _ in 0..400 {
            let probe = if rng.bool() {
                // a mutated copy of an existing key (case flips / byte tweaks)
                let mut k = owned[rng.below(owned.len())].clone();
                if !k.is_empty() {
                    let idx = rng.below(k.len());
                    k[idx] = match rng.below(3) {
                        0 => k[idx].to_ascii_uppercase(),
                        1 => k[idx].to_ascii_lowercase(),
                        _ => k[idx].wrapping_add(1).max(1),
                    };
                }
                k
            } else {
                rng.ascii(6)
            };
            let pb = CBuf::new(&probe);
            let ins = (api.cJSON_GetObjectItem)(obj2, pb.ptr());
            let sen = (api.cJSON_GetObjectItemCaseSensitive)(obj2, pb.ptr());
            let _ = writeln!(
                log,
                "probe {}: insens={:?} sens={:?} has={}",
                show(&probe),
                (!ins.is_null()).then(|| (*ins).valueint),
                (!sen.is_null()).then(|| (*sen).valueint),
                (api.cJSON_HasObjectItem)(obj2, pb.ptr())
            );
        }
        (api.cJSON_Delete)(obj);
        (api.cJSON_Delete)(obj2);
        log
    });
}

/// `\u0000` produces an embedded NUL inside `valuestring` / `string`, which
/// truncates every later `strlen`-based operation. Both implementations must
/// truncate at exactly the same place.
#[test]
fn extra_embedded_nul_strings() {
    diff("extra: embedded NUL via \\u0000", |api| unsafe {
        let mut log = String::new();
        let docs: [&[u8]; 12] = [
            br#""\u0000""#,
            br#""a\u0000b""#,
            br#""\u0000\u0000""#,
            br#"["a\u0000b","c"]"#,
            br#"{"\u0000":1}"#,
            br#"{"a\u0000b":1,"a":2}"#,
            br#"{"k":"v\u0000w"}"#,
            br#"[{"\u0000a":"\u0000b"}]"#,
            br#""A\u0000B""#,
            br#"{"\u0000":1,"":2}"#,
            br#""\u0000tail""#,
            br#"{"x":"\u0000"}"#,
        ];
        // and the same shapes with a *raw* NUL byte in the document text
        let raw_nul: Vec<Vec<u8>> = vec![
            vec![b'"', 0, b'"'],
            vec![b'"', b'a', 0, b'b', b'"'],
            vec![b'{', b'"', 0, b'"', b':', b'1', b'}'],
            vec![b'[', b'1', 0, b',', b'2', b']'],
            vec![0],
            vec![b'1', 0, b'2'],
        ];
        let all: Vec<&[u8]> = docs
            .iter()
            .copied()
            .chain(raw_nul.iter().map(|v| v.as_slice()))
            .collect();
        for d in all {
            let b = CBuf::new(d);
            let root = (api.cJSON_Parse)(b.ptr());
            let _ = writeln!(log, "src={} null={}", show(d), root.is_null());
            let _ = write!(log, "  {}", dump(root));
            let pf = take_print(api, (api.cJSON_Print)(root));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(root));
            let _ = writeln!(
                log,
                "  fmt={} unfmt={}",
                pf.map(|v| show(&v)).unwrap_or("NULL".into()),
                pu.map(|v| show(&v)).unwrap_or("NULL".into())
            );
            // lookups, duplication and comparison over the truncated strings
            if !root.is_null() {
                let empty = cs("");
                let _ = writeln!(
                    log,
                    "  get(\"\") insens={} sens={} size={}",
                    (api.cJSON_GetObjectItem)(root, empty.as_ptr()).is_null(),
                    (api.cJSON_GetObjectItemCaseSensitive)(root, empty.as_ptr()).is_null(),
                    (api.cJSON_GetArraySize)(root)
                );
                let d2 = (api.cJSON_Duplicate)(root, 1);
                let _ = write!(log, "  dup: {}", dump(d2));
                let _ = writeln!(
                    log,
                    "  cmp={} cmp0={}",
                    (api.cJSON_Compare)(root, d2, 1),
                    (api.cJSON_Compare)(root, d2, 0)
                );
                (api.cJSON_Delete)(d2);
            }
            (api.cJSON_Delete)(root);
        }
        log
    });
}

/// Printing a deeply nested document drives `print_object`'s depth-indexed
/// indentation and many `ensure()` growths.
#[test]
fn extra_print_deeply_nested() {
    diff("extra: print deeply nested documents", |api| unsafe {
        let mut log = String::new();
        for depth in [1usize, 2, 3, 10, 64, 100, 500, 999] {
            for kind in 0..3 {
                let mut src: Vec<u8> = Vec::new();
                match kind {
                    0 => {
                        for _ in 0..depth {
                            src.push(b'[');
                        }
                        src.push(b'1');
                        for _ in 0..depth {
                            src.push(b']');
                        }
                    }
                    1 => {
                        for _ in 0..depth {
                            src.extend_from_slice(b"{\"k\":");
                        }
                        src.push(b'1');
                        for _ in 0..depth {
                            src.push(b'}');
                        }
                    }
                    _ => {
                        for _ in 0..depth / 2 {
                            src.extend_from_slice(b"[{\"k\":");
                        }
                        src.push(b'1');
                        for _ in 0..depth / 2 {
                            src.extend_from_slice(b"}]");
                        }
                    }
                }
                let b = CBuf::new(&src);
                let root = (api.cJSON_Parse)(b.ptr());
                let pf = take_print(api, (api.cJSON_Print)(root));
                let pu = take_print(api, (api.cJSON_PrintUnformatted)(root));
                let h = |v: &Option<Vec<u8>>| {
                    v.as_ref().map(|x| {
                        x.iter()
                            .fold(0u64, |a, &b| a.wrapping_mul(1000003).wrapping_add(b as u64))
                    })
                };
                let _ = writeln!(
                    log,
                    "depth={depth} kind={kind}: parsed={} fmt_len={:?} fmt_hash={:?} unfmt_len={:?} unfmt_hash={:?}",
                    !root.is_null(),
                    pf.as_ref().map(|v| v.len()),
                    h(&pf),
                    pu.as_ref().map(|v| v.len()),
                    h(&pu)
                );
                if depth <= 10 {
                    let _ = writeln!(log, "  fmt={}", pf.as_ref().map(|v| show(v)).unwrap_or_default());
                    let _ = writeln!(log, "  unfmt={}", pu.as_ref().map(|v| show(v)).unwrap_or_default());
                } else if let Some(v) = &pf {
                    let _ = writeln!(
                        log,
                        "  head={} tail={}",
                        show(&v[..v.len().min(80)]),
                        show(&v[v.len().saturating_sub(80)..])
                    );
                }
                // preallocated printing of the same deep document, exact and short
                if let Some(v) = &pf {
                    for len in [v.len() as c_int, v.len() as c_int - 1, v.len() as c_int / 2] {
                        if len < 0 {
                            continue;
                        }
                        let mut buf = vec![0x2Du8; len as usize + 2];
                        let rc = (api.cJSON_PrintPreallocated)(
                            root,
                            buf.as_mut_ptr() as *mut i8,
                            len,
                            1,
                        );
                        let hh = buf
                            .iter()
                            .fold(0u64, |a, &b| a.wrapping_mul(1000003).wrapping_add(b as u64));
                        let _ = writeln!(log, "  prealloc len={len} rc={rc} hash={hh}");
                    }
                }
                (api.cJSON_Delete)(root);
            }
        }
        log
    });
}

/// `compare_double`'s relative-epsilon test over randomized values.
#[test]
fn extra_compare_random_numbers() {
    diff("extra: cJSON_Compare over random numbers", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0xC0DE_C0DE_C0DE_0001);
        for i in 0..800 {
            let x = if i % 4 == 0 { rng.any_f64() } else { rng.nice_f64() };
            let y = match i % 5 {
                0 => x,
                1 => x + x * f64::EPSILON,
                2 => x * (1.0 + 4.0 * f64::EPSILON),
                3 => -x,
                _ => rng.nice_f64(),
            };
            let a = (api.cJSON_CreateNumber)(x);
            let b = (api.cJSON_CreateNumber)(y);
            let _ = writeln!(
                log,
                "0x{:016x} vs 0x{:016x}: cmp1={} cmp0={}",
                x.to_bits(),
                y.to_bits(),
                (api.cJSON_Compare)(a, b, 1),
                (api.cJSON_Compare)(a, b, 0)
            );
            (api.cJSON_Delete)(a);
            (api.cJSON_Delete)(b);
        }
        log
    });
}

/// Duplicate keys: `get_object_item` always returns the first match, so
/// `cJSON_Compare`'s two-pass object walk has to agree exactly.
#[test]
fn extra_compare_duplicate_keys() {
    diff("extra: cJSON_Compare with duplicate keys", |api| unsafe {
        let mut log = String::new();
        let pairs: [(&str, &str); 12] = [
            (r#"{"a":1,"a":2}"#, r#"{"a":1,"a":2}"#),
            (r#"{"a":1,"a":2}"#, r#"{"a":2,"a":1}"#),
            (r#"{"a":1,"a":2}"#, r#"{"a":1}"#),
            (r#"{"a":1}"#, r#"{"a":1,"a":2}"#),
            (r#"{"a":1,"A":2}"#, r#"{"a":1,"A":2}"#),
            (r#"{"a":1,"A":2}"#, r#"{"A":2,"a":1}"#),
            (r#"{"a":1,"A":1}"#, r#"{"a":1}"#),
            (r#"{"":1,"":2}"#, r#"{"":1,"":2}"#),
            (r#"{"a":{"b":1,"b":2}}"#, r#"{"a":{"b":1,"b":2}}"#),
            (r#"{"a":{"b":1,"b":2}}"#, r#"{"a":{"b":2,"b":1}}"#),
            (r#"[{"a":1},{"a":1}]"#, r#"[{"a":1},{"a":1}]"#),
            (r#"{"a":1,"b":2,"a":3}"#, r#"{"b":2,"a":1,"a":3}"#),
        ];
        for (x, y) in pairs {
            let xb = cs(x);
            let yb = cs(y);
            let a = (api.cJSON_Parse)(xb.as_ptr());
            let b = (api.cJSON_Parse)(yb.as_ptr());
            for cs_ in [0i32, 1] {
                let _ = writeln!(log, "{x} vs {y} cs={cs_}: {}", (api.cJSON_Compare)(a, b, cs_));
            }
            let _ = write!(log, "  a: {}", dump(a));
            let _ = write!(log, "  b: {}", dump(b));
            (api.cJSON_Delete)(a);
            (api.cJSON_Delete)(b);
        }
        log
    });
}

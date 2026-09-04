//! Phase B — CONFIGS.md section C: the full encoder configuration matrix,
//! exercised through every public dump entry point (`json_dumps`, `json_dumpb`,
//! `json_dumpf`, `json_dumpfd`, `json_dump_file`, `json_dump_callback`).
mod common;

use common::*;
use std::io::Read;
use std::os::raw::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;

/// Documents to encode.  Covers every shape `dump.c` branches on.
fn documents() -> Vec<String> {
    let mut v = corpus();
    // C17 escapes and C4 ASCII/non-ASCII classes
    v.push("[\"\\u0000\"]".into()); // needs JSON_ALLOW_NUL on load
    v.push("[\"\\u0001\\u001f\\u007f\"]".into());
    v.push("[\"quote\\\" back\\\\ slash/ bs\\b ff\\f nl\\n cr\\r tab\\t\"]".into());
    v.push("[\"\\u00e9\\u20ac\\u4e2d\\ud834\\udd1e\"]".into());
    // C5 sort-keys shapes
    v.push("{\"b\":1,\"a\":2,\"c\":3}".into());
    v.push("{\"a\":1,\"aa\":2,\"a\":3,\"aaa\":4,\"ab\":5,\"\":6}".into());
    v.push("{\"zz\":1,\"z\":2,\"\":3,\"y\":4,\"yy\":5,\"yyy\":6,\"x\":7,\"w\":8,\"v\":9,\"u\":10}".into());
    // C18 integer boundaries
    v.push("[9223372036854775807,-9223372036854775808,0,-0]".into());
    // C19 reals over the exponent switch
    v.push("[1e-5,-1e-5,1e-4,-1e-4,1e16,-1e16,1e17,-1e17,1.0,-1.0,0.5]".into());
    v.push("[5e-324,1.7976931348623157e308,2.2250738585072014e-308]".into());
    // C21 empty containers nested in indented parents
    v.push("{\"a\":[],\"b\":{},\"c\":[[],{}],\"d\":{\"e\":[]}}".into());
    // C20 deep nesting -> n_spaces > 32 in dump_indent
    v.push(format!("{}1{}", "[".repeat(200), "]".repeat(200)));
    v.push(format!(
        "{}{}",
        (0..60).map(|i| format!("{{\"k{i}\":")).collect::<String>(),
        format!("1{}", "}".repeat(60))
    ));
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..120 {
        v.push(random_json(&mut rng, 4, true));
    }
    v
}

/// Every encoder flag mask worth testing.
fn flag_masks() -> Vec<usize> {
    let mut f: Vec<usize> = Vec::new();
    // C2: every indent 0..=32 (32 wraps to 0)
    for n in 0..=32usize {
        f.push(json_indent(n));
        f.push(n); // raw, unmasked — exercises the same 5 bits
    }
    // C3/C4/C5/C6/C7/C8/C10 single flags and pairs
    let bits = [
        JSON_COMPACT,
        JSON_ENSURE_ASCII,
        JSON_SORT_KEYS,
        JSON_PRESERVE_ORDER,
        JSON_ENCODE_ANY,
        JSON_ESCAPE_SLASH,
        JSON_EMBED,
    ];
    f.push(0);
    for b in bits {
        f.push(b);
        f.push(b | json_indent(4));
        for c in bits {
            f.push(b | c);
        }
    }
    // C9: every real precision 0..=32
    for n in 0..=32usize {
        f.push(json_real_precision(n));
        f.push(json_real_precision(n) | JSON_ENCODE_ANY);
    }
    // C11: random masks over the whole encoder bit space, incl. undefined bits
    let mut rng = Rng::new(0xC11);
    for _ in 0..300 {
        let mut m = 0usize;
        for b in [
            0x1F, JSON_COMPACT, JSON_ENSURE_ASCII, JSON_SORT_KEYS, JSON_PRESERVE_ORDER,
            JSON_ENCODE_ANY, JSON_ESCAPE_SLASH, 0xF800, JSON_EMBED, 1 << 20, 1 << 31,
        ] {
            if rng.bool() {
                m |= if b == 0x1F {
                    rng.below(32)
                } else if b == 0xF800 {
                    json_real_precision(rng.below(32))
                } else {
                    b
                };
            }
        }
        f.push(m);
    }
    f.sort_unstable();
    f.dedup();
    f
}

unsafe fn load_any(api: &Api, text: &str) -> Jt {
    unsafe {
        (api.json_loads)(
            cstr(text).as_ptr(),
            JSON_DECODE_ANY | JSON_ALLOW_NUL,
            std::ptr::null_mut(),
        )
    }
}

/// Documents whose indented encoding would be enormous (deep nesting / 1000
/// elements) — they still get exercised, but with a reduced mask set so the
/// test stays inside a sane memory budget.
fn is_heavy(d: &str) -> bool {
    d.len() > 400 || d.matches('[').count() > 40 || d.matches('{').count() > 40
}

/* =============== C1..C11: json_dumps over the whole flag matrix =============== */

#[test]
fn c1_c11_json_dumps_flag_matrix() {
    let _g = lock();
    let p = pair();
    let docs = documents();
    let masks = flag_masks();
    let light_masks: Vec<usize> = vec![
        0,
        json_indent(1),
        json_indent(4),
        JSON_COMPACT,
        JSON_SORT_KEYS,
        JSON_ENSURE_ASCII,
        JSON_ESCAPE_SLASH,
        JSON_EMBED,
        JSON_ENCODE_ANY,
    ];
    let mut compared = 0usize;
    unsafe {
        for d in &docs {
            let jc = load_any(p.c, d);
            let jr = load_any(p.r, d);
            assert_eq!(jc.is_null(), jr.is_null(), "load disagreement for {d:?}");
            if jc.is_null() {
                continue;
            }
            let use_masks: &[usize] = if is_heavy(d) { &light_masks } else { &masks };
            for m in use_masks {
                let a = dumps(p.c, jc, *m);
                let b = dumps(p.r, jr, *m);
                assert_eq!(
                    a,
                    b,
                    "json_dumps divergence: flags={m:#x} doc={:?}",
                    &d[..d.len().min(160)]
                );
                compared += 1;
            }
            decref(p.c, jc);
            decref(p.r, jr);
        }
    }
    assert!(compared > 50_000, "only {compared} comparisons");
}

/* =============== C7: JSON_ENCODE_ANY on every scalar type =============== */

#[test]
fn c7_encode_any_scalars() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        let mut out = Vec::new();
        unsafe {
            let vals: Vec<Jt> = vec![
                (api.json_null)(),
                (api.json_true)(),
                (api.json_false)(),
                (api.json_integer)(0),
                (api.json_integer)(i64::MIN),
                (api.json_integer)(i64::MAX),
                (api.json_real)(1.5),
                (api.json_real)(0.0),
                (api.json_real)(-0.0),
                (api.json_string)(cstr("").as_ptr()),
                (api.json_string)(cstr("hi/there").as_ptr()),
                (api.json_object)(),
                (api.json_array)(),
            ];
            for v in &vals {
                // without JSON_ENCODE_ANY the scalars must be rejected
                out.push((dumps(api, *v, 0), dumps(api, *v, JSON_ENCODE_ANY)));
                out.push((
                    dumps(api, *v, JSON_ENCODE_ANY | JSON_ESCAPE_SLASH),
                    dumps(api, *v, JSON_ENCODE_ANY | JSON_ENSURE_ASCII),
                ));
            }
            for v in vals {
                decref(api, v);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b);
}

/* =============== C12: json_dumpb size sweep =============== */

#[test]
fn c12_json_dumpb_size_sweep() {
    let _g = lock();
    let docs: Vec<String> = documents().into_iter().take(60).collect();
    let flags = [
        0usize,
        json_indent(2),
        JSON_COMPACT,
        JSON_SORT_KEYS,
        JSON_ENSURE_ASCII,
        JSON_EMBED,
    ];
    let script = |api: &'static Api| -> Vec<(usize, Vec<u8>)> {
        let mut out = Vec::new();
        unsafe {
            for d in &docs {
                let j = load_any(api, d);
                if j.is_null() {
                    out.push((usize::MAX, Vec::new()));
                    continue;
                }
                for f in flags {
                    let need = (api.json_dumpb)(j, std::ptr::null_mut(), 0, f);
                    for size in [
                        0usize,
                        1,
                        need.saturating_sub(1),
                        need,
                        need + 1,
                        need + 4096,
                    ] {
                        let mut buf = vec![0x7Ai8; size + 16];
                        let n = (api.json_dumpb)(j, buf.as_mut_ptr(), size, f);
                        out.push((n, buf.iter().map(|&c| c as u8).collect()));
                    }
                }
                decref(api, j);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "json_dumpb step {i}");
    }
}

/* =============== C13/C14/C15: FILE*, fd, and path sinks =============== */

#[test]
fn c13_c14_c15_file_sinks() {
    let _g = lock();
    let docs: Vec<String> = documents().into_iter().take(60).collect();
    let flags = [0usize, json_indent(3), JSON_COMPACT | JSON_SORT_KEYS, JSON_ENSURE_ASCII];
    let script = |api: &'static Api| -> Vec<(c_int, Vec<u8>)> {
        let libc = libc();
        let mut out = Vec::new();
        unsafe {
            for d in &docs {
                let j = load_any(api, d);
                if j.is_null() {
                    out.push((-99, Vec::new()));
                    continue;
                }
                for f in flags {
                    // json_dumpf via FILE*
                    let path = temp_path("dumpf");
                    let zp = cstr(path.to_str().unwrap());
                    let fp = (libc.fopen)(zp.as_ptr(), cstr("wb").as_ptr());
                    assert!(!fp.is_null());
                    let r = (api.json_dumpf)(j, fp, f);
                    (libc.fclose)(fp);
                    let mut content = Vec::new();
                    std::fs::File::open(&path).unwrap().read_to_end(&mut content).unwrap();
                    std::fs::remove_file(&path).ok();
                    out.push((r, content));

                    // json_dumpfd via a raw fd
                    let path = temp_path("dumpfd");
                    {
                        let file = std::fs::File::create(&path).unwrap();
                        let r = (api.json_dumpfd)(j, file.as_raw_fd(), f);
                        drop(file);
                        let mut content = Vec::new();
                        std::fs::File::open(&path).unwrap().read_to_end(&mut content).unwrap();
                        out.push((r, content));
                    }
                    std::fs::remove_file(&path).ok();

                    // json_dump_file via a path
                    let path = temp_path("dumpfile");
                    let zp = cstr(path.to_str().unwrap());
                    let r = (api.json_dump_file)(j, zp.as_ptr(), f);
                    let mut content = Vec::new();
                    if let Ok(mut fh) = std::fs::File::open(&path) {
                        fh.read_to_end(&mut content).unwrap();
                    }
                    std::fs::remove_file(&path).ok();
                    out.push((r, content));
                }
                decref(api, j);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "file sink step {i}");
    }
}

/* =============== C16: json_dump_callback, chunk boundaries included ======== */

static mut CHUNKS: Vec<Vec<u8>> = Vec::new();

unsafe extern "C" fn collect(buf: *const c_char, size: usize, _data: *mut c_void) -> c_int {
    unsafe {
        let s = std::slice::from_raw_parts(buf as *const u8, size).to_vec();
        (*std::ptr::addr_of_mut!(CHUNKS)).push(s);
    }
    0
}

#[test]
fn c16_dump_callback_chunk_boundaries() {
    let _g = lock();
    let docs: Vec<String> = documents().into_iter().take(80).collect();
    let flags = [
        0usize,
        json_indent(1),
        json_indent(31),
        JSON_COMPACT,
        JSON_SORT_KEYS,
        JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
        JSON_EMBED,
    ];
    let script = |api: &'static Api| -> Vec<(c_int, Vec<Vec<u8>>)> {
        let mut out = Vec::new();
        unsafe {
            for d in &docs {
                let j = load_any(api, d);
                if j.is_null() {
                    out.push((-99, Vec::new()));
                    continue;
                }
                for f in flags {
                    (*std::ptr::addr_of_mut!(CHUNKS)).clear();
                    let r = (api.json_dump_callback)(j, Some(collect), std::ptr::null_mut(), f);
                    out.push((r, (*std::ptr::addr_of!(CHUNKS)).clone()));
                }
                decref(api, j);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "dump_callback step {i}");
    }
}

/* =============== C9/C19: real formatting sweep =============== */

#[test]
fn c9_c19_real_precision_sweep() {
    let _g = lock();
    let mut rng = Rng::new(0xC919);
    let mut vals: Vec<f64> = vec![
        0.0, -0.0, 1.0, -1.0, 0.5, 0.1, 0.2, 0.3, 1e-5, -1e-5, 9.999e-5, 1e-4, 1e15,
        1e16, 1e17, -1e17, 1e300, -1e300, 1e-300, f64::MIN_POSITIVE, 5e-324, f64::MAX,
        f64::MIN, 3.141592653589793, 2.718281828459045, 1.0 / 3.0,
        123456789012345678.0, 0.30000000000000004, 9007199254740993.0,
    ];
    for _ in 0..600 {
        vals.push(rng.tame_f64());
    }
    for _ in 0..600 {
        vals.push(rng.finite_f64());
    }
    let script = move |api: &'static Api| -> Vec<u64> {
        let mut out = Vec::new();
        unsafe {
            for v in &vals {
                let j = (api.json_real)(*v);
                if j.is_null() {
                    out.push(u64::MAX);
                    continue;
                }
                for p in 0..=32usize {
                    let d = dumps(api, j, JSON_ENCODE_ANY | json_real_precision(p));
                    // fold to a digest so 40k comparisons stay cheap; the
                    // exact-bytes comparison happens in c1_c11 as well
                    let mut h: u64 = 1469598103934665603;
                    match d {
                        None => h = 0,
                        Some(b) => {
                            for x in b {
                                h ^= x as u64;
                                h = h.wrapping_mul(1099511628211);
                            }
                        }
                    }
                    out.push(h);
                }
                decref(api, j);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "real precision digest step {i}");
    }
}

/// The same sweep, but comparing the exact byte strings (smaller value set so
/// the full output can be held).
#[test]
fn c9_real_precision_exact_bytes() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0xC9EE);
    let mut vals: Vec<f64> = vec![
        0.0, -0.0, 1.0, -1.0, 0.5, 0.1, 1e-5, 1e-4, 1e16, 1e17, 1e300, 5e-324, f64::MAX,
        f64::MIN, 3.141592653589793, 1.0 / 3.0, 0.30000000000000004,
    ];
    for _ in 0..200 {
        vals.push(rng.tame_f64());
    }
    for _ in 0..200 {
        vals.push(rng.finite_f64());
    }
    unsafe {
        for v in &vals {
            let jc = (p.c.json_real)(*v);
            let jr = (p.r.json_real)(*v);
            assert_eq!(jc.is_null(), jr.is_null());
            if jc.is_null() {
                continue;
            }
            for prec in 0..=32usize {
                let f = JSON_ENCODE_ANY | json_real_precision(prec);
                assert_eq!(
                    dumps(p.c, jc, f),
                    dumps(p.r, jr, f),
                    "real {:?} bits={:#x} prec={prec}",
                    v,
                    v.to_bits()
                );
            }
            decref(p.c, jc);
            decref(p.r, jr);
        }
    }
}

//! Phase B — decoder / encoder (CONFIGS rows 38-67, 83, 84).
#![allow(unused_unsafe, dead_code, unsafe_op_in_unsafe_fn)]
mod common;
use common::*;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::raw::{c_char, c_int, c_void};

fn clip(s: &str, n: usize) -> String {
    let mut e = n.min(s.len());
    while e > 0 && !s.is_char_boundary(e) {
        e -= 1;
    }
    s[..e].to_string()
}

// -------------------------------------------------------------- flag battery

fn encode_flag_battery() -> Vec<usize> {
    let mut v = vec![
        0,
        JSON_COMPACT,
        JSON_ENSURE_ASCII,
        JSON_SORT_KEYS,
        JSON_ESCAPE_SLASH,
        JSON_PRESERVE_ORDER,
        JSON_ENCODE_ANY,
        JSON_EMBED,
        JSON_EMBED | json_indent(2),
        JSON_ENCODE_ANY | JSON_SORT_KEYS | JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
        JSON_ENCODE_ANY | JSON_COMPACT | json_indent(2),
    ];
    for n in 0..32 {
        v.push(JSON_ENCODE_ANY | json_indent(n));
    }
    for n in 0..32 {
        v.push(JSON_ENCODE_ANY | json_real_precision(n));
    }
    let mut rng = Rng::new(0xF1A65);
    for _ in 0..256 {
        v.push((rng.next_u64() as usize) & 0x1_07FF | JSON_ENCODE_ANY);
    }
    v
}

fn small_encode_battery() -> Vec<usize> {
    vec![
        JSON_ENCODE_ANY,
        JSON_ENCODE_ANY | JSON_COMPACT,
        JSON_ENCODE_ANY | JSON_SORT_KEYS,
        JSON_ENCODE_ANY | JSON_ENSURE_ASCII,
        JSON_ENCODE_ANY | JSON_ESCAPE_SLASH,
        JSON_ENCODE_ANY | json_indent(3),
        JSON_ENCODE_ANY | json_indent(2) | JSON_SORT_KEYS,
        JSON_ENCODE_ANY | JSON_EMBED,
        JSON_ENCODE_ANY | json_real_precision(5),
        JSON_ENCODE_ANY | json_real_precision(17),
        0,
    ]
}

/// Load `text` in both libs with `dflags` and, if both succeed, dump with every
/// flag word in `eflags`. Panics on any divergence.
unsafe fn diff_load_dump(text: &[u8], dflags: usize, eflags: &[usize], ctx: &str) {
    let (c, r) = both();
    let (jc, ec) = loads(c, text, dflags);
    let (jr, er) = loads(r, text, dflags);
    assert_eq!(
        jc.is_null(),
        jr.is_null(),
        "{}: load success differs (C null={}, RUST null={}) text={:?} dflags={:#x}\n C err: {} / code {}\n R err: {} / code {}",
        ctx,
        jc.is_null(),
        jr.is_null(),
        String::from_utf8_lossy(&text[..text.len().min(120)]),
        dflags,
        ec.text_str(),
        ec.code(),
        er.text_str(),
        er.code()
    );
    assert_err_eq(
        &format!(
            "{}: error struct differs text={:?} dflags={:#x}",
            ctx,
            String::from_utf8_lossy(&text[..text.len().min(120)]),
            dflags
        ),
        &ec,
        &er,
    );
    if jc.is_null() {
        return;
    }
    assert_eq!((*jc).typ, (*jr).typ, "{}: top-level type differs", ctx);
    for &ef in eflags {
        let dc = dumps(c, jc, ef);
        let dr = dumps(r, jr, ef);
        assert_bytes_eq(
            &format!(
                "{}: dump differs text={:?} dflags={:#x} eflags={:#x}",
                ctx,
                String::from_utf8_lossy(&text[..text.len().min(120)]),
                dflags,
                ef
            ),
            &dc,
            &dr,
        );
    }
    decref(c, jc);
    decref(r, jr);
}

// ---------------------------------------------------------- rows 38-44, 51-61

#[test]
fn row38_row51_default_flags() {
    let texts = corpus(60, 38);
    let eflags = encode_flag_battery();
    for t in &texts {
        unsafe { diff_load_dump(t.as_bytes(), 0, &eflags, "row38") };
    }
}

#[test]
fn row39_decode_any() {
    let texts = corpus(20, 39);
    let mut all: Vec<String> = texts;
    all.extend(
        [
            "1", "-1", "0", "1.5", "-0.0", "\"s\"", "\"\"", "true", "false", "null",
            "1e10", "9223372036854775807", "\"\\u00e9\"",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    let eflags = small_encode_battery();
    for t in &all {
        unsafe { diff_load_dump(t.as_bytes(), JSON_DECODE_ANY, &eflags, "row39") };
    }
}

#[test]
fn row40_decode_int_as_real() {
    let mut texts: Vec<String> = vec![
        "[0]".into(),
        "[1]".into(),
        "[-1]".into(),
        "[9223372036854775807]".into(),
        "[-9223372036854775808]".into(),
        "[9223372036854775808]".into(),
        "[-9223372036854775809]".into(),
        "[123456789012345678901234567890]".into(),
        "[1e10]".into(),
        "[1.5]".into(),
        "[0.0]".into(),
        "[1e400]".into(),
        "[1e-400]".into(),
    ];
    let mut rng = Rng::new(40);
    for _ in 0..2000 {
        texts.push(format!("[{}]", rng.i64()));
        texts.push(format!("[{}]", rng.below(1u64.wrapping_shl(40) as usize)));
    }
    texts.extend(corpus(20, 400));
    let eflags = small_encode_battery();
    for t in &texts {
        for f in [
            JSON_DECODE_INT_AS_REAL,
            JSON_DECODE_INT_AS_REAL | JSON_DECODE_ANY,
            0,
        ] {
            unsafe { diff_load_dump(t.as_bytes(), f, &eflags, "row40") };
        }
    }
}

#[test]
fn row41_reject_duplicates() {
    let texts: Vec<String> = vec![
        "{\"a\":1,\"a\":2}".into(),
        "{\"a\":1,\"b\":2}".into(),
        "{\"a\":1,\"b\":2,\"a\":3}".into(),
        "{\"\":1,\"\":2}".into(),
        "{\"a\":{\"b\":1,\"b\":2}}".into(),
        "[{\"a\":1},{\"a\":1}]".into(),
        "{\"\u{4e2d}\":1,\"\u{4e2d}\":2}".into(),
    ];
    let eflags = small_encode_battery();
    for t in &texts {
        for f in [0, JSON_REJECT_DUPLICATES] {
            unsafe { diff_load_dump(t.as_bytes(), f, &eflags, "row41") };
        }
    }
}

#[test]
fn row42_disable_eof_check() {
    let texts: Vec<String> = vec![
        "[1] trailing".into(),
        "[1]{}".into(),
        "{}[]".into(),
        "1 2".into(),
        "[1]\n\n".into(),
        "[1]x".into(),
        "nulll".into(),
        "truex".into(),
        "[1],".into(),
    ];
    let eflags = small_encode_battery();
    for t in &texts {
        for f in [
            0,
            JSON_DISABLE_EOF_CHECK,
            JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY,
        ] {
            unsafe { diff_load_dump(t.as_bytes(), f, &eflags, "row42") };
        }
    }
}

#[test]
fn row43_allow_nul() {
    let texts: Vec<&[u8]> = vec![
        b"[\"\\u0000\"]",
        b"[\"a\\u0000b\"]",
        b"[\"\\u0000\\u0000\"]",
        b"{\"\\u0000\":1}",
        b"{\"a\\u0000\":1}",
        b"[\"\\u0041\\u0000\\u0042\"]",
    ];
    let eflags = small_encode_battery();
    for t in &texts {
        for f in [0, JSON_ALLOW_NUL, JSON_ALLOW_NUL | JSON_DECODE_ANY] {
            unsafe { diff_load_dump(t, f, &eflags, "row43") };
        }
    }
}

#[test]
fn row44_all_decode_flag_combos() {
    let texts = corpus(30, 44);
    let mut all = texts;
    all.extend(invalid_corpus());
    let eflags = small_encode_battery();
    for f in all_decode_flag_combos() {
        for t in &all {
            unsafe { diff_load_dump(t.as_bytes(), f, &eflags, "row44") };
        }
    }
}

#[test]
fn row44b_invalid_utf8_inputs() {
    let eflags = small_encode_battery();
    for f in all_decode_flag_combos() {
        for t in invalid_utf8_corpus() {
            unsafe { diff_load_dump(&t, f, &eflags, "row44b") };
        }
    }
}

// ------------------------------------------------------------------ row 45

#[test]
fn row45_loadb_lengths() {
    let (c, r) = both();
    let texts = corpus(20, 45);
    let eflags = small_encode_battery();
    for t in &texts {
        let b = t.as_bytes();
        let mut lens: Vec<usize> = vec![0, b.len()];
        if b.len() > 1 {
            lens.push(b.len() - 1);
            lens.push(b.len() / 2);
            lens.push(1);
        }
        for len in lens {
            for f in [0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK] {
                unsafe { diff_load_dump(&b[..len], f, &eflags, "row45") };
            }
        }
    }
    // buflen larger than the terminator: NUL byte inside the buffer
    let payload = b"[1]\0[2]";
    for f in [0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK] {
        unsafe { diff_load_dump(payload, f, &eflags, "row45-nul") };
    }
    let _ = (c, r);
}

// ------------------------------------------------------- rows 46, 47, 48, 49

fn tmp_path(tag: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "jansson_difftest_{}_{}_{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    p
}

#[test]
fn row46_row47_row48_file_loaders() {
    let (c, r) = both();
    let mut texts = corpus(20, 46);
    texts.extend(invalid_corpus().into_iter().take(30));
    let eflags = small_encode_battery();

    for t in &texts {
        for &dflags in &[0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK, 0x1F] {
            let path = tmp_path("in");
            std::fs::write(&path, t.as_bytes()).unwrap();
            let cpath = cstr(path.to_str().unwrap());

            // json_load_file
            let mut out = Vec::new();
            for l in [c, r] {
                let f = sym!(l, "json_load_file", (*const c_char, usize, *mut JsonError) -> *mut JsonT);
                let mut e = JsonError::new();
                let j = unsafe { f(cpath.as_ptr(), dflags, &mut e) };
                let d = if j.is_null() {
                    None
                } else {
                    let d = unsafe { dumps(l, j, JSON_ENCODE_ANY | JSON_SORT_KEYS) };
                    unsafe { decref(l, j) };
                    d
                };
                out.push((d, e));
            }
            assert_bytes_eq(
                &format!("json_load_file dump {:?} dflags={:#x}", t, dflags),
                &out[0].0,
                &out[1].0,
            );
            assert_err_eq(
                &format!("json_load_file error {:?} dflags={:#x}", t, dflags),
                &out[0].1,
                &out[1].1,
            );

            // json_loadf (FILE*)
            let mut out = Vec::new();
            for l in [c, r] {
                let fopen = sym!(l, "json_loadf", (*mut c_void, usize, *mut JsonError) -> *mut JsonT);
                let mode = cstr("rb");
                let fp = unsafe { libc_fopen(cpath.as_ptr(), mode.as_ptr()) };
                assert!(!fp.is_null());
                let mut e = JsonError::new();
                let j = unsafe { fopen(fp, dflags, &mut e) };
                let d = if j.is_null() {
                    None
                } else {
                    let d = unsafe { dumps(l, j, JSON_ENCODE_ANY | JSON_SORT_KEYS) };
                    unsafe { decref(l, j) };
                    d
                };
                unsafe { libc_fclose(fp) };
                out.push((d, e));
            }
            assert_bytes_eq(
                &format!("json_loadf dump {:?} dflags={:#x}", t, dflags),
                &out[0].0,
                &out[1].0,
            );
            assert_err_eq(
                &format!("json_loadf error {:?} dflags={:#x}", t, dflags),
                &out[0].1,
                &out[1].1,
            );

            // json_loadfd (raw fd)
            let mut out = Vec::new();
            for l in [c, r] {
                let f = sym!(l, "json_loadfd", (c_int, usize, *mut JsonError) -> *mut JsonT);
                let mode = cstr("rb");
                let fp = unsafe { libc_fopen(cpath.as_ptr(), mode.as_ptr()) };
                let fd = unsafe { libc_fileno(fp) };
                let mut e = JsonError::new();
                let j = unsafe { f(fd, dflags, &mut e) };
                let d = if j.is_null() {
                    None
                } else {
                    let d = unsafe { dumps(l, j, JSON_ENCODE_ANY | JSON_SORT_KEYS) };
                    unsafe { decref(l, j) };
                    d
                };
                unsafe { libc_fclose(fp) };
                out.push((d, e));
            }
            assert_bytes_eq(
                &format!("json_loadfd dump {:?} dflags={:#x}", t, dflags),
                &out[0].0,
                &out[1].0,
            );
            assert_err_eq(
                &format!("json_loadfd error {:?} dflags={:#x}", t, dflags),
                &out[0].1,
                &out[1].1,
            );

            // json_loads (NUL-terminated) where possible
            if !t.as_bytes().contains(&0) {
                let cs = cstr(t);
                let mut out = Vec::new();
                for l in [c, r] {
                    let f = sym!(l, "json_loads", (*const c_char, usize, *mut JsonError) -> *mut JsonT);
                    let mut e = JsonError::new();
                    let j = unsafe { f(cs.as_ptr(), dflags, &mut e) };
                    let d = if j.is_null() {
                        None
                    } else {
                        let d = unsafe { dumps(l, j, JSON_ENCODE_ANY | JSON_SORT_KEYS) };
                        unsafe { decref(l, j) };
                        d
                    };
                    out.push((d, e));
                }
                assert_bytes_eq(
                    &format!("json_loads dump {:?} dflags={:#x}", t, dflags),
                    &out[0].0,
                    &out[1].0,
                );
                assert_err_eq(
                    &format!("json_loads error {:?} dflags={:#x}", t, dflags),
                    &out[0].1,
                    &out[1].1,
                );
            }
            let _ = std::fs::remove_file(&path);
        }
    }
    let _ = eflags;
}

// ------------------------------------------------------------------ row 49

#[repr(C)]
struct LoadSrc {
    data: Vec<u8>,
    pos: usize,
    chunk: usize,
    calls: usize,
}

unsafe extern "C" fn load_cb(buffer: *mut c_void, buflen: usize, data: *mut c_void) -> usize {
    let s = unsafe { &mut *(data as *mut LoadSrc) };
    s.calls += 1;
    let avail = s.data.len() - s.pos;
    let n = buflen.min(s.chunk).min(avail);
    if n > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(s.data.as_ptr().add(s.pos), buffer as *mut u8, n)
        };
    }
    s.pos += n;
    n
}

#[test]
fn row49_load_callback() {
    let (c, r) = both();
    let mut texts = corpus(20, 49);
    texts.extend(invalid_corpus().into_iter().take(30));
    for t in &texts {
        for chunk in [1usize, 3, 7, 64, 1 << 20] {
            for dflags in [0usize, JSON_DECODE_ANY, 0x1F] {
                let mut out = Vec::new();
                for l in [c, r] {
                    let f = sym!(l, "json_load_callback",
                        (unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize, *mut c_void, usize, *mut JsonError) -> *mut JsonT);
                    let mut src = LoadSrc {
                        data: t.as_bytes().to_vec(),
                        pos: 0,
                        chunk,
                        calls: 0,
                    };
                    let mut e = JsonError::new();
                    let j = unsafe {
                        f(
                            load_cb,
                            &mut src as *mut LoadSrc as *mut c_void,
                            dflags,
                            &mut e,
                        )
                    };
                    let d = if j.is_null() {
                        None
                    } else {
                        let d = unsafe { dumps(l, j, JSON_ENCODE_ANY | JSON_SORT_KEYS) };
                        unsafe { decref(l, j) };
                        d
                    };
                    out.push((d, e, src.pos, src.calls));
                }
                assert_bytes_eq(
                    &format!("json_load_callback dump {:?} chunk={}", t, chunk),
                    &out[0].0,
                    &out[1].0,
                );
                assert_err_eq(
                    &format!("json_load_callback error {:?} chunk={}", t, chunk),
                    &out[0].1,
                    &out[1].1,
                );
                assert_eq!(
                    (out[0].2, out[0].3),
                    (out[1].2, out[1].3),
                    "json_load_callback consumption differs {:?} chunk={} dflags={:#x}",
                    t,
                    chunk,
                    dflags
                );
            }
        }
    }
}

// ------------------------------------------------------------------ row 50

#[test]
fn row50_deep_nesting() {
    let eflags = vec![
        JSON_ENCODE_ANY,
        JSON_ENCODE_ANY | JSON_COMPACT,
        JSON_ENCODE_ANY | json_indent(1),
    ];
    for d in [
        1usize, 2, 3, 10, 31, 32, 33, 100, 500, 1000, 2040, 2046, 2047, 2048, 2049, 2050,
        3000,
    ] {
        let a = format!("{}1{}", "[".repeat(d), "]".repeat(d));
        let o = format!("{}1{}", "{\"a\":".repeat(d), "}".repeat(d));
        let mixed: String = {
            let mut s = String::new();
            let mut close = String::new();
            for i in 0..d {
                if i % 2 == 0 {
                    s.push('[');
                    close.insert(0, ']');
                } else {
                    s.push_str("{\"k\":");
                    close.insert(0, '}');
                }
            }
            s.push('1');
            s.push_str(&close);
            s
        };
        for t in [a, o, mixed] {
            unsafe { diff_load_dump(t.as_bytes(), JSON_DECODE_ANY, &eflags, "row50") };
        }
    }
}

// ------------------------------------------------------------- rows 52-61

#[test]
fn row52_to_row61_encoder_flags() {
    let (c, r) = both();
    let texts = corpus(80, 52);
    let eflags = encode_flag_battery();
    for t in &texts {
        let (jc, _) = unsafe { loads(c, t.as_bytes(), JSON_DECODE_ANY) };
        let (jr, _) = unsafe { loads(r, t.as_bytes(), JSON_DECODE_ANY) };
        assert_eq!(jc.is_null(), jr.is_null());
        if jc.is_null() {
            continue;
        }
        for &ef in &eflags {
            let dc = unsafe { dumps(c, jc, ef) };
            let dr = unsafe { dumps(r, jr, ef) };
            assert_bytes_eq(
                &format!("encoder flags {:#x} on {:?}", ef, clip(t, 80)),
                &dc,
                &dr,
            );
        }
        unsafe {
            decref(c, jc);
            decref(r, jr);
        }
    }
}

// ------------------------------------------------------------------ row 62

#[test]
fn row62_dumpb() {
    let (c, r) = both();
    let texts = corpus(40, 62);
    for t in &texts {
        let (jc, _) = unsafe { loads(c, t.as_bytes(), JSON_DECODE_ANY) };
        let (jr, _) = unsafe { loads(r, t.as_bytes(), JSON_DECODE_ANY) };
        if jc.is_null() || jr.is_null() {
            assert_eq!(jc.is_null(), jr.is_null());
            continue;
        }
        for &ef in &[
            JSON_ENCODE_ANY,
            JSON_ENCODE_ANY | JSON_COMPACT,
            JSON_ENCODE_ANY | json_indent(2),
            JSON_ENCODE_ANY | JSON_SORT_KEYS,
        ] {
            // exact size first
            let need = unsafe { dumps(c, jc, ef) }.map(|v| v.len()).unwrap_or(0);
            for size in [0usize, 1, need / 2, need.saturating_sub(1), need, need + 1, need + 16] {
                let mut out = Vec::new();
                for (l, j) in [(c, jc), (r, jr)] {
                    let f = sym!(l, "json_dumpb", (*const JsonT, *mut c_char, usize, usize) -> usize);
                    let mut buf = vec![0xAAu8; size + 32];
                    let n = unsafe { f(j, buf.as_mut_ptr() as *mut c_char, size, ef) };
                    buf.truncate(size + 32);
                    out.push((n, buf));
                }
                assert_eq!(
                    out[0], out[1],
                    "json_dumpb size={} eflags={:#x} text={:?}",
                    size,
                    ef,
                    clip(t, 60)
                );
            }
        }
        unsafe {
            decref(c, jc);
            decref(r, jr);
        }
    }
}

// ------------------------------------------------------------- rows 63-65

unsafe extern "C" {
    #[link_name = "fopen"]
    fn libc_fopen(p: *const c_char, m: *const c_char) -> *mut c_void;
    #[link_name = "fclose"]
    fn libc_fclose(f: *mut c_void) -> c_int;
    #[link_name = "fileno"]
    fn libc_fileno(f: *mut c_void) -> c_int;
    #[link_name = "fflush"]
    fn libc_fflush(f: *mut c_void) -> c_int;
}

#[test]
fn row63_row64_row65_file_dumpers() {
    let (c, r) = both();
    let texts = corpus(30, 63);
    let eflags = [
        JSON_ENCODE_ANY,
        JSON_ENCODE_ANY | JSON_COMPACT,
        JSON_ENCODE_ANY | json_indent(4),
        JSON_ENCODE_ANY | JSON_SORT_KEYS | JSON_ENSURE_ASCII,
    ];
    for t in &texts {
        let (jc, _) = unsafe { loads(c, t.as_bytes(), JSON_DECODE_ANY) };
        let (jr, _) = unsafe { loads(r, t.as_bytes(), JSON_DECODE_ANY) };
        if jc.is_null() || jr.is_null() {
            assert_eq!(jc.is_null(), jr.is_null());
            continue;
        }
        for &ef in &eflags {
            // json_dump_file
            let mut out = Vec::new();
            for (l, j) in [(c, jc), (r, jr)] {
                let path = tmp_path("out");
                let cp = cstr(path.to_str().unwrap());
                let f = sym!(l, "json_dump_file", (*const JsonT, *const c_char, usize) -> c_int);
                let rc = unsafe { f(j, cp.as_ptr(), ef) };
                let body = std::fs::read(&path).unwrap_or_default();
                let _ = std::fs::remove_file(&path);
                out.push((rc, body));
            }
            assert_eq!(out[0], out[1], "json_dump_file eflags={:#x}", ef);

            // json_dumpf
            let mut out = Vec::new();
            for (l, j) in [(c, jc), (r, jr)] {
                let path = tmp_path("outf");
                let cp = cstr(path.to_str().unwrap());
                let mode = cstr("wb");
                let fp = unsafe { libc_fopen(cp.as_ptr(), mode.as_ptr()) };
                let f = sym!(l, "json_dumpf", (*const JsonT, *mut c_void, usize) -> c_int);
                let rc = unsafe { f(j, fp, ef) };
                unsafe { libc_fclose(fp) };
                let body = std::fs::read(&path).unwrap_or_default();
                let _ = std::fs::remove_file(&path);
                out.push((rc, body));
            }
            assert_eq!(out[0], out[1], "json_dumpf eflags={:#x}", ef);

            // json_dumpfd
            let mut out = Vec::new();
            for (l, j) in [(c, jc), (r, jr)] {
                let path = tmp_path("outfd");
                let cp = cstr(path.to_str().unwrap());
                let mode = cstr("wb");
                let fp = unsafe { libc_fopen(cp.as_ptr(), mode.as_ptr()) };
                let fd = unsafe { libc_fileno(fp) };
                let f = sym!(l, "json_dumpfd", (*const JsonT, c_int, usize) -> c_int);
                let rc = unsafe { f(j, fd, ef) };
                unsafe { libc_fflush(fp) };
                unsafe { libc_fclose(fp) };
                let body = std::fs::read(&path).unwrap_or_default();
                let _ = std::fs::remove_file(&path);
                out.push((rc, body));
            }
            assert_eq!(out[0], out[1], "json_dumpfd eflags={:#x}", ef);
        }
        unsafe {
            decref(c, jc);
            decref(r, jr);
        }
    }
}

// ------------------------------------------------------------------ row 66

unsafe extern "C" fn dump_cb(buf: *const c_char, size: usize, data: *mut c_void) -> c_int {
    let sink = unsafe { &mut *(data as *mut Vec<(usize, Vec<u8>)>) };
    let bytes = if buf.is_null() || size == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(buf as *const u8, size).to_vec() }
    };
    sink.push((size, bytes));
    0
}

#[test]
fn row66_dump_callback_chunking() {
    let (c, r) = both();
    let texts = corpus(40, 66);
    let eflags = [
        JSON_ENCODE_ANY,
        JSON_ENCODE_ANY | JSON_COMPACT,
        JSON_ENCODE_ANY | json_indent(2),
        JSON_ENCODE_ANY | JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
        JSON_ENCODE_ANY | JSON_SORT_KEYS,
        JSON_ENCODE_ANY | JSON_EMBED,
    ];
    for t in &texts {
        let (jc, _) = unsafe { loads(c, t.as_bytes(), JSON_DECODE_ANY) };
        let (jr, _) = unsafe { loads(r, t.as_bytes(), JSON_DECODE_ANY) };
        if jc.is_null() || jr.is_null() {
            assert_eq!(jc.is_null(), jr.is_null());
            continue;
        }
        for &ef in &eflags {
            let mut out = Vec::new();
            for (l, j) in [(c, jc), (r, jr)] {
                let f = sym!(l, "json_dump_callback",
                    (*const JsonT, unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int, *mut c_void, usize) -> c_int);
                let mut sink: Vec<(usize, Vec<u8>)> = Vec::new();
                let rc = unsafe {
                    f(
                        j,
                        dump_cb,
                        &mut sink as *mut Vec<(usize, Vec<u8>)> as *mut c_void,
                        ef,
                    )
                };
                out.push((rc, sink));
            }
            assert_eq!(
                out[0], out[1],
                "json_dump_callback chunking eflags={:#x} text={:?}",
                ef,
                clip(t, 60)
            );
        }
        unsafe {
            decref(c, jc);
            decref(r, jr);
        }
    }
}

// --------------------------------------------------------------- rows 67, 83

#[test]
fn row67_roundtrip() {
    let (c, r) = both();
    let texts = corpus(25, 67);
    let dflags = all_decode_flag_combos();
    let eflags = [
        JSON_ENCODE_ANY,
        JSON_ENCODE_ANY | JSON_COMPACT,
        JSON_ENCODE_ANY | json_indent(2),
        JSON_ENCODE_ANY | JSON_SORT_KEYS,
        JSON_ENCODE_ANY | JSON_ENSURE_ASCII,
        JSON_ENCODE_ANY | JSON_ESCAPE_SLASH | JSON_SORT_KEYS,
    ];
    for t in &texts {
        for &df in &dflags {
            for &ef in &eflags {
                let a = unsafe {
                    let (j, _) = loads(c, t.as_bytes(), df);
                    if j.is_null() {
                        None
                    } else {
                        let d = dumps(c, j, ef);
                        decref(c, j);
                        d
                    }
                };
                let b = unsafe {
                    let (j, _) = loads(r, t.as_bytes(), df);
                    if j.is_null() {
                        None
                    } else {
                        let d = dumps(r, j, ef);
                        decref(r, j);
                        d
                    }
                };
                assert_bytes_eq(
                    &format!("roundtrip1 df={:#x} ef={:#x} {:?}", df, ef, clip(t, 60)),
                    &a,
                    &b,
                );
                // second generation
                if let (Some(ta), Some(tb)) = (&a, &b) {
                    assert_eq!(ta, tb);
                    unsafe { diff_load_dump(ta, df, &eflags, "roundtrip2") };
                }
            }
        }
    }
}

#[test]
fn row83_real_roundtrip() {
    let (c, r) = both();
    let mut rng = Rng::new(83);
    let mut vals: Vec<f64> = interesting_reals().to_vec();
    for _ in 0..4096 {
        vals.push(rng.f64());
    }
    for _ in 0..2048 {
        let m = rng.below(60) as i32 - 30;
        vals.push((rng.below(1_000_000) as f64) * 10f64.powi(m));
    }
    for &v in &vals {
        for prec in [0usize, 1, 2, 5, 10, 15, 16, 17, 18, 31] {
            let ef = JSON_ENCODE_ANY | json_real_precision(prec);
            let mut out = Vec::new();
            for l in [c, r] {
                unsafe {
                    let real = sym!(l, "json_real", (f64) -> *mut JsonT);
                    let j = real(v);
                    if j.is_null() {
                        out.push(None);
                        continue;
                    }
                    let d = dumps(l, j, ef);
                    decref(l, j);
                    // reload the dumped text and dump again
                    let d2 = d.as_ref().and_then(|txt| {
                        let (j2, _) = loads(l, txt, JSON_DECODE_ANY);
                        if j2.is_null() {
                            None
                        } else {
                            let x = dumps(l, j2, ef);
                            decref(l, j2);
                            x
                        }
                    });
                    out.push(Some((d, d2)));
                }
            }
            assert_eq!(out[0], out[1], "real roundtrip {:e} prec={}", v, prec);
        }
    }
}

// ------------------------------------------------------------------ row 84

#[test]
fn row84_mixed_pipeline() {
    let (c, r) = both();
    let mut rng = Rng::new(84);
    for trial in 0..400 {
        let v = rand_value(&mut rng, 5);
        let ef1 = JSON_ENCODE_ANY | (rng.next_u64() as usize & 0x1_07FF);
        let df = rng.next_u64() as usize & 0x1F;
        let ef2 = JSON_ENCODE_ANY | (rng.next_u64() as usize & 0x1_07FF);
        let mut out = Vec::new();
        for l in [c, r] {
            unsafe {
                let j = build_value(l, &v);
                let d1 = dumps(l, j, ef1);
                decref(l, j);
                let d2 = d1.as_ref().and_then(|t| {
                    let (j2, e) = loads(l, t, df);
                    if j2.is_null() {
                        Some((None, e.raw()))
                    } else {
                        let x = dumps(l, j2, ef2);
                        decref(l, j2);
                        Some((x, e.raw()))
                    }
                });
                out.push((d1, d2));
            }
        }
        assert_eq!(
            out[0], out[1],
            "mixed pipeline trial {} ef1={:#x} df={:#x} ef2={:#x}",
            trial, ef1, df, ef2
        );
    }
}

unsafe fn build_value(l: &Lib, v: &V) -> *mut JsonT {
    match v {
        V::Null => sym!(l, "json_null", () -> *mut JsonT)(),
        V::Bool(true) => sym!(l, "json_true", () -> *mut JsonT)(),
        V::Bool(false) => sym!(l, "json_false", () -> *mut JsonT)(),
        V::Int(i) => sym!(l, "json_integer", (i64) -> *mut JsonT)(*i),
        V::Real(f) => sym!(l, "json_real", (f64) -> *mut JsonT)(*f),
        V::Str(s) => sym!(l, "json_stringn", (*const c_char, usize) -> *mut JsonT)(
            s.as_ptr() as *const c_char,
            s.len(),
        ),
        V::Arr(a) => {
            let arr = sym!(l, "json_array", () -> *mut JsonT)();
            let app = sym!(l, "json_array_append_new", (*mut JsonT, *mut JsonT) -> c_int);
            for e in a {
                app(arr, build_value(l, e));
            }
            arr
        }
        V::Obj(m) => {
            let obj = sym!(l, "json_object", () -> *mut JsonT)();
            let set = sym!(l, "json_object_setn_new", (*mut JsonT, *const c_char, usize, *mut JsonT) -> c_int);
            for (k, e) in m {
                set(obj, k.as_ptr() as *const c_char, k.len(), build_value(l, e));
            }
            obj
        }
    }
}

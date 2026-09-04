//! Phase B — CONFIGS.md section D: the full decoder configuration matrix over
//! every public load entry point (`json_loads`, `json_loadb`, `json_loadf`,
//! `json_loadfd`, `json_load_file`, `json_load_callback`).
mod common;

use common::*;
use std::io::Write;
use std::os::raw::{c_char, c_void};
use std::os::unix::io::AsRawFd;

/// Everything the parser is driven with: the shared corpus plus decoder-specific
/// shapes (escapes, multi-byte UTF-8, number forms, whitespace, depth limits).
fn inputs() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = corpus().into_iter().map(|s| s.into_bytes()).collect();
    // D2 scalars at top level (need JSON_DECODE_ANY)
    for s in [
        "1", "-1", "0", "1.5", "\"str\"", "true", "false", "null", "1e5", "\"\"",
    ] {
        v.push(s.as_bytes().to_vec());
    }
    // D3 int-as-real
    v.push(b"[1,2,3]".to_vec());
    v.push(b"[9223372036854775807]".to_vec());
    v.push(b"[9223372036854775808]".to_vec()); // out of int range
    v.push(b"[-9223372036854775809]".to_vec());
    // D4 trailing content
    v.push(b"[] trailing".to_vec());
    v.push(b"[]   ".to_vec());
    v.push(b"{}{}".to_vec());
    v.push(b"[1]x".to_vec());
    // D5 duplicates
    v.push(b"{\"a\":1,\"a\":2}".to_vec());
    v.push(b"{\"a\":1,\"b\":2,\"a\":3}".to_vec());
    // D6 NUL in strings
    v.push(b"[\"a\\u0000b\"]".to_vec());
    v.push(b"[\"\\u0000\"]".to_vec());
    v.push(b"{\"k\":\"\\u0000\"}".to_vec());
    v.push(b"{\"a\\u0000b\":1}".to_vec());
    // D13 escapes
    v.push(b"[\"\\\"\\\\\\/\\b\\f\\n\\r\\t\"]".to_vec());
    v.push(b"[\"\\u0041\\u00e9\\u20ac\\uD834\\uDD1E\"]".to_vec());
    v.push(b"[\"\\uD834\"]".to_vec());
    v.push(b"[\"\\uDD1E\"]".to_vec());
    v.push(b"[\"\\uD834\\u0041\"]".to_vec());
    v.push(b"[\"\\uZZZZ\"]".to_vec());
    v.push(b"[\"\\x\"]".to_vec());
    // D14 raw multi-byte UTF-8 and invalid UTF-8
    for s in [
        "[\"é\"]",
        "[\"€\"]",
        "[\"𝄞\"]",
        "{\"ключ\":\"значение\"}",
        "[\"中文\u{1F600}\"]",
        "[\"a\nb\"]",
        "[1,\n2,\n@]",
        "[\n\n\n\"unterminated",
    ] {
        v.push(s.as_bytes().to_vec());
    }
    // D15 numbers
    for n in [
        "0", "-0", "1", "-1", "01", "1.", "1.e2", ".1", "1e", "1e+", "1e-", "1E10",
        "1e-10", "-1.5e-3", "1e999", "-1e999", "1e-999", "12345678901234567890",
        "-12345678901234567890", "0.1", "1.7976931348623157e308", "5e-324",
        "3.141592653589793", "9007199254740993",
    ] {
        v.push(format!("[{n}]").into_bytes());
    }
    // D16 depth limits
    v.push(format!("{}{}", "[".repeat(2048), "]".repeat(2048)).into_bytes());
    v.push(format!("{}{}", "[".repeat(2049), "]".repeat(2049)).into_bytes());
    v.push(format!("{}1{}", "[".repeat(2047), "]".repeat(2047)).into_bytes());
    // D17 whitespace
    v.push(b" \t\r\n{\r\n\"a\"\t:\n1\r,\n\"b\"\t:\r[\n]\n}\t ".to_vec());
    // malformed shapes (raw bytes: some are deliberately invalid UTF-8)
    let malformed: &[&[u8]] = &[
        b"",
        b" ",
        b"[",
        b"]",
        b"{",
        b"}",
        b"[,]",
        b"[:]",
        b"[}]",
        b"{]}",
        b"{1:2}",
        b"{\"a\" 1}",
        b"{\"a\":}",
        b"{\"a\":1",
        b"{\"a\":1,",
        b"[1",
        b"[1,",
        b"[1,,2]",
        b"tru",
        b"nul",
        b"fals",
        b"TRUE",
        b"@",
        b"\x01",
        b"\"unterminated",
        b"[\"a\x01b\"]",
        b"\xff",
        b"\xc2",
        b"[\"\xc2\"]",
        b"[\"\xc2\x41\"]",
        b"[\"\xed\xa0\x80\"]",
        b"[\"\xf5\x80\x80\x80\"]",
        b"[\"\xf0\x80\x80\x80\"]",
        b"[\"\xe0\x80\x80\"]",
        b"[\"\x80\"]",
        b"\xef\xbb\xbf{}",
    ];
    for s in malformed {
        v.push(s.to_vec());
    }
    let mut rng = Rng::new(0xD0C5);
    for _ in 0..200 {
        v.push(random_json(&mut rng, 4, true).into_bytes());
    }
    // random mutations of valid documents (mostly invalid)
    let base: Vec<String> = (0..120).map(|_| random_json(&mut rng, 3, false)).collect();
    for b in base {
        let mut bytes = b.into_bytes();
        if bytes.is_empty() {
            continue;
        }
        let i = rng.below(bytes.len());
        bytes[i] = (rng.next_u64() & 0x7F) as u8;
        if bytes[i] == 0 {
            bytes[i] = b'?';
        }
        v.push(bytes);
    }
    v
}

/// All 32 combinations of the 5 decoder flags plus a few undefined bits.
fn decoder_flag_sets() -> Vec<usize> {
    let mut v: Vec<usize> = (0..32usize).collect(); // bits 0x1..0x10 exactly
    v.push(1 << 20);
    v.push(0x1F | (1 << 31));
    v
}

/// An observation of one load call: return, error struct, and the canonical
/// re-encoding of the result (so the whole parsed tree is compared).
#[derive(PartialEq, Debug)]
struct Obs {
    null: bool,
    ty: i64,
    err: (i32, i32, i32, Vec<u8>, Vec<u8>),
    dump: Option<Vec<u8>>,
    dump_sorted: Option<Vec<u8>>,
}

unsafe fn observe(api: &Api, j: Jt, e: &JsonError) -> Obs {
    unsafe {
        Obs {
            null: j.is_null(),
            ty: if j.is_null() { -1 } else { (*j).type_ as i64 },
            err: e.snapshot(),
            dump: dumps(api, j, JSON_ENCODE_ANY),
            dump_sorted: dumps(api, j, JSON_ENCODE_ANY | JSON_SORT_KEYS | json_indent(2)),
        }
    }
}

/* ================= D1..D7, D13..D18: json_loads flag matrix ================= */

#[test]
fn d1_d7_json_loads_flag_matrix() {
    let _g = lock();
    let p = pair();
    let texts = inputs();
    let flags = decoder_flag_sets();
    let mut n = 0usize;
    unsafe {
        for t in &texts {
            let z = nul_terminated(t);
            for f in &flags {
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let jc = (p.c.json_loads)(z.as_ptr(), *f, &mut ec);
                let jr = (p.r.json_loads)(z.as_ptr(), *f, &mut er);
                let oc = observe(p.c, jc, &ec);
                let or = observe(p.r, jr, &er);
                assert_eq!(
                    oc, or,
                    "json_loads divergence: flags={f:#x} input={:?}",
                    String::from_utf8_lossy(&t[..t.len().min(120)])
                );
                decref(p.c, jc);
                decref(p.r, jr);
                n += 1;
            }
        }
    }
    assert!(n > 10_000, "only {n} comparisons");
}

/* ================= D8: json_loadb buflen sweep ================= */

#[test]
fn d8_json_loadb_buflen() {
    let _g = lock();
    let p = pair();
    let texts = inputs();
    unsafe {
        for t in &texts {
            if t.len() > 80 { continue; }
            let bytes: &[u8] = t;
            let z = nul_terminated(bytes);
            let mut lens: Vec<usize> = vec![0, bytes.len()];
            if bytes.len() > 0 {
                lens.push(1);
                lens.push(bytes.len() / 2);
                lens.push(bytes.len() - 1);
            }
            lens.push(bytes.len() + 1); // includes the NUL
            for f in [0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK, JSON_ALLOW_NUL] {
                for l in &lens {
                    let mut ec = JsonError::zeroed();
                    let mut er = JsonError::zeroed();
                    let jc = (p.c.json_loadb)(z.as_ptr(), *l, f, &mut ec);
                    let jr = (p.r.json_loadb)(z.as_ptr(), *l, f, &mut er);
                    assert_eq!(
                        observe(p.c, jc, &ec),
                        observe(p.r, jr, &er),
                        "json_loadb({:?}, buflen={l}, flags={f:#x})", String::from_utf8_lossy(t)
                    );
                    decref(p.c, jc);
                    decref(p.r, jr);
                }
            }
        }
    }
}

/* ================= D9/D10/D11: FILE*, fd and path sources ================= */

#[test]
fn d9_d10_d11_file_sources() {
    let _g = lock();
    let p = pair();
    let libc = libc();
    let texts: Vec<Vec<u8>> = inputs().into_iter().filter(|t| t.len() <= 4096).collect();
    unsafe {
        for t in &texts {
            let path = temp_path("load");
            std::fs::File::create(&path)
                .unwrap()
                .write_all(t)
                .unwrap();
            let zp = cstr(path.to_str().unwrap());
            for f in [
                0usize,
                JSON_DECODE_ANY,
                JSON_DISABLE_EOF_CHECK,
                JSON_REJECT_DUPLICATES,
                JSON_ALLOW_NUL | JSON_DECODE_ANY,
            ] {
                // D9 json_loadf
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let fc = (libc.fopen)(zp.as_ptr(), cstr("rb").as_ptr());
                let jc = (p.c.json_loadf)(fc, f, &mut ec);
                (libc.fclose)(fc);
                let fr = (libc.fopen)(zp.as_ptr(), cstr("rb").as_ptr());
                let jr = (p.r.json_loadf)(fr, f, &mut er);
                (libc.fclose)(fr);
                assert_eq!(
                    observe(p.c, jc, &ec),
                    observe(p.r, jr, &er),
                    "json_loadf({:?}, flags={f:#x})", String::from_utf8_lossy(t)
                );
                decref(p.c, jc);
                decref(p.r, jr);

                // D10 json_loadfd
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let hc = std::fs::File::open(&path).unwrap();
                let jc = (p.c.json_loadfd)(hc.as_raw_fd(), f, &mut ec);
                drop(hc);
                let hr = std::fs::File::open(&path).unwrap();
                let jr = (p.r.json_loadfd)(hr.as_raw_fd(), f, &mut er);
                drop(hr);
                assert_eq!(
                    observe(p.c, jc, &ec),
                    observe(p.r, jr, &er),
                    "json_loadfd({:?}, flags={f:#x})", String::from_utf8_lossy(t)
                );
                decref(p.c, jc);
                decref(p.r, jr);

                // D11 json_load_file — note the error `source` is the path
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let jc = (p.c.json_load_file)(zp.as_ptr(), f, &mut ec);
                let jr = (p.r.json_load_file)(zp.as_ptr(), f, &mut er);
                assert_eq!(
                    observe(p.c, jc, &ec),
                    observe(p.r, jr, &er),
                    "json_load_file({:?}, flags={f:#x})", String::from_utf8_lossy(t)
                );
                decref(p.c, jc);
                decref(p.r, jr);
            }
            std::fs::remove_file(&path).ok();
        }
    }
}

/* ================= D12: json_load_callback ================= */

struct CbState {
    data: Vec<u8>,
    pos: usize,
    chunk: usize,
}

unsafe extern "C" fn feeder(buf: *mut c_void, buflen: usize, data: *mut c_void) -> usize {
    unsafe {
        let st = &mut *(data as *mut CbState);
        if st.pos >= st.data.len() {
            return 0;
        }
        let n = buflen.min(st.chunk).min(st.data.len() - st.pos);
        std::ptr::copy_nonoverlapping(st.data[st.pos..].as_ptr(), buf as *mut u8, n);
        st.pos += n;
        n
    }
}

#[test]
fn d12_json_load_callback() {
    let _g = lock();
    let p = pair();
    let mut texts: Vec<Vec<u8>> = inputs().into_iter().filter(|t| t.len() <= 4096).collect();
    // A document larger than MAX_BUF_LEN (1024) to cross the refill boundary.
    texts.push(
        format!(
            "[{}]",
            (0..400).map(|i| format!("\"item{i:04}\"")).collect::<Vec<_>>().join(",")
        )
        .into_bytes(),
    );
    unsafe {
        for t in &texts {
            for chunk in [1usize, 7, 1023, 1024, 4096] {
                for f in [0usize, JSON_DECODE_ANY, JSON_ALLOW_NUL | JSON_DECODE_ANY] {
                    let mut sc = CbState {
                        data: t.clone(),
                        pos: 0,
                        chunk,
                    };
                    let mut sr = CbState {
                        data: t.clone(),
                        pos: 0,
                        chunk,
                    };
                    let mut ec = JsonError::zeroed();
                    let mut er = JsonError::zeroed();
                    let jc = (p.c.json_load_callback)(
                        Some(feeder),
                        &mut sc as *mut CbState as *mut c_void,
                        f,
                        &mut ec,
                    );
                    let jr = (p.r.json_load_callback)(
                        Some(feeder),
                        &mut sr as *mut CbState as *mut c_void,
                        f,
                        &mut er,
                    );
                    assert_eq!(
                        observe(p.c, jc, &ec),
                        observe(p.r, jr, &er),
                        "json_load_callback({:?}, chunk={chunk}, flags={f:#x})",
                        String::from_utf8_lossy(&t[..t.len().min(80)])
                    );
                    assert_eq!(sc.pos, sr.pos, "callback consumed a different amount");
                    decref(p.c, jc);
                    decref(p.r, jr);
                }
            }
        }
    }
}

/* ================= D18: error struct on success ================= */

#[test]
fn d18_error_struct_on_success() {
    let _g = lock();
    let p = pair();
    unsafe {
        for t in corpus() {
            let z = cstr(&t);
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let jc = (p.c.json_loads)(z.as_ptr(), 0, &mut ec);
            let jr = (p.r.json_loads)(z.as_ptr(), 0, &mut er);
            assert_eq!(jc.is_null(), jr.is_null());
            assert_eq!(ec.snapshot(), er.snapshot(), "error struct after success: {t:?}");
            assert_eq!(ec.position, er.position);
            assert_eq!(ec.source_str(), "<string>");
            decref(p.c, jc);
            decref(p.r, jr);
        }
        // and with a NULL error pointer (must not crash on either side)
        for t in corpus() {
            let z = cstr(&t);
            let jc = (p.c.json_loads)(z.as_ptr(), 0, std::ptr::null_mut());
            let jr = (p.r.json_loads)(z.as_ptr(), 0, std::ptr::null_mut());
            assert_eq!(dumps(p.c, jc, JSON_ENCODE_ANY), dumps(p.r, jr, JSON_ENCODE_ANY));
            decref(p.c, jc);
            decref(p.r, jr);
        }
    }
}

/* ================= D19/D20: round trip ================= */

#[test]
fn d19_d20_round_trip() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0xD19);
    let mut texts = corpus();
    for _ in 0..400 {
        texts.push(random_json(&mut rng, 5, true));
    }
    unsafe {
        for t in &texts {
            let z = cstr(t);
            for f in [0usize, json_indent(2), JSON_SORT_KEYS, JSON_COMPACT, JSON_ENSURE_ASCII] {
                let jc = (p.c.json_loads)(z.as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
                let jr = (p.r.json_loads)(z.as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
                assert_eq!(jc.is_null(), jr.is_null());
                if jc.is_null() {
                    continue;
                }
                let dc = dumps(p.c, jc, f | JSON_ENCODE_ANY).unwrap();
                let dr = dumps(p.r, jr, f | JSON_ENCODE_ANY).unwrap();
                assert_eq!(dc, dr, "dump after load: {t:?} flags={f:#x}");
                // second generation
                let z2c = nul_terminated(&dc);
                let z2r = nul_terminated(&dr);
                let j2c = (p.c.json_loads)(z2c.as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
                let j2r = (p.r.json_loads)(z2r.as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
                assert_eq!(
                    dumps(p.c, j2c, f | JSON_ENCODE_ANY),
                    dumps(p.r, j2r, f | JSON_ENCODE_ANY),
                    "second-generation round trip: {t:?}"
                );
                assert_eq!((p.c.json_equal)(jc, j2c), (p.r.json_equal)(jr, j2r));
                decref(p.c, j2c);
                decref(p.r, j2r);
                decref(p.c, jc);
                decref(p.r, jr);
            }
        }
    }
}

#[allow(unused)]
fn _unused(_: *const c_char) {}

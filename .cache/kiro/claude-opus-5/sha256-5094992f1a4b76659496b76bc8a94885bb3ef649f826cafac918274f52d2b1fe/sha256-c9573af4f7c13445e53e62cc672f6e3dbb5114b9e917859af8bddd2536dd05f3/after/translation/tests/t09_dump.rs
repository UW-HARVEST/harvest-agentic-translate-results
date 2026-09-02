//! Phase B/C — dump.c: every encoder flag, every sink, every escape.
//! CONFIGS rows 53-72 · ERRORS rows 128-146.
mod common;
use common::*;
use std::ffi::{CStr, c_char, c_int, c_void};

unsafe extern "C" {
    fn tmpfile() -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn rewind(f: *mut c_void);
    fn fread(p: *mut c_void, sz: usize, n: usize, f: *mut c_void) -> usize;
    fn fileno(f: *mut c_void) -> c_int;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn dup(fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
}

/// The full set of encoder flags the C branches on (CONFIGS header).
fn encoder_flag_axes() -> Vec<usize> {
    let mut v = vec![
        0,
        JSON_COMPACT,
        JSON_ENSURE_ASCII,
        JSON_SORT_KEYS,
        JSON_PRESERVE_ORDER,
        JSON_ESCAPE_SLASH,
        JSON_ENCODE_ANY,
        JSON_EMBED,
    ];
    for n in [1usize, 2, 4, 8, 31] {
        v.push(json_indent(n));
    }
    for p in [1usize, 5, 17, 31] {
        v.push(json_real_precision(p));
    }
    v
}

/// A fixed corpus of documents, as JSON text (parsed by both libraries).
fn corpus() -> Vec<String> {
    vec![
        "{}".into(),
        "[]".into(),
        "[1]".into(),
        "[1,2,3]".into(),
        r#"{"a":1}"#.into(),
        r#"{"a":1,"b":2,"c":3}"#.into(),
        r#"{"z":1,"y":2,"a":3,"aa":4,"A":5,"":6}"#.into(),
        r#"[[[[[1]]]]]"#.into(),
        r#"{"o":{"o":{"o":{}}},"a":[[],[[]]]}"#.into(),
        r#"[null,true,false,0,-0,1,-1,1.5,-1.5,1e10,1e-10]"#.into(),
        r#"["","a","\u0000x","\t\n\r\b\f\\\"/","\u00e9","\u20ac","\ud83d\ude00"]"#.into(),
        r#"{"k\u0000ey":1}"#.into(),
        r#"[9223372036854775807,-9223372036854775808]"#.into(),
        r#"{"nested":{"arr":[{"x":1},{"y":[1,2,{"z":null}]}]}}"#.into(),
        r#"["\u0001\u0002\u001f","\u007f","\uffff"]"#.into(),
        r#"{"dup":1,"dup2":2,"dup":3}"#.into(),
    ]
}

unsafe fn load_pair(text: &str, flags: usize) -> (*mut JsonT, *mut JsonT) {
    unsafe {
        let t = cs(text);
        let cj = (c().json_loads)(t.as_ptr(), flags, std::ptr::null_mut());
        let rj = (r().json_loads)(t.as_ptr(), flags, std::ptr::null_mut());
        assert_eq!(
            cj.is_null(),
            rj.is_null(),
            "setup: json_loads({text:?}, {flags:#x}) null-ness"
        );
        (cj, rj)
    }
}

/* ---- CONFIGS 53-66: json_dumps across the full flag surface ---- */

#[test]
fn dumps_single_flag_axes() {
    let _g = dtoa_guard();
    unsafe {
        for text in corpus() {
            let (cj, rj) = load_pair(&text, JSON_ALLOW_NUL | JSON_DECODE_ANY);
            if cj.is_null() {
                continue;
            }
            for f in encoder_flag_axes() {
                let flags = f | JSON_ENCODE_ANY;
                assert_bytes_eq(
                    &format!("json_dumps({text:?}, {flags:#x})"),
                    &dumps(c(), cj, flags),
                    &dumps(r(), rj, flags),
                );
            }
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}

#[test]
fn dumps_all_flag_pairs_and_triples() {
    let _g = dtoa_guard();
    unsafe {
        let axes = encoder_flag_axes();
        for text in corpus() {
            let (cj, rj) = load_pair(&text, JSON_ALLOW_NUL | JSON_DECODE_ANY);
            if cj.is_null() {
                continue;
            }
            for (i, &a) in axes.iter().enumerate() {
                for &b in &axes[i..] {
                    for &e in &[0usize, JSON_ENCODE_ANY] {
                        let flags = a | b | e;
                        assert_bytes_eq(
                            &format!("json_dumps({text:?}, {flags:#x})"),
                            &dumps(c(), cj, flags),
                            &dumps(r(), rj, flags),
                        );
                    }
                }
            }
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}

/* ---- CONFIGS 66: randomized flag bit-vectors over the full mask ---- */

#[test]
fn dumps_randomized_flag_vectors() {
    let _g = dtoa_guard();
    unsafe {
        let mut rng = Rng::new(0xD0_0001);
        // Every bit the encoder can look at, plus reserved bits (ERRORS 249/250).
        let mask = 0x1F | JSON_COMPACT | JSON_ENSURE_ASCII | JSON_SORT_KEYS
            | JSON_PRESERVE_ORDER | JSON_ENCODE_ANY | JSON_ESCAPE_SLASH
            | (0x1F << 11) | JSON_EMBED;
        for trial in 0..4000 {
            let text = gen_json(&mut rng, 4);
            let (cj, rj) = load_pair(&text, JSON_DECODE_ANY);
            if cj.is_null() {
                continue;
            }
            for _ in 0..6 {
                let mut flags = (rng.next_u64() as usize) & mask;
                if rng.below(8) == 0 {
                    // reserved / unknown bits must be ignored identically
                    flags |= (rng.next_u64() as usize) & !mask;
                }
                assert_bytes_eq(
                    &format!("trial {trial}: json_dumps({text:?}, {flags:#x})"),
                    &dumps(c(), cj, flags),
                    &dumps(r(), rj, flags),
                );
            }
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}

/* ---- CONFIGS 59: SORT_KEYS with prefix-equal keys (compare_keys tiebreak) ---- */

#[test]
fn dumps_sort_keys_prefix_tiebreak() {
    let _g = dtoa_guard();
    unsafe {
        let mut rng = Rng::new(0xD0_0002);
        // deliberately many prefix relationships: "", "a", "aa", "aaa", ...
        let mut key_sets: Vec<Vec<String>> = vec![
            vec!["".into(), "a".into(), "aa".into(), "aaa".into(), "ab".into(), "b".into()],
            vec!["z".into(), "zz".into(), "zzz".into(), "y".into()],
            (0..64).map(|i| "x".repeat(i % 8 + 1) + &i.to_string()).collect(),
        ];
        for _ in 0..300 {
            let n = rng.below(40);
            let mut ks = Vec::new();
            let mut seen = std::collections::BTreeSet::new();
            for _ in 0..n {
                let base = rng.key(4);
                let k = base.repeat(1 + rng.below(3));
                if seen.insert(k.clone()) {
                    ks.push(k);
                }
            }
            key_sets.push(ks);
        }
        for ks in key_sets {
            let co = (c().json_object)();
            let ro = (r().json_object)();
            for (i, k) in ks.iter().enumerate() {
                let kc = cs(k);
                (c().json_object_set_new_nocheck)(co, kc.as_ptr(), (c().json_integer)(i as i64));
                (r().json_object_set_new_nocheck)(ro, kc.as_ptr(), (r().json_integer)(i as i64));
            }
            for extra in [0usize, JSON_COMPACT, json_indent(2), JSON_ENSURE_ASCII] {
                let flags = JSON_SORT_KEYS | extra;
                assert_bytes_eq(
                    &format!("SORT_KEYS flags={flags:#x} keys={ks:?}"),
                    &dumps(c(), co, flags),
                    &dumps(r(), ro, flags),
                );
            }
            decref(c(), co);
            decref(r(), ro);
        }
    }
}

/* ---- CONFIGS 72 · ERRORS 134: string escaping ---- */

#[test]
fn dumps_string_escaping_all_codepoints() {
    let _g = dtoa_guard();
    unsafe {
        // Every codepoint from 0x00 to 0x2FF, plus BMP and non-BMP boundaries.
        let mut cps: Vec<u32> = (0u32..0x300).collect();
        cps.extend([
            0x7F, 0x80, 0x7FF, 0x800, 0xFFF, 0xFFFD, 0xFFFF, 0x1_0000, 0x1_0001,
            0x1_F600, 0x10_FFFE, 0x10_FFFF,
        ]);
        for cp in cps {
            if (0xD800..=0xDFFF).contains(&cp) {
                continue; // not encodable
            }
            let ch = char::from_u32(cp).unwrap();
            let mut s = String::new();
            s.push('<');
            if cp != 0 {
                s.push(ch);
            }
            s.push('>');
            if cp == 0 {
                continue; // interior NUL handled by the byte-level case below
            }
            let sc = cs(&s);
            let cj = (c().json_string)(sc.as_ptr());
            let rj = (r().json_string)(sc.as_ptr());
            assert_eq!(cj.is_null(), rj.is_null(), "json_string(U+{cp:04X})");
            if cj.is_null() {
                continue;
            }
            for extra in [
                0usize,
                JSON_ENSURE_ASCII,
                JSON_ESCAPE_SLASH,
                JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
                JSON_COMPACT,
            ] {
                let flags = JSON_ENCODE_ANY | extra;
                assert_bytes_eq(
                    &format!("dumps(U+{cp:04X}, {flags:#x})"),
                    &dumps(c(), cj, flags),
                    &dumps(r(), rj, flags),
                );
            }
            decref(c(), cj);
            decref(r(), rj);
        }

        // interior NUL and the mandatory-escape set, via explicit lengths
        let payloads: Vec<Vec<u8>> = vec![
            b"a\0b".to_vec(),
            b"\0".to_vec(),
            b"\"\\/\x08\x0c\n\r\t".to_vec(),
            (0u8..0x20).collect(),
            b"/slash/".to_vec(),
        ];
        for p in payloads {
            let buf = cbytes(&p);
            let cj = (c().json_stringn_nocheck)(buf.as_ptr() as *const c_char, p.len());
            let rj = (r().json_stringn_nocheck)(buf.as_ptr() as *const c_char, p.len());
            for extra in [0usize, JSON_ENSURE_ASCII, JSON_ESCAPE_SLASH] {
                let flags = JSON_ENCODE_ANY | extra;
                assert_bytes_eq(
                    &format!("dumps({p:02x?}, {flags:#x})"),
                    &dumps(c(), cj, flags),
                    &dumps(r(), rj, flags),
                );
            }
            decref(c(), cj);
            decref(r(), rj);
        }

        // ERRORS 134: invalid UTF-8 payload (reachable via json_string_nocheck)
        for bad in [
            vec![0xC2u8],
            vec![0xE2, 0x82],
            vec![0xFF, 0xFE],
            vec![0x80],
            vec![0xF5, 0x80, 0x80, 0x80],
            vec![0xED, 0xA0, 0x80],
        ] {
            let buf = cbytes(&bad);
            let cj = (c().json_stringn_nocheck)(buf.as_ptr() as *const c_char, bad.len());
            let rj = (r().json_stringn_nocheck)(buf.as_ptr() as *const c_char, bad.len());
            assert!(!cj.is_null() && !rj.is_null());
            let cd = dumps(c(), cj, JSON_ENCODE_ANY);
            let rd = dumps(r(), rj, JSON_ENCODE_ANY);
            assert_bytes_eq(&format!("ERRORS 134: dumps({bad:02x?})"), &cd, &rd);
            assert!(cd.is_none(), "invalid UTF-8 must fail to dump");
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}

/* ---- CONFIGS 61 · ERRORS 128: ENCODE_ANY gate ---- */

#[test]
fn dump_callback_encode_any_gate() {
    let _g = dtoa_guard();
    unsafe {
        for api in both() {
            let scalars: Vec<*mut JsonT> = vec![
                (api.json_string)(cs("s").as_ptr()),
                (api.json_integer)(1),
                (api.json_real)(1.5),
                (api.json_true)(),
                (api.json_false)(),
                (api.json_null)(),
            ];
            for &p in &scalars {
                // ERRORS 128: without ENCODE_ANY, scalar roots are rejected
                assert!(
                    (api.json_dumps)(p, 0).is_null(),
                    "{}: scalar root without ENCODE_ANY",
                    api.tag
                );
                assert_eq!((api.json_dumpb)(p, std::ptr::null_mut(), 0, 0), 0);
                assert_eq!((api.json_dumpfd)(p, -1, 0), -1);
                assert_eq!(
                    (api.json_dump_callback)(p, Some(count_cb), std::ptr::null_mut(), 0),
                    -1
                );
                // with ENCODE_ANY it works
                let d = (api.json_dumps)(p, JSON_ENCODE_ANY);
                assert!(!d.is_null());
                (api.jsonp_free)(d as *mut c_void);
            }
            // ERRORS 128/129: NULL root
            assert!((api.json_dumps)(std::ptr::null(), 0).is_null());
            assert!((api.json_dumps)(std::ptr::null(), JSON_ENCODE_ANY).is_null());
            assert_eq!(
                (api.json_dump_callback)(
                    std::ptr::null(),
                    Some(count_cb),
                    std::ptr::null_mut(),
                    JSON_ENCODE_ANY
                ),
                -1
            );
            for p in scalars {
                decref(api, p);
            }
        }
    }
}

unsafe extern "C" fn count_cb(_b: *const c_char, _n: usize, _d: *mut c_void) -> c_int {
    0
}

/* ---- ERRORS 130: out-of-range type in do_dump ---- */

#[test]
fn dump_out_of_range_type() {
    let _g = dtoa_guard();
    unsafe {
        let mut results = Vec::new();
        for api in both() {
            let mut row = Vec::new();
            for bad in [8i32, 9, 42, 255, -1, i32::MAX, i32::MIN] {
                let p = (api.json_integer)(1);
                (*p).type_ = bad;
                row.push((api.json_dumps)(p, JSON_ENCODE_ANY).is_null());
                row.push((api.json_dumps)(p, 0).is_null());
                // nested inside a valid array
                let a = (api.json_array)();
                (*p).type_ = JSON_INTEGER;
                (api.json_array_append_new)(a, p);
                (*p).type_ = bad;
                row.push((api.json_dumps)(a, 0).is_null());
                (*p).type_ = JSON_INTEGER;
                decref(api, a);
            }
            results.push(row);
        }
        assert_eq!(results[0], results[1], "ERRORS 130: bad json_type in do_dump");
        assert!(results[0].iter().all(|&x| x), "all must be rejected");
    }
}

/* ---- ERRORS 133: circular references ---- */

#[test]
fn dump_circular_references() {
    let _g = dtoa_guard();
    unsafe {
        let mut results = Vec::new();
        for api in both() {
            let mut row: Vec<Option<Vec<u8>>> = Vec::new();
            // array -> array cycle
            let a = (api.json_array)();
            let b = (api.json_array)();
            (api.json_array_append_new)(a, incref(b));
            (api.json_array_append_new)(b, incref(a));
            row.push(dumps(api, a, 0));
            row.push(dumps(api, b, json_indent(2)));
            // object -> object cycle
            let o1 = (api.json_object)();
            let o2 = (api.json_object)();
            (api.json_object_set_new_nocheck)(o1, cs("o2").as_ptr(), incref(o2));
            (api.json_object_set_new_nocheck)(o2, cs("o1").as_ptr(), incref(o1));
            row.push(dumps(api, o1, 0));
            row.push(dumps(api, o1, JSON_SORT_KEYS));
            // deeper cycle: a -> [x] -> a
            let m = (api.json_array)();
            let inner = (api.json_object)();
            (api.json_object_set_new_nocheck)(inner, cs("back").as_ptr(), incref(m));
            (api.json_array_append_new)(m, inner);
            row.push(dumps(api, m, 0));
            // diamond (shared, acyclic) must SUCCEED and be dumped twice
            let leaf = (api.json_array)();
            (api.json_array_append_new)(leaf, (api.json_integer)(7));
            let d = (api.json_array)();
            (api.json_array_append_new)(d, incref(leaf));
            (api.json_array_append_new)(d, incref(leaf));
            row.push(dumps(api, d, 0));
            let od = (api.json_object)();
            (api.json_object_set_new_nocheck)(od, cs("l1").as_ptr(), incref(leaf));
            (api.json_object_set_new_nocheck)(od, cs("l2").as_ptr(), incref(leaf));
            row.push(dumps(api, od, 0));
            row.push(dumps(api, od, JSON_SORT_KEYS));
            results.push(row);
        }
        assert_eq!(results[0], results[1], "ERRORS 133: circular reference dumps");
        // the first five must be rejected, the diamonds must succeed
        for i in 0..5 {
            assert!(results[0][i].is_none(), "cycle #{i} must be rejected");
        }
        // flags == 0 is neither COMPACT nor indented, so items are separated
        // by ", " (dump_indent with space=1).
        assert_eq!(results[0][5].as_deref(), Some(&b"[[7], [7]]"[..]));
    }
}

/* ---- CONFIGS 67 · ERRORS 139, 140: json_dumpb ---- */

#[test]
fn json_dumpb_buffer_sizes() {
    let _g = dtoa_guard();
    unsafe {
        for text in corpus() {
            let (cj, rj) = load_pair(&text, JSON_ALLOW_NUL | JSON_DECODE_ANY);
            if cj.is_null() {
                continue;
            }
            for flags in [0usize, JSON_COMPACT, json_indent(2), JSON_SORT_KEYS] {
                let f = flags | JSON_ENCODE_ANY;
                // required size, with a zero-size buffer
                let creq = (c().json_dumpb)(cj, std::ptr::null_mut(), 0, f);
                let rreq = (r().json_dumpb)(rj, std::ptr::null_mut(), 0, f);
                assert_eq!(creq, rreq, "json_dumpb required size ({text:?}, {f:#x})");
                for size in [
                    0usize,
                    1,
                    creq / 2,
                    creq.saturating_sub(1),
                    creq,
                    creq + 1,
                    creq + 100,
                ] {
                    let mut cbuf = vec![0x5Au8; size + 8];
                    let mut rbuf = vec![0x5Au8; size + 8];
                    let cn = (c().json_dumpb)(cj, cbuf.as_mut_ptr() as *mut c_char, size, f);
                    let rn = (r().json_dumpb)(rj, rbuf.as_mut_ptr() as *mut c_char, size, f);
                    assert_eq!(cn, rn, "json_dumpb({text:?}, size={size}, {f:#x}) used");
                    assert_eq!(
                        cbuf, rbuf,
                        "json_dumpb({text:?}, size={size}, {f:#x}) buffer bytes"
                    );
                    if size >= creq && creq > 0 {
                        assert_eq!(&cbuf[..cn], &rbuf[..rn]);
                    }
                }
            }
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}

/* ---- CONFIGS 68 · ERRORS 136, 141: json_dumpf ---- */

#[test]
fn json_dumpf_to_tmpfile() {
    let _g = dtoa_guard();
    unsafe {
        for text in corpus() {
            let (cj, rj) = load_pair(&text, JSON_ALLOW_NUL | JSON_DECODE_ANY);
            if cj.is_null() {
                continue;
            }
            for flags in [0usize, JSON_COMPACT | JSON_SORT_KEYS, json_indent(4)] {
                let f = flags | JSON_ENCODE_ANY;
                let cout = read_via_file(c().json_dumpf, cj, f);
                let rout = read_via_file(r().json_dumpf, rj, f);
                assert_eq!(cout, rout, "json_dumpf({text:?}, {f:#x})");
                // must equal json_dumps
                if let (Some((rc, bytes)), Some(d)) = (&cout, dumps(c(), cj, f)) {
                    assert_eq!(*rc, 0);
                    assert_eq!(bytes, &d);
                }
            }
            decref(c(), cj);
            decref(r(), rj);
        }

        // ERRORS 136/141: fwrite failure. A stream opened read-only makes
        // fwrite() fail immediately, unlike closing the fd behind a buffered
        // stream (which glibc only notices at flush time).
        let mut rcs = Vec::new();
        for api in both() {
            let a = (api.json_array)();
            (api.json_array_append_new)(a, (api.json_integer)(1));
            let path = cs("/dev/null");
            let mode = cs("r");
            let fp = fopen(path.as_ptr(), mode.as_ptr());
            assert!(!fp.is_null());
            let rc = (api.json_dumpf)(a, fp, 0);
            fclose(fp);
            rcs.push(rc);
            decref(api, a);
        }
        assert_eq!(rcs[0], rcs[1], "ERRORS 136/141: fwrite failure");
        assert_eq!(rcs[0], -1, "read-only stream must fail");
        let _ = (dup as usize, close as usize, fileno as usize);
    }
}

type DumpfFn = unsafe extern "C" fn(*const JsonT, *mut c_void, usize) -> c_int;

unsafe fn read_via_file(f: DumpfFn, j: *const JsonT, flags: usize) -> Option<(c_int, Vec<u8>)> {
    unsafe {
        let fp = tmpfile();
        if fp.is_null() {
            return None;
        }
        let rc = f(j, fp, flags);
        fflush(fp);
        rewind(fp);
        let mut buf = vec![0u8; 1 << 20];
        let n = fread(buf.as_mut_ptr() as *mut c_void, 1, buf.len(), fp);
        buf.truncate(n);
        fclose(fp);
        Some((rc, buf))
    }
}

/* ---- CONFIGS 69 · ERRORS 137, 142: json_dumpfd ---- */

#[test]
fn json_dumpfd_to_fd() {
    let _g = dtoa_guard();
    unsafe {
        for text in corpus() {
            let (cj, rj) = load_pair(&text, JSON_ALLOW_NUL | JSON_DECODE_ANY);
            if cj.is_null() {
                continue;
            }
            for flags in [0usize, JSON_COMPACT, json_indent(1)] {
                let f = flags | JSON_ENCODE_ANY;
                let cout = read_via_fd(c().json_dumpfd, cj, f);
                let rout = read_via_fd(r().json_dumpfd, rj, f);
                assert_eq!(cout, rout, "json_dumpfd({text:?}, {f:#x})");
            }
            decref(c(), cj);
            decref(r(), rj);
        }
        // ERRORS 137/142: invalid fd
        for api in both() {
            let a = (api.json_array)();
            (api.json_array_append_new)(a, (api.json_integer)(1));
            assert_eq!((api.json_dumpfd)(a, -1, 0), -1, "{}: fd = -1", api.tag);
            assert_eq!((api.json_dumpfd)(a, 999_999, 0), -1, "{}: bogus fd", api.tag);
            decref(api, a);
        }
    }
}

type DumpfdFn = unsafe extern "C" fn(*const JsonT, c_int, usize) -> c_int;

unsafe fn read_via_fd(f: DumpfdFn, j: *const JsonT, flags: usize) -> Option<(c_int, Vec<u8>)> {
    unsafe {
        let fp = tmpfile();
        if fp.is_null() {
            return None;
        }
        let fd = fileno(fp);
        let rc = f(j, fd, flags);
        lseek(fd, 0, 0);
        let mut buf = vec![0u8; 1 << 20];
        let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
        buf.truncate(if n > 0 { n as usize } else { 0 });
        fclose(fp);
        Some((rc, buf))
    }
}

/* ---- CONFIGS 70 · ERRORS 143, 144: json_dump_file ---- */

#[test]
fn json_dump_file_roundtrip_and_errors() {
    let _g = dtoa_guard();
    unsafe {
        let dir = std::env::temp_dir();
        for (i, text) in corpus().iter().enumerate() {
            let (cj, rj) = load_pair(text, JSON_ALLOW_NUL | JSON_DECODE_ANY);
            if cj.is_null() {
                continue;
            }
            for flags in [0usize, JSON_COMPACT | JSON_SORT_KEYS, json_indent(2)] {
                let f = flags | JSON_ENCODE_ANY;
                let cpath = dir.join(format!("jansson_c_{i}_{f}.json"));
                let rpath = dir.join(format!("jansson_r_{i}_{f}.json"));
                let cps = cs(cpath.to_str().unwrap());
                let rps = cs(rpath.to_str().unwrap());
                let crc = (c().json_dump_file)(cj, cps.as_ptr(), f);
                let rrc = (r().json_dump_file)(rj, rps.as_ptr(), f);
                assert_eq!(crc, rrc, "json_dump_file rc ({text:?}, {f:#x})");
                let cb = std::fs::read(&cpath).unwrap_or_default();
                let rb = std::fs::read(&rpath).unwrap_or_default();
                assert_eq!(cb, rb, "json_dump_file contents ({text:?}, {f:#x})");
                let _ = std::fs::remove_file(&cpath);
                let _ = std::fs::remove_file(&rpath);
            }
            decref(c(), cj);
            decref(r(), rj);
        }
        // ERRORS 143: fopen failure
        for api in both() {
            let a = (api.json_array)();
            (api.json_array_append_new)(a, (api.json_integer)(1));
            let bad = cs("/nonexistent-dir-xyz/deeper/out.json");
            assert_eq!((api.json_dump_file)(a, bad.as_ptr(), 0), -1);
            let dirpath = cs("/");
            assert_eq!((api.json_dump_file)(a, dirpath.as_ptr(), 0), -1);
            decref(api, a);
        }
    }
}

/* ---- CONFIGS 71 · ERRORS 135: json_dump_callback ---- */

struct Sink {
    chunks: Vec<Vec<u8>>,
    fail_after: usize,
    calls: usize,
}

unsafe extern "C" fn sink_cb(b: *const c_char, n: usize, d: *mut c_void) -> c_int {
    unsafe {
        let s = &mut *(d as *mut Sink);
        s.calls += 1;
        if s.calls > s.fail_after {
            return -1;
        }
        s.chunks
            .push(std::slice::from_raw_parts(b as *const u8, n).to_vec());
        0
    }
}

#[test]
fn json_dump_callback_chunks_and_failure() {
    let _g = dtoa_guard();
    unsafe {
        for text in corpus() {
            let (cj, rj) = load_pair(&text, JSON_ALLOW_NUL | JSON_DECODE_ANY);
            if cj.is_null() {
                continue;
            }
            for flags in [
                0usize,
                JSON_COMPACT,
                json_indent(2),
                JSON_SORT_KEYS,
                JSON_EMBED,
                JSON_EMBED | json_indent(2),
            ] {
                let f = flags | JSON_ENCODE_ANY;
                let mut cs_ = Sink { chunks: vec![], fail_after: usize::MAX, calls: 0 };
                let mut rs_ = Sink { chunks: vec![], fail_after: usize::MAX, calls: 0 };
                let crc = (c().json_dump_callback)(
                    cj,
                    Some(sink_cb),
                    &mut cs_ as *mut Sink as *mut c_void,
                    f,
                );
                let rrc = (r().json_dump_callback)(
                    rj,
                    Some(sink_cb),
                    &mut rs_ as *mut Sink as *mut c_void,
                    f,
                );
                assert_eq!(crc, rrc, "dump_callback rc ({text:?}, {f:#x})");
                // The exact chunk boundaries are part of the observable
                // behaviour of the callback API, so compare them, not just the
                // concatenation.
                assert_eq!(
                    cs_.chunks, rs_.chunks,
                    "dump_callback chunk sequence ({text:?}, {f:#x})"
                );
                assert_eq!(cs_.calls, rs_.calls, "dump_callback call count");

                // ERRORS 135: make the callback fail at each call index
                for k in 0..cs_.calls.min(12) {
                    let mut cf = Sink { chunks: vec![], fail_after: k, calls: 0 };
                    let mut rf = Sink { chunks: vec![], fail_after: k, calls: 0 };
                    let c2 = (c().json_dump_callback)(
                        cj,
                        Some(sink_cb),
                        &mut cf as *mut Sink as *mut c_void,
                        f,
                    );
                    let r2 = (r().json_dump_callback)(
                        rj,
                        Some(sink_cb),
                        &mut rf as *mut Sink as *mut c_void,
                        f,
                    );
                    assert_eq!(c2, r2, "dump_callback fail@{k} rc ({text:?}, {f:#x})");
                    assert_eq!(cf.chunks, rf.chunks, "dump_callback fail@{k} chunks");
                    assert_eq!(cf.calls, rf.calls, "dump_callback fail@{k} calls");
                }
            }
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}

/* ---- CONFIGS 64, 65: JSON_EMBED ---- */

#[test]
fn dumps_embed_flag() {
    let _g = dtoa_guard();
    unsafe {
        for text in corpus() {
            let (cj, rj) = load_pair(&text, JSON_ALLOW_NUL | JSON_DECODE_ANY);
            if cj.is_null() {
                continue;
            }
            for extra in [
                0usize,
                json_indent(2),
                JSON_COMPACT,
                JSON_SORT_KEYS,
                JSON_ENSURE_ASCII,
                json_indent(4) | JSON_SORT_KEYS,
            ] {
                let f = JSON_EMBED | JSON_ENCODE_ANY | extra;
                assert_bytes_eq(
                    &format!("EMBED dumps({text:?}, {f:#x})"),
                    &dumps(c(), cj, f),
                    &dumps(r(), rj, f),
                );
            }
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}

/* ---- ERRORS 131: MAX_INTEGER_STR_LENGTH guard ---- */

#[test]
fn dumps_integer_extremes() {
    let _g = dtoa_guard();
    unsafe {
        // json_int_t is long long; the widest decimal form is 20 chars, so the
        // `size >= MAX_INTEGER_STR_LENGTH` (25) guard is unreachable. Verify
        // that the extremes format identically instead.
        for v in [
            0i64,
            1,
            -1,
            i64::MIN,
            i64::MAX,
            i64::MIN + 1,
            i64::MAX - 1,
            -999_999_999_999_999_999,
        ] {
            let cj = (c().json_integer)(v);
            let rj = (r().json_integer)(v);
            assert_bytes_eq(
                &format!("dumps(integer {v})"),
                &dumps(c(), cj, JSON_ENCODE_ANY),
                &dumps(r(), rj, JSON_ENCODE_ANY),
            );
            assert_eq!(
                dumps(c(), cj, JSON_ENCODE_ANY).unwrap(),
                v.to_string().into_bytes()
            );
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}

/* ---- ERRORS 145, 146: unreachable SORT_KEYS asserts (documented) ---- */

#[test]
fn dumps_sort_keys_empty_object_takes_the_iter_null_path() {
    let _g = dtoa_guard();
    unsafe {
        // The `size == 0` / `assert(i == size)` / `assert(value)` paths in the
        // SORT_KEYS branch are guarded by `if (!iter)` returning first, so an
        // empty object never reaches the malloc. Confirm both agree.
        for api in both() {
            let o = (api.json_object)();
            let d = dumps(api, o, JSON_SORT_KEYS);
            assert_eq!(d.as_deref(), Some(&b"{}"[..]), "{}", api.tag);
            let d2 = dumps(api, o, JSON_SORT_KEYS | json_indent(2));
            assert_eq!(d2.as_deref(), Some(&b"{}"[..]));
            let d3 = dumps(api, o, JSON_SORT_KEYS | JSON_EMBED);
            assert_eq!(d3.as_deref(), Some(&b""[..]));
            decref(api, o);
        }
        let co = (c().json_object)();
        let ro = (r().json_object)();
        for f in [JSON_SORT_KEYS, JSON_SORT_KEYS | JSON_EMBED, JSON_SORT_KEYS | JSON_COMPACT] {
            assert_bytes_eq("empty object + SORT_KEYS", &dumps(c(), co, f), &dumps(r(), ro, f));
        }
        decref(c(), co);
        decref(r(), ro);
        let _ = CStr::from_bytes_with_nul(b"\0");
    }
}

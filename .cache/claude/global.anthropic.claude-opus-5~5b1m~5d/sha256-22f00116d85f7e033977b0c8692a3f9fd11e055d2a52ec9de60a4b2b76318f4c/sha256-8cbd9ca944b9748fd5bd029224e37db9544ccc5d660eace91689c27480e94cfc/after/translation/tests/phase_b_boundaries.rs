//! Phase B — exact-size boundary conditions that only show up when the
//! buffer/length is precisely at a limit:
//!   * `skip_utf8_bom` requires `can_access_at_index(buffer, 4)`, so a BOM is
//!     only skipped when at least 5 bytes are readable — reachable only through
//!     the length-based parse entry points.
//!   * `print_string_ptr`'s NULL-input branch reserves `sizeof("\"\"") == 3`,
//!     observable only through `cJSON_PrintPreallocated` with a tight buffer.

mod common;
use common::*;
use std::ffi::{c_char, c_int};

/// BOM handling as a function of the exact `buffer_length`.
#[test]
fn bom_exact_buffer_lengths() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let bodies: Vec<&[u8]> = vec![b"1", b"{}", b"[]", b"12", b"null", b"\"a\"", b" 1"];
        for body in bodies {
            let mut text = b"\xef\xbb\xbf".to_vec();
            text.extend_from_slice(body);
            let buf = cbytes(&text);
            for len in 0..(text.len() + 3) {
                let a = (c.cJSON_ParseWithLength)(buf.as_ptr() as *const c_char, len);
                let b = (r.cJSON_ParseWithLength)(buf.as_ptr() as *const c_char, len);
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "BOM+{:?} len={}: NULL-ness (C null={})",
                    String::from_utf8_lossy(body),
                    len,
                    a.is_null()
                );
                if !a.is_null() {
                    assert_eq!(
                        show(&print_unformatted_and_free(&c, a)),
                        show(&print_unformatted_and_free(&r, b)),
                        "BOM+{:?} len={}: value",
                        String::from_utf8_lossy(body),
                        len
                    );
                }
                assert_eq!(
                    (c.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize,
                    (r.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize,
                    "BOM+{:?} len={}: error offset",
                    String::from_utf8_lossy(body),
                    len
                );
                (c.cJSON_Delete)(a);
                (r.cJSON_Delete)(b);

                for rnt in [0i32, 1] {
                    let a = (c.cJSON_ParseWithLengthOpts)(
                        buf.as_ptr() as *const c_char,
                        len,
                        std::ptr::null_mut(),
                        rnt,
                    );
                    let b = (r.cJSON_ParseWithLengthOpts)(
                        buf.as_ptr() as *const c_char,
                        len,
                        std::ptr::null_mut(),
                        rnt,
                    );
                    assert_eq!(
                        a.is_null(),
                        b.is_null(),
                        "BOM+{:?} len={} rnt={}",
                        String::from_utf8_lossy(body),
                        len,
                        rnt
                    );
                    (c.cJSON_Delete)(a);
                    (r.cJSON_Delete)(b);
                }
            }
        }
        // Truncated / partial BOMs at exact lengths
        for prefix in [&b"\xef"[..], b"\xef\xbb", b"\xef\xbb\xbf", b"\xbb\xbf"] {
            let mut text = prefix.to_vec();
            text.extend_from_slice(b"1");
            let buf = cbytes(&text);
            for len in 0..(text.len() + 3) {
                let a = (c.cJSON_ParseWithLength)(buf.as_ptr() as *const c_char, len);
                let b = (r.cJSON_ParseWithLength)(buf.as_ptr() as *const c_char, len);
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "partial BOM {:?} len={}",
                    prefix,
                    len
                );
                if !a.is_null() {
                    assert_eq!(
                        show(&print_unformatted_and_free(&c, a)),
                        show(&print_unformatted_and_free(&r, b)),
                        "partial BOM {:?} len={} value",
                        prefix,
                        len
                    );
                }
                (c.cJSON_Delete)(a);
                (r.cJSON_Delete)(b);
            }
        }
    }
}

/// `cJSON_PrintPreallocated` over EVERY buffer length for the items whose
/// printers use hand-written size constants.
#[test]
fn print_preallocated_every_length() {
    unsafe {
        let (c, r) = both();

        // 1. string item with valuestring == NULL -> print_string_ptr(NULL) path
        for t in [cJSON_String, cJSON_Raw] {
            let ci = (c.cJSON_CreateNumber)(0.0);
            let ri = (r.cJSON_CreateNumber)(0.0);
            (*ci).type_ = t;
            (*ri).type_ = t;
            for len in 0..10usize {
                for format in [0i32, 1] {
                    let mut cb = vec![0xAAu8; 32];
                    let mut rb = vec![0xAAu8; 32];
                    let a = (c.cJSON_PrintPreallocated)(
                        ci,
                        cb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        format,
                    );
                    let b = (r.cJSON_PrintPreallocated)(
                        ri,
                        rb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        format,
                    );
                    assert_eq!(
                        a, b,
                        "NULL-valuestring t={} len={} format={} return",
                        t, len, format
                    );
                    assert_eq!(
                        cb, rb,
                        "NULL-valuestring t={} len={} format={} bytes",
                        t, len, format
                    );
                }
            }
            (*ci).type_ = cJSON_Number;
            (*ri).type_ = cJSON_Number;
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }

        // 2. every small item shape, every buffer length
        let shapes: Vec<Node> = vec![
            Node::Null,
            Node::True,
            Node::False,
            Node::Number(0.0),
            Node::Number(-1.0),
            Node::Number(1e300),
            Node::Number(f64::NAN),
            Node::Str(String::new()),
            Node::Str("a".into()),
            Node::Str("\n".into()),
            Node::Str("\u{1}".into()),
            Node::Raw("1".into()),
            Node::Raw(String::new()),
            Node::Array(vec![]),
            Node::Array(vec![Node::Number(1.0)]),
            Node::Array(vec![Node::Number(1.0), Node::Number(2.0)]),
            Node::Object(vec![]),
            Node::Object(vec![("a".into(), Node::Number(1.0))]),
            Node::Object(vec![
                ("a".into(), Node::Number(1.0)),
                ("b".into(), Node::Str(String::new())),
            ]),
            Node::Object(vec![("".into(), Node::Array(vec![Node::Object(vec![])]))]),
        ];
        for (i, n) in shapes.iter().enumerate() {
            let ci = build(&c, n);
            let ri = build(&r, n);
            for len in 0..40usize {
                for format in [0i32, 1] {
                    let mut cb = vec![0x55u8; 64];
                    let mut rb = vec![0x55u8; 64];
                    let a = (c.cJSON_PrintPreallocated)(
                        ci,
                        cb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        format,
                    );
                    let b = (r.cJSON_PrintPreallocated)(
                        ri,
                        rb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        format,
                    );
                    assert_eq!(
                        a, b,
                        "shape #{} len={} format={} return",
                        i, len, format
                    );
                    assert_eq!(cb, rb, "shape #{} len={} format={} bytes", i, len, format);
                }
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }

        // 3. randomized trees, every buffer length up to the exact size + 3
        let mut rng = Rng::new(0xB0_11_DA);
        for i in 0..120 {
            let n = random_node(&mut rng, 3);
            let ci = build(&c, n_ref(&n));
            let ri = build(&r, n_ref(&n));
            for format in [0i32, 1] {
                let exact = if format == 1 {
                    print_and_free(&c, ci)
                } else {
                    print_unformatted_and_free(&c, ci)
                }
                .map(|v| v.len() + 1)
                .unwrap_or(1);
                let lo = exact.saturating_sub(4);
                for len in lo..(exact + 3) {
                    let mut cb = vec![0x33u8; exact + 16];
                    let mut rb = vec![0x33u8; exact + 16];
                    let a = (c.cJSON_PrintPreallocated)(
                        ci,
                        cb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        format,
                    );
                    let b = (r.cJSON_PrintPreallocated)(
                        ri,
                        rb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        format,
                    );
                    assert_eq!(a, b, "random #{} len={} format={} return", i, len, format);
                    assert_eq!(cb, rb, "random #{} len={} format={} bytes", i, len, format);
                }
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

fn n_ref(n: &Node) -> &Node {
    n
}

/// `cJSON_PrintBuffered` with the prebuffer exactly at / around the needed size.
#[test]
fn print_buffered_every_prebuffer() {
    unsafe {
        let (c, r) = both();
        let mut trees: Vec<Node> = vec![
            Node::Null,
            Node::Str(String::new()),
            Node::Array(vec![]),
            Node::Object(vec![]),
            Node::Number(1.0 / 3.0),
            Node::Object(vec![("a".into(), Node::Array(vec![Node::Number(1.0)]))]),
        ];
        let mut rng2 = Rng::new(0xB0FF);
        for _ in 0..60 {
            trees.push(random_node(&mut rng2, 3));
        }
        for (i, n) in trees.iter().enumerate() {
            let ci = build(&c, n);
            let ri = build(&r, n);
            for prebuffer in 0..24i32 {
                for fmt in [0i32, 1] {
                    assert_eq!(
                        show(&print_buffered_and_free(&c, ci, prebuffer, fmt)),
                        show(&print_buffered_and_free(&r, ri, prebuffer, fmt)),
                        "tree #{} prebuffer={} fmt={}",
                        i,
                        prebuffer,
                        fmt
                    );
                }
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

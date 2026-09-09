//! Phase B — CONFIGS.md sections B (decoding) and C (round-trip).
//!
//! `json_loadb` is the lowest-level decoder (buffer + explicit length); the
//! other five entry points wrap the same `parse_json` over a different byte
//! source, so each is driven over the same corpus.

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

/// The flag set used to render a parsed value for comparison. `JSON_ENCODE_ANY`
/// is needed because `JSON_DECODE_ANY` can yield scalar roots, and
/// `JSON_SORT_KEYS` is deliberately *not* used so insertion order is compared.
const RENDER: usize = JSON_ENCODE_ANY;
/// A second rendering that also pins key order, catching value equality with
/// different iteration order.
const RENDER_SORTED: usize = JSON_ENCODE_ANY | JSON_SORT_KEYS | 2 /* JSON_INDENT(2) */;

// ---------------------------------------------------------------------------
// Corpus
// ---------------------------------------------------------------------------

/// Documents that are structurally valid (for at least some flag set), covering
/// every shape `load.c` special-cases.
fn valid_docs() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    macro_rules! push { ($s:expr) => { v.push(($s).as_bytes().to_vec()) } }

    // B1: containers, empties, singletons, nesting
    push!("[]");
    push!("{}");
    push!("[1]");
    push!("[1,2,3]");
    push!("{\"a\":1}");
    push!("{\"a\":1,\"b\":2}");
    push!("[[],[[]],[[[]]]]");
    push!("{\"a\":{\"b\":{\"c\":{}}}}");
    push!("[null,true,false,0,1.5,\"s\",[],{}]");
    push!("{\"\":\"empty key\"}");
    // B22: whitespace in every position
    push!("  [ 1 , 2 ]  ");
    push!("\t[\n1,\r\n2\n]\t");
    push!("{\n  \"a\" : 1 ,\n  \"b\" : 2\n}");
    push!("[\n\n\n1\n\n\n]");
    // B13/B14: number grammar and integer boundaries
    push!("[0]");
    push!("[-0]");
    push!("[9223372036854775807]");
    push!("[-9223372036854775808]");
    push!("[0.0,-0.0,0.5,-0.5]");
    push!("[1e5,1E5,1e+5,1e-5,1E+5,1E-5]");
    push!("[1.5e5,1.5E-5,0.1,0.0001]");
    push!("[1e308,1e-308]");
    push!("[1e-400]"); // underflow to 0.0, silently
    push!("[-1e-400]");
    push!("[1.7976931348623157e308]");
    push!("[2.2250738585072014e-308]");
    push!("[4.9406564584124654e-324]");
    push!("[9007199254740992,9007199254740993]");
    push!("[123456789012345678901234567890]"); // integer overflow unless INT_AS_REAL
    push!("[-123456789012345678901234567890]");
    push!("[1e999]"); // real overflow
    // B17: all valid escapes
    push!(r#"["\"","\\","\/","\b","\f","\n","\r","\t"]"#);
    push!(r#"["\u0041","\u00e9","\u20ac","\uffff","\u0001","\u001f"]"#);
    // B15: valid surrogate pairs
    push!(r#"["\ud83d\ude00"]"#);
    push!(r#"["\uD800\uDC00","\uDBFF\uDFFF"]"#);
    push!(r#"["a\ud83d\ude00b"]"#);
    // B10: \u0000 (needs JSON_ALLOW_NUL)
    push!(r#"["\u0000"]"#);
    push!(r#"["a\u0000b"]"#);
    // B11: NUL in a key (always rejected)
    push!(r#"{"a\u0000b":1}"#);
    // B16: raw multi-byte UTF-8
    push!("[\"é\",\"€\",\"😀\",\"aé€😀z\"]");
    push!("{\"é\":\"€\",\"😀\":1}");
    push!("[\"\u{10FFFF}\",\"\u{FFFD}\"]");
    // B4/B5: duplicate keys
    push!(r#"{"a":1,"a":2}"#);
    push!(r#"{"a":1,"b":2,"a":3,"b":4}"#);
    push!(r#"{"a":1,"\u0061":2}"#); // decoded keys collide
    // B2/B3: scalar roots
    push!("42");
    push!("-42");
    push!("1.5");
    push!("\"str\"");
    push!("true");
    push!("false");
    push!("null");
    push!("\"\"");
    // B6/B7: trailing garbage
    push!("[1] garbage");
    push!("{} {}");
    push!("[1][2]");
    push!("42 43");
    push!("[1]\n");
    push!("[1]   ");
    // B21: array sizes
    for n in [7usize, 8, 9, 16, 17, 33] {
        let items: Vec<String> = (0..n).map(|i| i.to_string()).collect();
        v.push(format!("[{}]", items.join(",")).into_bytes());
    }
    // B19: object sizes straddling every rehash threshold
    for n in [0usize, 1, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129] {
        let items: Vec<String> = (0..n).map(|i| format!("\"k{i:04}\":{i}")).collect();
        v.push(format!("{{{}}}", items.join(",")).into_bytes());
    }
    // B20: overwriting the 8th key (rehash without size growth)
    let mut items: Vec<String> = (0..8).map(|i| format!("\"k{i}\":{i}")).collect();
    items.push("\"k7\":99".into());
    v.push(format!("{{{}}}", items.join(",")).into_bytes());
    // B18: nesting depth exactly at and past JSON_PARSER_MAX_DEPTH
    for d in [1usize, 2, 100, 2047, 2048, 2049, 2100] {
        v.push(format!("{}{}", "[".repeat(d), "]".repeat(d)).into_bytes());
        v.push(
            format!(
                "{}1{}",
                "{\"a\":".repeat(d.min(2049)),
                "}".repeat(d.min(2049))
            )
            .into_bytes(),
        );
    }
    // Mixed nesting to the limit
    v.push(
        format!(
            "{}{}",
            "[{\"a\":".repeat(1024),
            "}]".repeat(1024)
        )
        .into_bytes(),
    );
    // B24: raw NUL inside the buffer
    v.push(b"[1,\x002]".to_vec());
    v.push(b"[\"a\x00b\"]".to_vec());
    v.push(b"\x00".to_vec());
    v.push(b"[]\x00".to_vec());
    // B38: empty input
    v.push(Vec::new());
    v.push(b" ".to_vec());
    v
}

/// Malformed documents, kept in the Phase-B corpus too because a *valid* flag
/// combination must still agree on the rejection.
fn invalid_docs() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    macro_rules! push { ($s:expr) => { v.push(($s).as_bytes().to_vec()) } }
    push!("[");
    push!("]");
    push!("{");
    push!("}");
    push!(",");
    push!(":");
    push!("[1,");
    push!("[1,2");
    push!("[1,]");
    push!("[,1]");
    push!("[:]");
    push!("[1 2]");
    push!("{\"a\"}");
    push!("{\"a\":}");
    push!("{\"a\" 1}");
    push!("{\"a\",1}");
    push!("{1:2}");
    push!("{,}");
    push!("{\"a\":1");
    push!("{\"a\":1]");
    push!("[1}");
    push!("nul");
    push!("True");
    push!("NaN");
    push!("Infinity");
    push!("foo");
    push!("01");
    push!("-012");
    push!("-");
    push!("-x");
    push!("-.5");
    push!("+1");
    push!(".5");
    push!("1.");
    push!("1.e5");
    push!("1e");
    push!("1e+");
    push!("1ex");
    push!("0x10");
    push!("[00]");
    push!("#");
    push!("'a'");
    push!("@");
    push!("(");
    push!(")");
    push!("\"abc");
    push!("\"\\x\"");
    push!("\"\\ \"");
    push!("\"\\u12g4\"");
    push!("\"\\u1\"");
    push!("\"\\u\"");
    push!("\"\\ud800\"");
    push!("\"\\ud800x\"");
    push!("\"\\ud800\\u0041\"");
    push!("\"\\udc00\"");
    push!("\"\\udfff\"");
    push!("\"\\ud800\\ud800\"");
    push!("[\"a\nb\"]"); // raw newline in string
    // Raw control chars in strings
    for c in [0x01u8, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x1f] {
        v.push([b'[', b'"', c, b'"', b']'].to_vec());
    }
    // Invalid UTF-8, inside and outside strings
    for bad in [
        vec![0x80u8],
        vec![0xBF],
        vec![0xC0, 0x80],
        vec![0xC1, 0xBF],
        vec![0xF5, 0x80, 0x80, 0x80],
        vec![0xFF],
        vec![0xFE],
        vec![0xE0, 0x80, 0x80], // overlong
        vec![0xED, 0xA0, 0x80], // surrogate
        vec![0xF4, 0x90, 0x80, 0x80], // > U+10FFFF
        vec![0xC3],             // truncated 2-byte
        vec![0xE2, 0x82],       // truncated 3-byte
    ] {
        let mut inside = b"[\"".to_vec();
        inside.extend_from_slice(&bad);
        inside.extend_from_slice(b"\"]");
        v.push(inside);
        let mut outside = b"[".to_vec();
        outside.extend_from_slice(&bad);
        outside.extend_from_slice(b"]");
        v.push(outside);
        v.push(bad);
    }
    // Long "near" context (>20 chars suppresses the suffix)
    push!("[verylongidentifierthatexceedstwentychars]");
    push!("[abcdefghijklmnopqrst]"); // exactly 20
    push!("[abcdefghijklmnopqrstu]"); // 21
    v
}

fn random_docs(seed: u64, n: usize) -> Vec<Vec<u8>> {
    // Render random trees through the C library, then feed the text back in.
    let p = pair();
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    unsafe {
        for _ in 0..n {
            let node = rng.container(4);
            let j = node.build(&p.c);
            if let Some(d) = p.c.dumps(j, JSON_ENCODE_ANY) {
                out.push(d);
            }
            // Also an ENSURE_ASCII rendering, so \u escapes get re-parsed.
            if let Some(d) = p.c.dumps(j, JSON_ENCODE_ANY | JSON_ENSURE_ASCII) {
                out.push(d);
            }
            p.c.json_decref(j);
        }
    }
    out
}

/// Randomly mutated documents: byte flips / insertions / deletions on valid
/// input, which reaches lexer states no hand-written case does.
fn mutated_docs(seed: u64, n: usize) -> Vec<Vec<u8>> {
    let base = random_docs(seed ^ 0x5A5A, 40);
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    for _ in 0..n {
        if base.is_empty() {
            break;
        }
        let mut d = base[rng.below(base.len())].clone();
        for _ in 0..1 + rng.below(3) {
            if d.is_empty() {
                break;
            }
            let i = rng.below(d.len());
            match rng.below(3) {
                0 => d[i] = rng.below(256) as u8,
                1 => d.insert(i, rng.below(256) as u8),
                _ => {
                    d.remove(i);
                }
            }
        }
        out.push(d);
    }
    out
}

// ---------------------------------------------------------------------------
// Comparison core
// ---------------------------------------------------------------------------

/// Parse `doc` with `json_loadb` in both libraries under `flags`; assert the
/// success/failure, the whole `json_error_t`, and the re-encoded value all match.
fn cmp_loadb(doc: &[u8], flags: usize) {
    unsafe {
        let p = pair();
        let mut ce = json_error_t::default();
        let mut re = json_error_t::default();
        let cj = p.c.json_loadb(doc.as_ptr() as *const c_char, doc.len(), flags, &mut ce);
        let rj = p.r.json_loadb(doc.as_ptr() as *const c_char, doc.len(), flags, &mut re);
        let ctx = || {
            format!(
                "json_loadb(flags=0x{flags:x}) doc={:?} ({} bytes)",
                String::from_utf8_lossy(doc),
                doc.len()
            )
        };
        assert_eq!(
            cj.is_null(),
            rj.is_null(),
            "{}: NULL-ness (C err={:?} / RUST err={:?})",
            ctx(),
            ce.snap().text,
            re.snap().text
        );
        // The whole error struct, including line/column/position and the code
        // byte at text[159], must match on BOTH the success and failure paths.
        assert_eq!(ce.snap(), re.snap(), "{}: json_error_t", ctx());
        if !cj.is_null() {
            assert_eq!(
                typeof_json(cj),
                typeof_json(rj),
                "{}: root json_type",
                ctx()
            );
            for r in [RENDER, RENDER_SORTED, RENDER | JSON_ENSURE_ASCII] {
                let cd = p.c.dumps(cj, r);
                let rd = p.r.dumps(rj, r);
                assert_eq!(
                    cd,
                    rd,
                    "{}: re-encode(0x{r:x}):\n  C   = {}\n  RUST= {}",
                    ctx(),
                    show(&cd),
                    show(&rd)
                );
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
    }
}

/// The full 32-element decode-flag lattice.
fn all_decode_flags() -> Vec<usize> {
    (0..32usize).collect()
}

// ===========================================================================
// B1..B24, B32, B33, B38 — json_loadb over the whole flag lattice
// ===========================================================================

#[test]
fn b_loadb_full_flag_lattice_valid() {
    for doc in valid_docs() {
        for f in all_decode_flags() {
            cmp_loadb(&doc, f);
        }
    }
}

#[test]
fn b_loadb_full_flag_lattice_invalid() {
    for doc in invalid_docs() {
        for f in all_decode_flags() {
            cmp_loadb(&doc, f);
        }
    }
}

#[test]
fn b_loadb_random_documents() {
    for doc in random_docs(0xB000, 400) {
        for f in all_decode_flags() {
            cmp_loadb(&doc, f);
        }
    }
}

#[test]
fn b_loadb_mutated_documents() {
    for doc in mutated_docs(0xB111, 4000) {
        for f in [
            0,
            JSON_DECODE_ANY,
            JSON_ALLOW_NUL,
            JSON_REJECT_DUPLICATES,
            JSON_DISABLE_EOF_CHECK,
            JSON_DECODE_INT_AS_REAL,
            31,
        ] {
            cmp_loadb(&doc, f);
        }
    }
}

/// B23: `buflen` shorter than the buffer, truncating mid-token, mid-escape and
/// mid-UTF-8-sequence.
#[test]
fn b23_truncated_buflen() {
    let docs: Vec<Vec<u8>> = valid_docs()
        .into_iter()
        .chain(invalid_docs())
        .filter(|d| d.len() <= 80)
        .collect();
    for doc in docs {
        for len in 0..=doc.len() {
            for f in [0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK, JSON_ALLOW_NUL, 31] {
                cmp_loadb(&doc[..len], f);
            }
        }
    }
}

/// B33: on success `error->position` is the number of bytes consumed; verify all
/// five error-struct fields on the success path too, for a range of documents
/// and both EOF-check settings.
#[test]
fn b33_error_struct_on_success() {
    unsafe {
        let p = pair();
        for doc in valid_docs() {
            for f in [0usize, JSON_DISABLE_EOF_CHECK, JSON_DECODE_ANY,
                      JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK] {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cj = p.c.json_loadb(doc.as_ptr() as *const c_char, doc.len(), f, &mut ce);
                let rj = p.r.json_loadb(doc.as_ptr() as *const c_char, doc.len(), f, &mut re);
                assert_eq!(
                    ce.snap(),
                    re.snap(),
                    "error struct for {:?} flags 0x{f:x}",
                    String::from_utf8_lossy(&doc)
                );
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }
        }
    }
}

/// A NULL `error` out-param must be tolerated identically (ERRORS L1).
#[test]
fn b_null_error_out_param() {
    unsafe {
        let p = pair();
        for doc in valid_docs().into_iter().chain(invalid_docs()) {
            for f in [0usize, JSON_DECODE_ANY, 31] {
                let cj = p.c.json_loadb(
                    doc.as_ptr() as *const c_char,
                    doc.len(),
                    f,
                    std::ptr::null_mut(),
                );
                let rj = p.r.json_loadb(
                    doc.as_ptr() as *const c_char,
                    doc.len(),
                    f,
                    std::ptr::null_mut(),
                );
                assert_eq!(
                    cj.is_null(),
                    rj.is_null(),
                    "NULL error out-param: {:?} flags 0x{f:x}",
                    String::from_utf8_lossy(&doc)
                );
                if !cj.is_null() {
                    assert_eq!(p.c.dumps(cj, RENDER), p.r.dumps(rj, RENDER));
                }
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }
        }
    }
}

// ===========================================================================
// B25 — json_loads (NUL-terminated; stops at the first '\0')
// ===========================================================================

#[test]
fn b25_json_loads() {
    unsafe {
        let p = pair();
        let docs: Vec<Vec<u8>> = valid_docs()
            .into_iter()
            .chain(invalid_docs())
            .chain(random_docs(0xB25, 150))
            .collect();
        for doc in docs {
            let z = raw_z(&doc); // NUL-terminate, keeping any interior NULs
            for f in all_decode_flags() {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cj = p.c.json_loads(z.as_ptr() as *const c_char, f, &mut ce);
                let rj = p.r.json_loads(z.as_ptr() as *const c_char, f, &mut re);
                let ctx = format!(
                    "json_loads({:?}, flags 0x{f:x})",
                    String::from_utf8_lossy(&doc)
                );
                assert_eq!(cj.is_null(), rj.is_null(), "{ctx}: NULL-ness");
                assert_eq!(ce.snap(), re.snap(), "{ctx}: json_error_t");
                if !cj.is_null() {
                    assert_eq!(
                        p.c.dumps(cj, RENDER),
                        p.r.dumps(rj, RENDER),
                        "{ctx}: re-encode"
                    );
                    p.c.json_decref(cj);
                    p.r.json_decref(rj);
                }
            }
        }
    }
}

// ===========================================================================
// B26..B28 — json_loadf / json_loadfd / json_load_file
// ===========================================================================

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}

fn tmp_path(tag: &str) -> std::path::PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("jansson_load_{tag}_{}", std::process::id()));
    d
}

#[test]
fn b26_b28_stream_decoders() {
    unsafe {
        let p = pair();
        let docs: Vec<Vec<u8>> = valid_docs()
            .into_iter()
            .chain(invalid_docs())
            .chain(random_docs(0xB26, 120))
            .collect();
        let flags = [
            0usize,
            JSON_DECODE_ANY,
            JSON_ALLOW_NUL,
            JSON_REJECT_DUPLICATES,
            JSON_DISABLE_EOF_CHECK,
            JSON_DECODE_INT_AS_REAL,
            31,
        ];
        let path = tmp_path("doc");
        for doc in &docs {
            std::fs::write(&path, doc).unwrap();
            let ps = cs(path.to_str().unwrap());
            for &f in &flags {
                let ctx = format!("doc={:?} flags=0x{f:x}", String::from_utf8_lossy(doc));

                // --- B28: json_load_file
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cj = p.c.json_load_file(ps.as_ptr(), f, &mut ce);
                let rj = p.r.json_load_file(ps.as_ptr(), f, &mut re);
                assert_eq!(cj.is_null(), rj.is_null(), "json_load_file NULL-ness [{ctx}]");
                assert_eq!(ce.snap(), re.snap(), "json_load_file error [{ctx}]");
                if !cj.is_null() {
                    assert_eq!(
                        p.c.dumps(cj, RENDER),
                        p.r.dumps(rj, RENDER),
                        "json_load_file value [{ctx}]"
                    );
                }
                p.c.json_decref(cj);
                p.r.json_decref(rj);

                // --- B26: json_loadf, each library given its own FILE*
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let m = cs("rb");
                let cfp = fopen(ps.as_ptr(), m.as_ptr());
                let rfp = fopen(ps.as_ptr(), m.as_ptr());
                assert!(!cfp.is_null() && !rfp.is_null());
                let cj = p.c.json_loadf(cfp, f, &mut ce);
                let rj = p.r.json_loadf(rfp, f, &mut re);
                fclose(cfp);
                fclose(rfp);
                assert_eq!(cj.is_null(), rj.is_null(), "json_loadf NULL-ness [{ctx}]");
                assert_eq!(ce.snap(), re.snap(), "json_loadf error [{ctx}]");
                if !cj.is_null() {
                    assert_eq!(
                        p.c.dumps(cj, RENDER),
                        p.r.dumps(rj, RENDER),
                        "json_loadf value [{ctx}]"
                    );
                }
                p.c.json_decref(cj);
                p.r.json_decref(rj);

                // --- B27: json_loadfd, one read() per byte
                use std::os::unix::io::AsRawFd;
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cfile = std::fs::File::open(&path).unwrap();
                let rfile = std::fs::File::open(&path).unwrap();
                let cj = p.c.json_loadfd(cfile.as_raw_fd(), f, &mut ce);
                let rj = p.r.json_loadfd(rfile.as_raw_fd(), f, &mut re);
                assert_eq!(cj.is_null(), rj.is_null(), "json_loadfd NULL-ness [{ctx}]");
                assert_eq!(ce.snap(), re.snap(), "json_loadfd error [{ctx}]");
                if !cj.is_null() {
                    assert_eq!(
                        p.c.dumps(cj, RENDER),
                        p.r.dumps(rj, RENDER),
                        "json_loadfd value [{ctx}]"
                    );
                }
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }
        }
        let _ = std::fs::remove_file(&path);
    }
}

// ===========================================================================
// B29, B30 — json_load_callback (MAX_BUF_LEN == 1024)
// ===========================================================================

struct Feeder {
    data: Vec<u8>,
    pos: usize,
    /// Bytes to hand over per call (clamped to the requested `buflen`).
    chunk: usize,
    /// If > 0, return `stop_kind` on this 1-based call number.
    stop_at: usize,
    /// 0 => return 0 (EOF), 1 => return (size_t)-1 (also EOF)
    stop_kind: usize,
    calls: usize,
}

unsafe extern "C" fn feed_cb(buffer: *mut c_void, buflen: usize, data: *mut c_void) -> usize {
    let f = &mut *(data as *mut Feeder);
    f.calls += 1;
    if f.stop_at != 0 && f.calls == f.stop_at {
        return if f.stop_kind == 0 { 0 } else { usize::MAX };
    }
    let want = f.chunk.min(buflen);
    let avail = f.data.len() - f.pos;
    let n = want.min(avail);
    if n > 0 {
        std::ptr::copy_nonoverlapping(f.data[f.pos..].as_ptr(), buffer as *mut u8, n);
        f.pos += n;
    }
    n
}

fn load_via_callback(
    l: &Lib,
    doc: &[u8],
    chunk: usize,
    stop_at: usize,
    stop_kind: usize,
    flags: usize,
    err: &mut json_error_t,
) -> json_ptr {
    let mut f = Feeder {
        data: doc.to_vec(),
        pos: 0,
        chunk,
        stop_at,
        stop_kind,
        calls: 0,
    };
    unsafe { l.json_load_callback(Some(feed_cb), &mut f as *mut _ as *mut c_void, flags, err) }
}

#[test]
fn b29_b30_json_load_callback() {
    unsafe {
        let p = pair();
        // Include a document comfortably larger than MAX_BUF_LEN (1024).
        let big: Vec<u8> = {
            let items: Vec<String> = (0..400).map(|i| format!("\"key{i:06}\":{i}")).collect();
            format!("{{{}}}", items.join(",")).into_bytes()
        };
        let exactly_1024: Vec<u8> = {
            let mut s = String::from("[");
            while s.len() < 1023 {
                s.push('1');
                s.push(',');
            }
            s.truncate(1022);
            s.push(']');
            let mut b = s.into_bytes();
            b.resize(1024, b' ');
            b
        };
        let docs: Vec<Vec<u8>> = valid_docs()
            .into_iter()
            .chain(invalid_docs())
            .chain(random_docs(0xB29, 60))
            .chain([big, exactly_1024])
            .collect();

        let flags = [0usize, JSON_DECODE_ANY, JSON_ALLOW_NUL, JSON_DISABLE_EOF_CHECK, 31];
        // B29: chunk sizes around the 1024-byte boundary
        for chunk in [1usize, 2, 7, 1023, 1024, 1025, 4096] {
            for doc in &docs {
                for &f in &flags {
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let cj = load_via_callback(&p.c, doc, chunk, 0, 0, f, &mut ce);
                    let rj = load_via_callback(&p.r, doc, chunk, 0, 0, f, &mut re);
                    let ctx = format!(
                        "json_load_callback(chunk {chunk}, flags 0x{f:x}) doc={:?}",
                        String::from_utf8_lossy(doc)
                    );
                    assert_eq!(cj.is_null(), rj.is_null(), "{ctx}: NULL-ness");
                    assert_eq!(ce.snap(), re.snap(), "{ctx}: json_error_t");
                    if !cj.is_null() {
                        assert_eq!(
                            p.c.dumps(cj, RENDER),
                            p.r.dumps(rj, RENDER),
                            "{ctx}: re-encode"
                        );
                    }
                    p.c.json_decref(cj);
                    p.r.json_decref(rj);
                }
            }
        }
        // B30: callback stopping early with 0 and with (size_t)-1
        for stop_kind in [0usize, 1] {
            for stop_at in 1..=4usize {
                for doc in docs.iter().take(60) {
                    for &f in &flags {
                        let mut ce = json_error_t::default();
                        let mut re = json_error_t::default();
                        let cj = load_via_callback(&p.c, doc, 4, stop_at, stop_kind, f, &mut ce);
                        let rj = load_via_callback(&p.r, doc, 4, stop_at, stop_kind, f, &mut re);
                        let ctx = format!(
                            "json_load_callback(stop@{stop_at} kind {stop_kind}, flags 0x{f:x}) \
                             doc={:?}",
                            String::from_utf8_lossy(doc)
                        );
                        assert_eq!(cj.is_null(), rj.is_null(), "{ctx}: NULL-ness");
                        assert_eq!(ce.snap(), re.snap(), "{ctx}: json_error_t");
                        if !cj.is_null() {
                            assert_eq!(p.c.dumps(cj, RENDER), p.r.dumps(rj, RENDER), "{ctx}");
                        }
                        p.c.json_decref(cj);
                        p.r.json_decref(rj);
                    }
                }
            }
        }
    }
}

/// B31: `JSON_DISABLE_EOF_CHECK` streaming several documents out of one stream.
#[test]
fn b31_streaming_multiple_documents() {
    unsafe {
        let p = pair();
        let streams: Vec<&str> = vec![
            "[1][2][3]",
            "{}{}{}",
            "[1] [2]  [3]",
            "{\"a\":1}{\"b\":2}",
            "[1]\n[2]\n[3]\n",
            "1 2 3",           // needs DECODE_ANY too
            "[1][2]garbage",   // third read fails
        ];
        for s in streams {
            for f in [
                JSON_DISABLE_EOF_CHECK,
                JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY,
            ] {
                // --- FILE* variant
                let path = tmp_path("stream");
                std::fs::write(&path, s).unwrap();
                let ps = cs(path.to_str().unwrap());
                let m = cs("rb");
                let cfp = fopen(ps.as_ptr(), m.as_ptr());
                let rfp = fopen(ps.as_ptr(), m.as_ptr());
                for round in 0..4 {
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let cj = p.c.json_loadf(cfp, f, &mut ce);
                    let rj = p.r.json_loadf(rfp, f, &mut re);
                    let ctx = format!("stream {s:?} flags 0x{f:x} round {round}");
                    assert_eq!(cj.is_null(), rj.is_null(), "{ctx}: NULL-ness");
                    assert_eq!(ce.snap(), re.snap(), "{ctx}: json_error_t");
                    if !cj.is_null() {
                        assert_eq!(p.c.dumps(cj, RENDER), p.r.dumps(rj, RENDER), "{ctx}");
                    }
                    p.c.json_decref(cj);
                    p.r.json_decref(rj);
                }
                fclose(cfp);
                fclose(rfp);
                let _ = std::fs::remove_file(&path);

                // --- fd variant
                let path = tmp_path("stream_fd");
                std::fs::write(&path, s).unwrap();
                use std::os::unix::io::AsRawFd;
                let cfile = std::fs::File::open(&path).unwrap();
                let rfile = std::fs::File::open(&path).unwrap();
                for round in 0..4 {
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let cj = p.c.json_loadfd(cfile.as_raw_fd(), f, &mut ce);
                    let rj = p.r.json_loadfd(rfile.as_raw_fd(), f, &mut re);
                    let ctx = format!("fd stream {s:?} flags 0x{f:x} round {round}");
                    assert_eq!(cj.is_null(), rj.is_null(), "{ctx}: NULL-ness");
                    assert_eq!(ce.snap(), re.snap(), "{ctx}: json_error_t");
                    if !cj.is_null() {
                        assert_eq!(p.c.dumps(cj, RENDER), p.r.dumps(rj, RENDER), "{ctx}");
                    }
                    p.c.json_decref(cj);
                    p.r.json_decref(rj);
                }
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

// ===========================================================================
// C1..C4 — round-trip composition
// ===========================================================================

#[test]
fn c1_c2_roundtrip_decode_then_encode() {
    unsafe {
        let p = pair();
        let encode_flags: Vec<usize> = vec![
            0,
            JSON_COMPACT,
            JSON_INDENT(2),
            JSON_INDENT(31),
            JSON_SORT_KEYS,
            JSON_ENSURE_ASCII,
            JSON_ESCAPE_SLASH,
            JSON_ENCODE_ANY,
            JSON_EMBED,
            JSON_ENCODE_ANY | JSON_ENSURE_ASCII | JSON_SORT_KEYS | JSON_INDENT(1),
        ];
        // C2: every usable real precision (25..31 fail, which must also match).
        let mut prec_flags: Vec<usize> = (0..=31usize)
            .map(|q| JSON_REAL_PRECISION(q) | JSON_ENCODE_ANY)
            .collect();
        prec_flags.extend(encode_flags.iter().copied());

        let docs: Vec<Vec<u8>> = valid_docs()
            .into_iter()
            .chain(random_docs(0xC1, 250))
            .collect();
        for doc in &docs {
            for df in all_decode_flags() {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cj = p.c.json_loadb(doc.as_ptr() as *const c_char, doc.len(), df, &mut ce);
                let rj = p.r.json_loadb(doc.as_ptr() as *const c_char, doc.len(), df, &mut re);
                assert_eq!(cj.is_null(), rj.is_null());
                if cj.is_null() {
                    continue;
                }
                for &ef in &prec_flags {
                    let cd = p.c.dumps(cj, ef);
                    let rd = p.r.dumps(rj, ef);
                    assert_eq!(
                        cd,
                        rd,
                        "round-trip decode 0x{df:x} -> encode 0x{ef:x} for {:?}:\n\
                         C   = {}\nRUST= {}",
                        String::from_utf8_lossy(doc),
                        show(&cd),
                        show(&rd)
                    );
                }
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }
        }
    }
}

/// C3: `JSON_ALLOW_NUL` decode -> re-encode -> re-decode, twice around.
#[test]
fn c3_allow_nul_roundtrip() {
    unsafe {
        let p = pair();
        let docs: Vec<&str> = vec![
            r#"["\u0000"]"#,
            r#"["a\u0000b"]"#,
            r#"["\u0000\u0000"]"#,
            r#"{"k":"a\u0000"}"#,
            r#"["\u0000",1,null]"#,
        ];
        for d in docs {
            for ef in [
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | JSON_ENSURE_ASCII,
                JSON_ENCODE_ANY | JSON_COMPACT,
            ] {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let cj = p.c.json_loadb(
                    d.as_ptr() as *const c_char,
                    d.len(),
                    JSON_ALLOW_NUL,
                    &mut ce,
                );
                let rj = p.r.json_loadb(
                    d.as_ptr() as *const c_char,
                    d.len(),
                    JSON_ALLOW_NUL,
                    &mut re,
                );
                assert_eq!(ce.snap(), re.snap(), "{d}");
                assert_eq!(cj.is_null(), rj.is_null(), "{d}");
                if cj.is_null() {
                    continue;
                }
                let cd = p.c.dumps(cj, ef).unwrap();
                let rd = p.r.dumps(rj, ef).unwrap();
                assert_eq!(cd, rd, "{d} re-encode 0x{ef:x}");
                // Re-decode the re-encoding.
                let mut ce2 = json_error_t::default();
                let mut re2 = json_error_t::default();
                let cj2 = p.c.json_loadb(
                    cd.as_ptr() as *const c_char,
                    cd.len(),
                    JSON_ALLOW_NUL | JSON_DECODE_ANY,
                    &mut ce2,
                );
                let rj2 = p.r.json_loadb(
                    rd.as_ptr() as *const c_char,
                    rd.len(),
                    JSON_ALLOW_NUL | JSON_DECODE_ANY,
                    &mut re2,
                );
                assert_eq!(ce2.snap(), re2.snap(), "{d} re-decode");
                assert_eq!(cj2.is_null(), rj2.is_null(), "{d} re-decode");
                if !cj2.is_null() {
                    assert_eq!(p.c.dumps(cj2, ef), p.r.dumps(rj2, ef), "{d} second encode");
                }
                p.c.json_decref(cj2);
                p.r.json_decref(rj2);
                p.c.json_decref(cj);
                p.r.json_decref(rj);
            }
        }
    }
}

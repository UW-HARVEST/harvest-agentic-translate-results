//! Phase B — CONFIGS.md section A: encoding.
//!
//! Drives `json_dump_callback` (the single real implementation) directly as well
//! as all five wrappers, across the full encode-flag surface.

mod common;
use common::*;
use std::io::Read;
use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Corpora
// ---------------------------------------------------------------------------

/// A fixed set of container roots that between them hit every structural branch
/// in `do_dump`: empty, one element, many, nested, all 8 value types, key
/// shapes that stress `compare_keys`, and every escapable character class.
fn structural_corpus() -> Vec<Node> {
    let s = |t: &str| Node::Str(t.as_bytes().to_vec());
    let k = |t: &str| t.as_bytes().to_vec();
    vec![
        // A26: empty containers
        Node::Arr(vec![]),
        Node::Obj(vec![]),
        // A27: exactly one element (the comma loop is skipped entirely)
        Node::Arr(vec![Node::Int(1)]),
        Node::Obj(vec![(k("a"), Node::Int(1))]),
        // two, then many
        Node::Arr(vec![Node::Int(1), Node::Int(2)]),
        Node::Obj(vec![(k("a"), Node::Int(1)), (k("b"), Node::Int(2))]),
        Node::Arr((0..10).map(Node::Int).collect()),
        // all 8 types side by side
        Node::Arr(vec![
            Node::Null,
            Node::True,
            Node::False,
            Node::Int(0),
            Node::Real(1.5),
            s("x"),
            Node::Arr(vec![]),
            Node::Obj(vec![]),
        ]),
        // nested empties
        Node::Arr(vec![Node::Arr(vec![Node::Arr(vec![])])]),
        Node::Obj(vec![(k("a"), Node::Obj(vec![(k("b"), Node::Obj(vec![]))]))]),
        // deep-ish mixed nesting
        Node::Obj(vec![
            (k("arr"), Node::Arr(vec![Node::Int(1), Node::Arr(vec![Node::Int(2)])])),
            (k("obj"), Node::Obj(vec![(k("n"), Node::Null)])),
            (k(""), s("empty key")),
        ]),
        // A13: keys that stress compare_keys (memcmp over min(len), then len diff)
        Node::Obj(vec![
            (k("b"), Node::Int(1)),
            (k("a"), Node::Int(2)),
            (k("ab"), Node::Int(3)),
            (k("aa"), Node::Int(4)),
            (k("a"), Node::Int(5)), // duplicate: overwrite, keeps first position
            (k(""), Node::Int(6)),
            (k("A"), Node::Int(7)),
            (k("~"), Node::Int(8)),
            (vec![0xC3, 0xA9], Node::Int(9)),        // é
            (vec![0xFF], Node::Int(10)),             // invalid UTF-8 key (nocheck path)
            (k("aaaaaaaaaaaaaaaaaaaaaaaa"), Node::Int(11)),
        ]),
        // A12: every escape class
        Node::Arr(vec![
            s("\u{8}\u{c}\n\r\t"),
            s("\"\\"),
            s("/"),
            Node::Str(vec![0x01, 0x02, 0x1F]),
            Node::Str(vec![0x7F]), // DEL — never escaped (A10)
            Node::Str(vec![0x20]),
        ]),
        // A7/A8/A9: 1/2/3/4-byte UTF-8, incl. non-BMP
        Node::Arr(vec![
            s("ascii"),
            s("é"),        // 2-byte
            s("€"),        // 3-byte
            s("😀"),       // 4-byte (surrogate pair under ENSURE_ASCII)
            s("\u{10FFFF}"),
            s("\u{FFFF}"),
            s("\u{10000}"),
            s("aé€😀z"),
        ]),
        // A36: embedded NUL (only constructible via *_nocheck)
        Node::Arr(vec![
            Node::StrNoCheck(b"a\0b".to_vec()),
            Node::StrNoCheck(b"\0".to_vec()),
            Node::StrNoCheck(vec![]),
        ]),
        // A21: integer boundaries
        Node::Arr(vec![
            Node::Int(0),
            Node::Int(1),
            Node::Int(-1),
            Node::Int(i64::MAX),
            Node::Int(i64::MIN),
        ]),
        // A20: real shapes
        Node::Arr(vec![
            Node::Real(0.0),
            Node::Real(-0.0),
            Node::Real(1.0),
            Node::Real(-1.0),
            Node::Real(0.5),
            Node::Real(1e15),
            Node::Real(1e16),
            Node::Real(1e17),
            Node::Real(1e-4),
            Node::Real(1e-5),
            Node::Real(1e300),
            Node::Real(1e-300),
            Node::Real(f64::MAX),
            Node::Real(f64::MIN),
            Node::Real(f64::MIN_POSITIVE),
            Node::Real(5e-324),
            Node::Real(1.0 / 3.0),
            Node::Real(9007199254740992.0),
        ]),
    ]
}

/// Random container roots, reproducible from a fixed seed.
fn random_corpus(seed: u64, n: usize, depth: usize) -> Vec<Node> {
    let mut rng = Rng::new(seed);
    (0..n).map(|_| rng.container(depth)).collect()
}

/// Every scalar root (needs `JSON_ENCODE_ANY`).
fn scalar_corpus() -> Vec<Node> {
    vec![
        Node::Null,
        Node::True,
        Node::False,
        Node::Int(0),
        Node::Int(i64::MIN),
        Node::Real(1.5),
        Node::Real(-0.0),
        Node::Str(b"hi".to_vec()),
        Node::Str(vec![]),
        Node::Str("é€😀".as_bytes().to_vec()),
    ]
}

fn sweep(nodes: &[Node], flags: &[usize], ctx: &str) {
    unsafe {
        for node in nodes {
            with_both(node, |p, cj, rj| {
                for &f in flags {
                    assert_dumps_eq(p, cj, rj, f, &format!("{ctx} node={node:?}"));
                }
            });
        }
    }
}

// ===========================================================================
// A1..A3, A11, A16..A18, A26, A27 — the base flag combinations
// ===========================================================================

#[test]
fn a1_a3_a26_a27_base_flags() {
    let flags = [
        0,
        JSON_COMPACT,                     // A2
        JSON_ESCAPE_SLASH,                // A11
        JSON_COMPACT | JSON_ESCAPE_SLASH, // A3 baseline vs compact
    ];
    sweep(&structural_corpus(), &flags, "base");
    sweep(&random_corpus(0xA1, 400, 3), &flags, "base/random");
}

/// A4: every indent width 0..=32 (32 masks back to 0), and A5: indent 31 at
/// depth >= 2, where the total indentation exceeds the 32-byte `whitespace[]`
/// and must be emitted in multiple chunks.
#[test]
fn a4_a5_every_indent_width() {
    let mut flags = Vec::new();
    for n in 0..=32usize {
        flags.push(JSON_INDENT(n));
    }
    let mut nodes = structural_corpus();
    // A5: force depth >= 2 so `depth * 31 > 32`.
    nodes.push(Node::Arr(vec![Node::Arr(vec![Node::Arr(vec![
        Node::Arr(vec![Node::Int(1), Node::Int(2)]),
        Node::Int(3),
    ])])]));
    nodes.push(Node::Obj(vec![(
        b"a".to_vec(),
        Node::Obj(vec![(
            b"b".to_vec(),
            Node::Obj(vec![(b"c".to_vec(), Node::Arr(vec![Node::Int(1), Node::Int(2)]))]),
        )]),
    )]));
    sweep(&nodes, &flags, "indent");
    sweep(&random_corpus(0xA4, 200, 4), &flags, "indent/random");
}

/// A6: `JSON_INDENT(n) | JSON_COMPACT` — the indent branch wins in
/// `dump_indent`, but `JSON_COMPACT` still shortens the `key: value` separator.
#[test]
fn a6_indent_with_compact() {
    let flags: Vec<usize> = (0..=31usize).map(|n| JSON_INDENT(n) | JSON_COMPACT).collect();
    sweep(&structural_corpus(), &flags, "indent|compact");
    sweep(&random_corpus(0xA6, 200, 4), &flags, "indent|compact/random");
}

/// A7..A10: JSON_ENSURE_ASCII off/on across 1/2/3/4-byte UTF-8, including the
/// non-BMP surrogate-pair path and the exactly-0x7F case.
#[test]
fn a7_a10_ensure_ascii() {
    let flags = [
        0,
        JSON_ENSURE_ASCII,
        JSON_ENSURE_ASCII | JSON_COMPACT,
        JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
        JSON_ENSURE_ASCII | JSON_INDENT(2),
    ];
    sweep(&structural_corpus(), &flags, "ensure_ascii");

    // Every codepoint class, one string per codepoint, in bulk.
    let mut rng = Rng::new(0xA7_A7);
    let mut nodes = Vec::new();
    let mut edge: Vec<u32> = vec![
        0x20, 0x7E, 0x7F, 0x80, 0x81, 0x7FF, 0x800, 0x801, 0xD7FF, 0xE000, 0xFFFD, 0xFFFF,
        0x10000, 0x10001, 0x1F600, 0x10FFFE, 0x10FFFF,
    ];
    for _ in 0..3000 {
        edge.push(rng.below(0x110000) as u32);
    }
    let mut batch = Vec::new();
    for cp in edge {
        if let Some(ch) = char::from_u32(cp) {
            if ch != '\0' {
                batch.push(Node::Str(ch.to_string().into_bytes()));
            }
        }
        if batch.len() == 64 {
            nodes.push(Node::Arr(std::mem::take(&mut batch)));
        }
    }
    if !batch.is_empty() {
        nodes.push(Node::Arr(batch));
    }
    sweep(&nodes, &flags, "ensure_ascii/codepoints");
}

/// A13..A15: JSON_SORT_KEYS, on its own and combined with indent/compact.
/// `compare_keys` is `memcmp` over `min(len)` then `len1 - len2`.
#[test]
fn a13_a15_sort_keys() {
    let flags = [
        JSON_SORT_KEYS,
        JSON_SORT_KEYS | JSON_COMPACT,
        JSON_SORT_KEYS | JSON_INDENT(4),
        JSON_SORT_KEYS | JSON_INDENT(1) | JSON_COMPACT,
        JSON_SORT_KEYS | JSON_ENSURE_ASCII,
        0, // A14: insertion order when the flag is off
    ];
    sweep(&structural_corpus(), &flags, "sort_keys");

    // Objects whose keys are prefixes of one another at many lengths, which is
    // exactly where `memcmp(min(len))` then `len1-len2` matters.
    let mut nodes = Vec::new();
    let mut rng = Rng::new(0xA1_3);
    for _ in 0..300 {
        let n = 1 + rng.below(30);
        let mut entries = Vec::new();
        for _ in 0..n {
            // Draw keys from a small alphabet so prefixes collide often.
            let len = rng.below(5);
            let key: Vec<u8> = (0..len).map(|_| b"ab\xC3\xA9\xFF"[rng.below(5)]).collect();
            entries.push((key, Node::Int(rng.range(-100, 100))));
        }
        nodes.push(Node::Obj(entries));
    }
    // Objects large enough to straddle several hashtable rehash thresholds.
    for n in [8usize, 9, 16, 17, 32, 33, 64, 65, 128, 129] {
        nodes.push(Node::Obj(
            (0..n)
                .map(|i| (format!("k{:04}", (i * 7919) % n).into_bytes(), Node::Int(i as i64)))
                .collect(),
        ));
    }
    sweep(&nodes, &flags, "sort_keys/prefixes");
}

/// A16: JSON_PRESERVE_ORDER is referenced nowhere in the C — setting it must be
/// byte-identical to leaving it clear, in both libraries.
#[test]
fn a16_preserve_order_is_a_noop() {
    unsafe {
        let p = pair();
        for node in structural_corpus().iter().chain(random_corpus(0xA16, 200, 3).iter()) {
            with_both(node, |p2, cj, rj| {
                for base in [0usize, JSON_COMPACT, JSON_INDENT(3), JSON_SORT_KEYS] {
                    let a = p2.c.dumps(cj, base);
                    let b = p2.c.dumps(cj, base | JSON_PRESERVE_ORDER);
                    assert_eq!(a, b, "C: PRESERVE_ORDER changed output for {node:?}");
                    let ar = p2.r.dumps(rj, base);
                    let br = p2.r.dumps(rj, base | JSON_PRESERVE_ORDER);
                    assert_eq!(ar, br, "RUST: PRESERVE_ORDER changed output for {node:?}");
                    assert_eq!(a, ar, "C vs RUST for {node:?}");
                }
            });
        }
        let _ = p;
    }
}

/// A17, A18: JSON_ENCODE_ANY on (scalar roots accepted) and off (rejected).
#[test]
fn a17_a18_encode_any() {
    let flags = [
        JSON_ENCODE_ANY,
        JSON_ENCODE_ANY | JSON_COMPACT,
        JSON_ENCODE_ANY | JSON_INDENT(2),
        JSON_ENCODE_ANY | JSON_ENSURE_ASCII,
        JSON_ENCODE_ANY | JSON_SORT_KEYS,
    ];
    sweep(&scalar_corpus(), &flags, "encode_any");
    sweep(&structural_corpus(), &flags, "encode_any/containers");
    // Off: scalar roots must be rejected identically (see also ERRORS D43).
    unsafe {
        for node in scalar_corpus() {
            with_both(&node, |p, cj, rj| {
                for f in [0usize, JSON_COMPACT, JSON_INDENT(2)] {
                    let cd = p.c.dumps(cj, f);
                    let rd = p.r.dumps(rj, f);
                    assert_eq!(cd, rd, "scalar root without ENCODE_ANY: {node:?}");
                    assert!(cd.is_none(), "C should have rejected scalar root {node:?}");
                }
            });
        }
    }
}

/// A19, A20: JSON_REAL_PRECISION(p) for every p in 0..=31 over a broad double
/// corpus. p in 25..=31 makes every real fail (the `dtoa_r` scratch is 25 bytes),
/// which the two libraries must agree on.
#[test]
fn a19_a20_real_precision_sweep() {
    let mut rng = Rng::new(0xA19_A19);
    let mut vals: Vec<f64> = vec![
        0.0, -0.0, 1.0, -1.0, 0.5, -0.5, 2.0, 10.0, 100.0, 1e15, 1e16, 1e17, 1e18, 1e-3, 1e-4,
        1e-5, 1e-6, 1e300, 1e-300, f64::MAX, f64::MIN, f64::MIN_POSITIVE, 5e-324, f64::EPSILON,
        1.0 / 3.0, 2.0 / 3.0, 9007199254740992.0, 123456789.123456789, -987654321.5,
        3.141592653589793, 2.718281828459045,
    ];
    for _ in 0..2000 {
        vals.push(rng.finite_f64());
    }

    // Batch the values so each dump exercises many reals at once.
    let nodes: Vec<Node> = vals
        .chunks(32)
        .map(|c| Node::Arr(c.iter().map(|&v| Node::Real(v)).collect()))
        .collect();

    let mut flags = Vec::new();
    for prec in 0..=32usize {
        flags.push(JSON_REAL_PRECISION(prec));
        flags.push(JSON_REAL_PRECISION(prec) | JSON_COMPACT);
    }
    sweep(&nodes, &flags, "real_precision");

    // Also one real per dump, so a per-value failure is not masked by a batch.
    let singles: Vec<Node> = vals.iter().map(|&v| Node::Arr(vec![Node::Real(v)])).collect();
    sweep(&singles, &flags, "real_precision/single");
}

/// A22..A25: JSON_EMBED. The bit is stripped before recursion, so it affects
/// only the root; empty embedded containers emit *nothing*.
#[test]
fn a22_a25_embed() {
    let mut flags = Vec::new();
    for base in [
        0usize,
        JSON_COMPACT,
        JSON_INDENT(2),
        JSON_INDENT(4) | JSON_COMPACT,
        JSON_SORT_KEYS,
        JSON_ENSURE_ASCII,
    ] {
        flags.push(base | JSON_EMBED);
        flags.push(base | JSON_EMBED | JSON_ENCODE_ANY);
    }
    // A22/A23/A26: containers, incl. empty ones
    sweep(&structural_corpus(), &flags, "embed");
    // A24: scalar roots — EMBED is silently ignored
    sweep(&scalar_corpus(), &flags, "embed/scalar");
    // A25: nested containers keep their own brackets
    sweep(&random_corpus(0xA22, 300, 4), &flags, "embed/random");
}

/// A28: randomized sweep over the whole encode flag word at once, including
/// undefined high bits and `SIZE_MAX`.
#[test]
fn a28_random_flag_words() {
    let mut rng = Rng::new(0xA28_A28);
    let known = JSON_MAX_INDENT
        | JSON_COMPACT
        | JSON_ENSURE_ASCII
        | JSON_SORT_KEYS
        | JSON_PRESERVE_ORDER
        | JSON_ENCODE_ANY
        | JSON_ESCAPE_SLASH
        | JSON_REAL_PRECISION(31)
        | JSON_EMBED;
    let mut flags: Vec<usize> = vec![0, usize::MAX, known, !known];
    for _ in 0..400 {
        flags.push(match rng.below(3) {
            0 => rng.next_u64() as usize,           // fully random word
            1 => (rng.next_u64() as usize) & known, // random known bits
            _ => (rng.next_u64() as usize) | JSON_ENCODE_ANY,
        });
    }
    sweep(&structural_corpus(), &flags, "random flag word");
    sweep(&scalar_corpus(), &flags, "random flag word/scalar");
    sweep(&random_corpus(0xA28_2, 150, 3), &flags, "random flag word/random");
}

/// A37: deeply nested trees — encoding has no depth limit.
#[test]
fn a37_deep_nesting() {
    let mut deep = Node::Int(1);
    for _ in 0..100 {
        deep = Node::Arr(vec![deep]);
    }
    let mut deep_obj = Node::Int(1);
    for i in 0..100 {
        deep_obj = Node::Obj(vec![(format!("k{i}").into_bytes(), deep_obj)]);
    }
    let mut deeper = Node::Int(1);
    for _ in 0..1000 {
        deeper = Node::Arr(vec![deeper]);
    }
    sweep(
        &[deep, deep_obj, deeper],
        &[0, JSON_COMPACT, JSON_INDENT(1), JSON_INDENT(31), JSON_SORT_KEYS],
        "deep",
    );
}

// ===========================================================================
// A29, A30 — json_dumpb
// ===========================================================================

#[test]
fn a29_a30_json_dumpb() {
    unsafe {
        let flags: Vec<usize> = vec![
            0,
            JSON_COMPACT,
            JSON_INDENT(2),
            JSON_INDENT(31),
            JSON_SORT_KEYS,
            JSON_ENSURE_ASCII,
            JSON_ENCODE_ANY,
            JSON_EMBED,
            JSON_REAL_PRECISION(5),
            JSON_SORT_KEYS | JSON_INDENT(3) | JSON_ESCAPE_SLASH,
        ];
        let mut nodes = structural_corpus();
        nodes.extend(scalar_corpus());
        nodes.extend(random_corpus(0xA29, 200, 3));

        for node in &nodes {
            with_both(node, |p, cj, rj| {
                for &f in &flags {
                    let want = p.c.dumps(cj, f | JSON_ENCODE_ANY.min(f)); // reference only
                    let _ = want;
                    // Determine the required size from dumpb with size 0.
                    let cn = p.c.json_dumpb(cj, std::ptr::null_mut(), 0, f);
                    let rn = p.r.json_dumpb(rj, std::ptr::null_mut(), 0, f);
                    assert_eq!(
                        cn, rn,
                        "json_dumpb size-0 required length (flags 0x{f:x}) for {node:?}"
                    );
                    if cn == 0 {
                        continue; // both refused
                    }
                    // A30: must equal the json_dumps length.
                    if let Some(d) = p.c.dumps(cj, f) {
                        assert_eq!(
                            d.len(),
                            cn,
                            "C: dumpb length != dumps length (flags 0x{f:x})"
                        );
                    }
                    // A29: exact, short by one, long by one, and every size in a
                    // window around the required length (chunk-skip behaviour).
                    let mut sizes: Vec<usize> = vec![0, 1, 2, cn, cn + 1, cn + 100];
                    if cn > 0 {
                        sizes.push(cn - 1);
                    }
                    for s in 0..cn.min(80) {
                        sizes.push(s);
                    }
                    for &size in &sizes {
                        let mut cbuf = vec![0xA5u8; size + 8];
                        let mut rbuf = vec![0xA5u8; size + 8];
                        let cr = p.c.json_dumpb(cj, cbuf.as_mut_ptr() as *mut c_char, size, f);
                        let rr = p.r.json_dumpb(rj, rbuf.as_mut_ptr() as *mut c_char, size, f);
                        assert_eq!(
                            cr, rr,
                            "json_dumpb(size {size}, flags 0x{f:x}) return for {node:?}"
                        );
                        assert_eq!(
                            cbuf, rbuf,
                            "json_dumpb(size {size}, flags 0x{f:x}) buffer for {node:?}:\n\
                             C   = {:?}\nRUST= {:?}",
                            String::from_utf8_lossy(&cbuf),
                            String::from_utf8_lossy(&rbuf)
                        );
                    }
                }
            });
        }
    }
}

// ===========================================================================
// A31..A33 — json_dumpf / json_dumpfd / json_dump_file
// ===========================================================================

fn tmp_path(tag: &str) -> std::path::PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!(
        "jansson_diff_{tag}_{}_{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    d
}

#[test]
fn a31_a33_file_and_fd_dumps() {
    unsafe {
        let p = pair();
        let flags: Vec<usize> = vec![
            0,
            JSON_COMPACT,
            JSON_INDENT(2),
            JSON_SORT_KEYS,
            JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
            JSON_ENCODE_ANY,
            JSON_EMBED,
        ];
        let mut nodes = structural_corpus();
        nodes.extend(scalar_corpus());
        nodes.extend(random_corpus(0xA31, 80, 3));

        // Each library must open its own FILE*/fd, since a FILE* created by one
        // libc must not be handed to the other library's fclose.
        let cfopen = p.c.lib.get::<unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void>(b"fopen\0").ok();
        let _ = cfopen;

        for node in &nodes {
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            for &f in &flags {
                // --- A33: json_dump_file
                let cpath = tmp_path("c_df");
                let rpath = tmp_path("r_df");
                let cps = cs(cpath.to_str().unwrap());
                let rps = cs(rpath.to_str().unwrap());
                let cr = p.c.json_dump_file(cj, cps.as_ptr(), f);
                let rr = p.r.json_dump_file(rj, rps.as_ptr(), f);
                assert_eq!(cr, rr, "json_dump_file return (flags 0x{f:x}) {node:?}");
                let cb = std::fs::read(&cpath).unwrap_or_default();
                let rb = std::fs::read(&rpath).unwrap_or_default();
                assert_eq!(
                    cb, rb,
                    "json_dump_file bytes (flags 0x{f:x}) {node:?}:\nC   = {:?}\nRUST= {:?}",
                    String::from_utf8_lossy(&cb),
                    String::from_utf8_lossy(&rb)
                );
                let _ = std::fs::remove_file(&cpath);
                let _ = std::fs::remove_file(&rpath);

                // --- A32: json_dumpfd, via a real file descriptor
                let cpath = tmp_path("c_fd");
                let rpath = tmp_path("r_fd");
                {
                    use std::os::unix::io::AsRawFd;
                    let cfile = std::fs::File::create(&cpath).unwrap();
                    let rfile = std::fs::File::create(&rpath).unwrap();
                    let cr = p.c.json_dumpfd(cj, cfile.as_raw_fd(), f);
                    let rr = p.r.json_dumpfd(rj, rfile.as_raw_fd(), f);
                    assert_eq!(cr, rr, "json_dumpfd return (flags 0x{f:x}) {node:?}");
                }
                let cb = std::fs::read(&cpath).unwrap_or_default();
                let rb = std::fs::read(&rpath).unwrap_or_default();
                assert_eq!(cb, rb, "json_dumpfd bytes (flags 0x{f:x}) {node:?}");
                let _ = std::fs::remove_file(&cpath);
                let _ = std::fs::remove_file(&rpath);

                // --- A31: json_dumpf, via each library's own fopen
                let cpath = tmp_path("c_f");
                let rpath = tmp_path("r_f");
                {
                    let cfp = libc_fopen(&cpath, "w");
                    let rfp = libc_fopen(&rpath, "w");
                    let cr = p.c.json_dumpf(cj, cfp, f);
                    let rr = p.r.json_dumpf(rj, rfp, f);
                    assert_eq!(cr, rr, "json_dumpf return (flags 0x{f:x}) {node:?}");
                    libc_fclose(cfp);
                    libc_fclose(rfp);
                }
                let cb = std::fs::read(&cpath).unwrap_or_default();
                let rb = std::fs::read(&rpath).unwrap_or_default();
                assert_eq!(cb, rb, "json_dumpf bytes (flags 0x{f:x}) {node:?}");
                let _ = std::fs::remove_file(&cpath);
                let _ = std::fs::remove_file(&rpath);
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
    }
}

// Both `.so`s link the same system libc, so one `fopen`/`fclose` pair is
// correct for both.
extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
}

fn libc_fopen(path: &std::path::Path, mode: &str) -> *mut c_void {
    let p = cs(path.to_str().unwrap());
    let m = cs(mode);
    let f = unsafe { fopen(p.as_ptr(), m.as_ptr()) };
    assert!(!f.is_null(), "fopen({path:?}) failed");
    f
}
fn libc_fclose(f: *mut c_void) {
    unsafe {
        fflush(f);
        fclose(f);
    }
}

// ===========================================================================
// A34, A35 — json_dump_callback (the low-level entry point)
// ===========================================================================

struct Recorder {
    chunks: Vec<Vec<u8>>,
    /// Abort (return non-zero) on the 1-based call number `fail_at`, if > 0.
    fail_at: usize,
    calls: usize,
}

unsafe extern "C" fn record_cb(buf: *const c_char, size: usize, data: *mut c_void) -> c_int {
    let r = &mut *(data as *mut Recorder);
    r.calls += 1;
    if r.fail_at != 0 && r.calls == r.fail_at {
        return -7;
    }
    r.chunks
        .push(std::slice::from_raw_parts(buf as *const u8, size).to_vec());
    0
}

fn dump_chunks(l: &Lib, j: json_ptr, flags: usize, fail_at: usize) -> (c_int, Vec<Vec<u8>>, usize) {
    let mut rec = Recorder {
        chunks: Vec::new(),
        fail_at,
        calls: 0,
    };
    let r = unsafe {
        l.json_dump_callback(j, Some(record_cb), &mut rec as *mut _ as *mut c_void, flags)
    };
    (r, rec.chunks, rec.calls)
}

/// A34: the *chunk boundary sequence* must match, not just the concatenation.
/// This is what catches a differently-batched `dump_indent` or escape run.
#[test]
fn a34_dump_callback_chunk_sequence() {
    unsafe {
        let flags: Vec<usize> = vec![
            0,
            JSON_COMPACT,
            JSON_INDENT(1),
            JSON_INDENT(2),
            JSON_INDENT(31),
            JSON_INDENT(31) | JSON_COMPACT,
            JSON_SORT_KEYS,
            JSON_ENSURE_ASCII,
            JSON_ESCAPE_SLASH,
            JSON_ENCODE_ANY,
            JSON_EMBED,
            JSON_REAL_PRECISION(7),
        ];
        let mut nodes = structural_corpus();
        nodes.extend(scalar_corpus());
        nodes.extend(random_corpus(0xA34, 250, 4));
        // A very deep tree with indent 31 maximizes the multi-chunk indent path.
        let mut deep = Node::Arr(vec![Node::Int(1), Node::Int(2)]);
        for _ in 0..6 {
            deep = Node::Arr(vec![deep.clone(), Node::Int(0)]);
        }
        nodes.push(deep);

        for node in &nodes {
            with_both(node, |p, cj, rj| {
                for &f in &flags {
                    let (cr, cc, ccalls) = dump_chunks(&p.c, cj, f, 0);
                    let (rr, rc, rcalls) = dump_chunks(&p.r, rj, f, 0);
                    assert_eq!(cr, rr, "dump_callback return (flags 0x{f:x}) {node:?}");
                    assert_eq!(
                        ccalls, rcalls,
                        "dump_callback call count (flags 0x{f:x}) {node:?}"
                    );
                    assert_eq!(
                        cc, rc,
                        "dump_callback chunk sequence (flags 0x{f:x}) {node:?}:\n\
                         C   = {:?}\nRUST= {:?}",
                        cc.iter().map(|c| String::from_utf8_lossy(c).into_owned()).collect::<Vec<_>>(),
                        rc.iter().map(|c| String::from_utf8_lossy(c).into_owned()).collect::<Vec<_>>()
                    );
                }
            });
        }
    }
}

/// A35 / ERRORS D45: a callback that fails on the k-th chunk, for every k, must
/// produce the same `-1` and the same already-emitted prefix.
#[test]
fn a35_dump_callback_failure_at_every_chunk() {
    unsafe {
        let flags = [
            0usize,
            JSON_COMPACT,
            JSON_INDENT(3),
            JSON_SORT_KEYS,
            JSON_ENSURE_ASCII,
            JSON_ENCODE_ANY,
            JSON_EMBED,
        ];
        let mut nodes = structural_corpus();
        nodes.extend(scalar_corpus());
        nodes.extend(random_corpus(0xA35, 60, 3));

        for node in &nodes {
            with_both(node, |p, cj, rj| {
                for &f in &flags {
                    let (_, _, total) = dump_chunks(&p.c, cj, f, 0);
                    for k in 1..=total.min(60) {
                        let (cr, cc, _) = dump_chunks(&p.c, cj, f, k);
                        let (rr, rc, _) = dump_chunks(&p.r, rj, f, k);
                        assert_eq!(
                            cr, rr,
                            "dump_callback fail@{k} return (flags 0x{f:x}) {node:?}"
                        );
                        assert_eq!(
                            cc, rc,
                            "dump_callback fail@{k} emitted prefix (flags 0x{f:x}) {node:?}"
                        );
                    }
                }
            });
        }
    }
}

// Keep `Read` imported for the file comparisons above without a warning.
#[allow(dead_code)]
fn _read_marker<R: Read>(_r: R) {}

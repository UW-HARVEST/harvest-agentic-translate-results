//! Differential tests for `pack_unpack.c` — CONFIGS.md rows 112-128,
//! ERRORS.md rows 157-199.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;

/// `dtoa.c` is compiled WITHOUT `MULTIPLE_THREADS`, so `Balloc`'s `freelist`,
/// `p5s` and `dtoa_result` are unsynchronised mutable statics in BOTH libraries.
/// Any test that formats a real number must therefore run exclusively.
fn lock() -> std::sync::MutexGuard<'static, ()> {
    static L: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match L.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

fn big_stack<F: FnOnce() + Send + 'static>(f: F) {
    let _g = lock();
    std::thread::Builder::new()
        .stack_size(96 * 1024 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

// ---------------------------------------------------------------------------
// Argument description — the same logical argument list is materialised twice,
// once per library (json_t* args must belong to the library being called).
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum A {
    /// `const char *` pointing at a NUL-terminated copy of these bytes
    S(Vec<u8>),
    /// `const char *` == NULL
    SNull,
    /// C `int`
    I(c_int),
    /// `json_int_t` (long long)
    L(i64),
    /// `size_t`
    Z(usize),
    /// `double`
    D(f64),
    /// `json_t *` for `O` (the callee increfs, we keep our reference)
    JO(&'static str),
    /// `json_t *` for `o` (the callee steals, so we incref before passing)
    Jo(&'static str),
    /// `json_t *` == NULL
    JNull,
}

/// Materialised arguments for one library. Must stay alive across the call.
struct Bound {
    strings: Vec<Vec<u8>>,
    values: Vec<*mut json_t>,
    words: Vec<u64>,
}

fn bind(l: &Lib, args: &[A]) -> Bound {
    let mut b = Bound {
        strings: Vec::new(),
        values: Vec::new(),
        words: Vec::new(),
    };
    // First materialise everything so the addresses are stable.
    for a in args {
        match a {
            A::S(bytes) => {
                let mut v = bytes.clone();
                v.push(0);
                b.strings.push(v);
            }
            A::JO(text) | A::Jo(text) => unsafe {
                let z = cs(text);
                let j = (l.json_loads)(z.as_ptr(), JSON_DECODE_ANY | JSON_ALLOW_NUL, ptr::null_mut());
                assert!(!j.is_null(), "{}: cannot build arg {:?}", l.which, text);
                b.values.push(j);
            },
            _ => {}
        }
    }
    let mut si = 0usize;
    let mut vi = 0usize;
    for a in args {
        match a {
            A::S(_) => {
                b.words.push(b.strings[si].as_ptr() as usize as u64);
                si += 1;
            }
            A::SNull => b.words.push(0),
            A::I(v) => b.words.push(*v as u32 as u64),
            A::L(v) => b.words.push(*v as u64),
            A::Z(v) => b.words.push(*v as u64),
            A::D(v) => b.words.push(v.to_bits()),
            A::JO(_) => {
                b.words.push(b.values[vi] as usize as u64);
                vi += 1;
            }
            A::Jo(_) => {
                // the callee steals the reference, so keep one for ourselves
                b.words.push(incref(b.values[vi]) as usize as u64);
                vi += 1;
            }
            A::JNull => b.words.push(0),
        }
    }
    b
}

impl Bound {
    fn va(&mut self) -> (*mut VaListTag, Box<VaListTag>) {
        for _ in 0..16 {
            self.words.push(0);
        }
        let mut tag = Box::new(VaListTag {
            gp_offset: 48,
            fp_offset: 176,
            overflow_arg_area: self.words.as_mut_ptr() as *mut c_void,
            reg_save_area: ptr::null_mut(),
        });
        let p = &mut *tag as *mut VaListTag;
        (p, tag)
    }
    fn release(self, l: &Lib) {
        for v in self.values {
            decref(l, v);
        }
    }
}

/// Drive `json_vpack_ex` on both libraries with the same logical arguments and
/// compare every observable.
#[track_caller]
fn cmp_pack(d: &Duo, tag: &str, fmt: &str, args: &[A], flags: usize) {
    let f = cs(fmt);
    unsafe {
        for use_err in [true, false] {
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            let cep = if use_err { &mut ce as *mut json_error_t } else { ptr::null_mut() };
            let rep = if use_err { &mut re as *mut json_error_t } else { ptr::null_mut() };

            let mut cb = bind(&d.c, args);
            let (cap, _c_keep) = cb.va();
            let cj = (d.c.json_vpack_ex)(cep, flags, f.as_ptr(), cap);

            let mut rb = bind(&d.rs, args);
            let (rap, _r_keep) = rb.va();
            let rj = (d.rs.json_vpack_ex)(rep, flags, f.as_ptr(), rap);

            let what = format!("vpack {} fmt={:?} flags={:#x} err={}", tag, fmt, flags, use_err);
            eq(&format!("{} null", what), cj.is_null(), rj.is_null());
            if use_err {
                eq_err(&what, &ce, &re);
            }
            if !cj.is_null() {
                eq(&format!("{} tree", what), describe(&d.c, cj), describe(&d.rs, rj));
                let (cd, rd) = dumps_both(d, cj, rj, JSON_ENCODE_ANY | JSON_SORT_KEYS);
                eq(&format!("{} dump null", what), cd.is_none(), rd.is_none());
                if let (Some(a), Some(bb)) = (&cd, &rd) {
                    eq_bytes(&format!("{} dump", what), a, bb);
                }
            }
            decref(&d.c, cj);
            decref(&d.rs, rj);
            cb.release(&d.c);
            rb.release(&d.rs);
        }
    }
}

// ---------------------------------------------------------------------------
// Unpack: out-parameters. Every slot is a pointer to a distinct writable 8-byte
// cell, which is simultaneously valid as `int*`, `json_int_t*`, `double*`,
// `size_t*`, `const char**`, `json_t**` and (when read as `const char*`) an
// empty string. Cells are zeroed before each call and compared afterwards.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum U {
    /// an object key: `const char *`
    Key(&'static str),
    /// `const char *` == NULL (a NULL key)
    KeyNull,
    /// an out-pointer to a writable cell
    Out,
    /// an out-pointer == NULL
    OutNull,
}

struct UBound {
    keys: Vec<CString>,
    cells: Box<[u64; 64]>,
    words: Vec<u64>,
    n_out: usize,
}

fn ubind(args: &[U]) -> UBound {
    let mut b = UBound {
        keys: Vec::new(),
        cells: Box::new([0u64; 64]),
        words: Vec::new(),
        n_out: 0,
    };
    for a in args {
        if let U::Key(k) = a {
            b.keys.push(cs(k));
        }
    }
    let mut ki = 0usize;
    let mut oi = 0usize;
    for a in args {
        match a {
            U::Key(_) => {
                b.words.push(b.keys[ki].as_ptr() as usize as u64);
                ki += 1;
            }
            U::KeyNull => b.words.push(0),
            U::Out => {
                b.words.push(b.cells.as_ptr().wrapping_add(oi) as usize as u64);
                oi += 1;
            }
            U::OutNull => b.words.push(0),
        }
    }
    b.n_out = oi;
    b
}

impl UBound {
    fn va(&mut self) -> (*mut VaListTag, Box<VaListTag>) {
        for _ in 0..16 {
            self.words.push(0);
        }
        let mut tag = Box::new(VaListTag {
            gp_offset: 48,
            fp_offset: 176,
            overflow_arg_area: self.words.as_mut_ptr() as *mut c_void,
            reg_save_area: ptr::null_mut(),
        });
        let p = &mut *tag as *mut VaListTag;
        (p, tag)
    }
}

/// Compare a cell as raw bits, and (when it looks like a pointer into the root's
/// string data) as the C string it points to. Raw pointer values differ between
/// the libraries, so string cells are compared by CONTENT.
fn cell_view(l: &Lib, cell: u64, as_str: bool, as_json: bool) -> String {
    if as_str {
        if cell == 0 {
            return "str:<null>".into();
        }
        return format!("str:{:?}", cstr_bytes(cell as usize as *const c_char));
    }
    if as_json {
        if cell == 0 {
            return "json:<null>".into();
        }
        return format!("json:{}", describe(l, cell as usize as *const json_t));
    }
    format!("raw:{:#018x}", cell)
}

/// `json_vunpack_ex` on both libraries. `str_cells` / `json_cells` say which out
/// cells hold pointers whose *target* must be compared instead of the pointer.
#[track_caller]
fn cmp_unpack(
    d: &Duo,
    tag: &str,
    root_text: &str,
    fmt: &str,
    args: &[U],
    flags: usize,
    str_cells: &[usize],
    json_cells: &[usize],
) {
    let f = cs(fmt);
    let rz = cs(root_text);
    unsafe {
        for use_err in [true, false] {
            let croot = (d.c.json_loads)(rz.as_ptr(), JSON_DECODE_ANY | JSON_ALLOW_NUL, ptr::null_mut());
            let rroot = (d.rs.json_loads)(rz.as_ptr(), JSON_DECODE_ANY | JSON_ALLOW_NUL, ptr::null_mut());
            assert!(!croot.is_null() && !rroot.is_null(), "bad root {:?}", root_text);

            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            let cep = if use_err { &mut ce as *mut json_error_t } else { ptr::null_mut() };
            let rep = if use_err { &mut re as *mut json_error_t } else { ptr::null_mut() };

            let mut cb = ubind(args);
            let (cap, _ck) = cb.va();
            let crc = (d.c.json_vunpack_ex)(croot, cep, flags, f.as_ptr(), cap);

            let mut rb = ubind(args);
            let (rap, _rk) = rb.va();
            let rrc = (d.rs.json_vunpack_ex)(rroot, rep, flags, f.as_ptr(), rap);

            let what = format!(
                "vunpack {} root={:?} fmt={:?} flags={:#x} err={}",
                tag, root_text, fmt, flags, use_err
            );
            eq(&format!("{} ret", what), crc, rrc);
            if use_err {
                eq_err(&what, &ce, &re);
            }
            for i in 0..cb.n_out {
                let s = str_cells.contains(&i);
                let j = json_cells.contains(&i);
                eq(
                    &format!("{} cell[{}]", what, i),
                    cell_view(&d.c, cb.cells[i], s, j),
                    cell_view(&d.rs, rb.cells[i], s, j),
                );
            }
            // `O` increments the root's refcount; compare the root afterwards.
            eq(
                &format!("{} root after", what),
                describe(&d.c, croot),
                describe(&d.rs, rroot),
            );
            decref(&d.c, croot);
            decref(&d.rs, rroot);
        }
    }
}

// ===========================================================================
// CONFIGS 112-119 — pack
// ===========================================================================

#[test]
fn pack_every_format_char() {
    let d = duo();
    let _g = lock();
    let k = |s: &str| A::S(s.as_bytes().to_vec());
    // (tag, fmt, args)
    let cases: Vec<(&str, &str, Vec<A>)> = vec![
        ("obj-empty", "{}", vec![]),
        ("arr-empty", "[]", vec![]),
        ("s", "s", vec![k("hello")]),
        ("s-empty", "s", vec![k("")]),
        ("s-utf8", "s", vec![k("héllo €𝄞")]),
        ("n", "n", vec![]),
        ("b-true", "b", vec![A::I(1)]),
        ("b-false", "b", vec![A::I(0)]),
        ("b-neg", "b", vec![A::I(-1)]),
        ("b-max", "b", vec![A::I(c_int::MAX)]),
        ("i-0", "i", vec![A::I(0)]),
        ("i-max", "i", vec![A::I(c_int::MAX)]),
        ("i-min", "i", vec![A::I(c_int::MIN)]),
        ("I-0", "I", vec![A::L(0)]),
        ("I-max", "I", vec![A::L(i64::MAX)]),
        ("I-min", "I", vec![A::L(i64::MIN)]),
        ("f-0", "f", vec![A::D(0.0)]),
        ("f-neg0", "f", vec![A::D(-0.0)]),
        ("f-half", "f", vec![A::D(0.5)]),
        ("f-big", "f", vec![A::D(1e300)]),
        ("O-obj", "O", vec![A::JO(r#"{"a":1}"#)]),
        ("O-scalar", "O", vec![A::JO("42")]),
        ("o-obj", "o", vec![A::Jo(r#"[1,2]"#)]),
        ("o-scalar", "o", vec![A::Jo("true")]),
        ("obj-1", "{s:i}", vec![k("a"), A::I(1)]),
        (
            "obj-all",
            "{s:n,s:b,s:i,s:I,s:f,s:s,s:[],s:{}}",
            vec![
                k("n"),
                k("b"),
                A::I(1),
                k("i"),
                A::I(2),
                k("I"),
                A::L(3),
                k("f"),
                A::D(4.5),
                k("s"),
                k("v"),
                k("a"),
                k("o"),
            ],
        ),
        ("arr-1", "[i]", vec![A::I(7)]),
        (
            "arr-all",
            "[n,b,i,I,f,s,[],{}]",
            vec![A::I(1), A::I(2), A::L(3), A::D(4.5), k("v")],
        ),
        (
            "nested-3",
            "{s:{s:[i,i,i]}}",
            vec![k("a"), k("b"), A::I(1), A::I(2), A::I(3)],
        ),
        (
            "nested-arr",
            "[[[i]],[{s:i}],{s:[i]}]",
            vec![A::I(1), k("k"), A::I(2), k("m"), A::I(3)],
        ),
        // CONFIGS 117: whitespace and , : separators are skipped
        ("sep-ws", "  {  s  :  i  }  ", vec![k("a"), A::I(1)]),
        ("sep-commas", "{,,s,,:,,i,,}", vec![k("a"), A::I(1)]),
        ("sep-colons", "[::i::i::]", vec![A::I(1), A::I(2)]),
        ("sep-newline", "{\ns\n:\ni\n}", vec![k("a"), A::I(1)]),
        ("sep-tab", "[\ti\t]", vec![A::I(1)]),
        // duplicate keys: last one wins (json_object_setn_new_nocheck)
        ("dup-keys", "{s:i,s:i}", vec![k("a"), A::I(1), k("a"), A::I(2)]),
        // keys with UTF-8 and empty keys
        ("key-utf8", "{s:i}", vec![k("kéy€"), A::I(1)]),
        ("key-empty", "{s:i}", vec![k(""), A::I(1)]),
    ];
    for (tag, fmt, args) in &cases {
        for flags in [0usize, 1 << 20, usize::MAX] {
            cmp_pack(d, tag, fmt, args, flags);
        }
    }
}

/// CONFIGS 114: `s#`, `s%`, `s+`, `s+#`, `s+%`.
#[test]
fn pack_string_length_and_concat() {
    let d = duo();
    let _g = lock();
    let k = |s: &str| A::S(s.as_bytes().to_vec());
    let cases: Vec<(&str, &str, Vec<A>)> = vec![
        ("s#-full", "s#", vec![k("hello"), A::I(5)]),
        ("s#-short", "s#", vec![k("hello"), A::I(3)]),
        ("s#-zero", "s#", vec![k("hello"), A::I(0)]),
        ("s#-one", "s#", vec![k("hello"), A::I(1)]),
        ("s%-full", "s%", vec![k("hello"), A::Z(5)]),
        ("s%-short", "s%", vec![k("hello"), A::Z(2)]),
        ("s%-zero", "s%", vec![k("hello"), A::Z(0)]),
        ("s+2", "s+", vec![k("ab"), k("cd")]),
        ("s+3", "s++", vec![k("a"), k("b"), k("c")]),
        ("s+empty", "s+", vec![k(""), k("")]),
        ("s+#", "s+#", vec![k("abc"), k("defgh"), A::I(2)]),
        ("s#+", "s#+", vec![k("abc"), A::I(2), k("de")]),
        ("s#+#", "s#+#", vec![k("abc"), A::I(2), k("defgh"), A::I(3)]),
        ("s%+%", "s%+%", vec![k("abc"), A::Z(2), k("defgh"), A::Z(3)]),
        ("s+-utf8", "s+", vec![k("hé"), k("€𝄞")]),
        // split a multi-byte sequence across the concat boundary: the halves are
        // invalid on their own but the concatenation is valid UTF-8
        (
            "s+-split-utf8",
            "s+#",
            vec![A::S(vec![0xE2, 0x82]), A::S(vec![0xAC]), A::I(1)],
        ),
        ("obj-s#", "{s#:i}", vec![k("abcd"), A::I(2), A::I(9)]),
        ("obj-s%", "{s%:i}", vec![k("abcd"), A::Z(3), A::I(9)]),
        ("obj-s+", "{s+:i}", vec![k("ab"), k("cd"), A::I(9)]),
        ("arr-s#", "[s#,s#]", vec![k("abc"), A::I(1), k("def"), A::I(2)]),
    ];
    for (tag, fmt, args) in &cases {
        cmp_pack(d, tag, fmt, args, 0);
    }
    // `s#` with a length longer than the string: the C copies `length` bytes
    // from the buffer, so give it a buffer that really is that long.
    cmp_pack(
        d,
        "s#-embedded-nul",
        "s#",
        &[A::S(b"ab\0cd".to_vec()), A::I(5)],
        0,
    );
    cmp_pack(
        d,
        "s%-embedded-nul",
        "s%",
        &[A::S(b"ab\0cd".to_vec()), A::Z(5)],
        0,
    );
}

/// CONFIGS 115: the optional modifiers `?` and `*`.
#[test]
fn pack_optional_modifiers() {
    let d = duo();
    let _g = lock();
    let k = |s: &str| A::S(s.as_bytes().to_vec());
    let cases: Vec<(&str, &str, Vec<A>)> = vec![
        ("s?-set", "s?", vec![k("v")]),
        ("s?-null", "s?", vec![A::SNull]),
        ("s*-set", "s*", vec![k("v")]),
        ("s*-null", "s*", vec![A::SNull]),
        ("O?-set", "O?", vec![A::JO("1")]),
        ("O?-null", "O?", vec![A::JNull]),
        ("O*-set", "O*", vec![A::JO("1")]),
        ("O*-null", "O*", vec![A::JNull]),
        ("o?-set", "o?", vec![A::Jo("1")]),
        ("o?-null", "o?", vec![A::JNull]),
        ("o*-set", "o*", vec![A::Jo("1")]),
        ("o*-null", "o*", vec![A::JNull]),
        // inside objects: `*` drops the member, `?` inserts null
        ("obj-s?-null", "{s:s?}", vec![k("a"), A::SNull]),
        ("obj-s*-null", "{s:s*}", vec![k("a"), A::SNull]),
        ("obj-s*-null-2", "{s:s*,s:i}", vec![k("a"), A::SNull, k("b"), A::I(1)]),
        ("obj-O*-null", "{s:O*}", vec![k("a"), A::JNull]),
        ("obj-O?-null", "{s:O?}", vec![k("a"), A::JNull]),
        ("obj-o*-null", "{s:o*}", vec![k("a"), A::JNull]),
        // inside arrays
        ("arr-s?-null", "[s?]", vec![A::SNull]),
        ("arr-s*-null", "[s*]", vec![A::SNull]),
        ("arr-s*-mix", "[i,s*,i]", vec![A::I(1), A::SNull, A::I(2)]),
        ("arr-O*-null", "[O*]", vec![A::JNull]),
        ("arr-O?-null", "[O?]", vec![A::JNull]),
        ("arr-o*-null", "[o*]", vec![A::JNull]),
        (
            "obj-many-optional",
            "{s:s*,s:s*,s:s*}",
            vec![k("a"), A::SNull, k("b"), k("v"), k("c"), A::SNull],
        ),
    ];
    for (tag, fmt, args) in &cases {
        cmp_pack(d, tag, fmt, args, 0);
    }
}

/// CONFIGS 119: randomized doubles through `f`.
#[test]
fn pack_randomized_reals_and_integers() {
    let d = duo();
    let _g = lock();
    let mut rng = Rng::new(0x9AC4_F00D);
    for _ in 0..1200 {
        let v = if rng.bool() { rng.tame_f64() } else { rng.finite_f64() };
        cmp_pack(d, "f", "f", &[A::D(v)], 0);
        cmp_pack(d, "arr-f", "[f,f]", &[A::D(v), A::D(-v)], 0);
        let i = rng.next_u64() as i64;
        cmp_pack(d, "I", "I", &[A::L(i)], 0);
        cmp_pack(d, "i", "i", &[A::I(i as c_int)], 0);
        cmp_pack(d, "b", "b", &[A::I(i as c_int)], 0);
    }
    // NaN / Inf must be rejected (ERRORS 168)
    for v in [f64::NAN, -f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        cmp_pack(d, "f-nonfinite", "f", &[A::D(v)], 0);
        cmp_pack(d, "obj-f-nonfinite", "{s:f}", &[A::S(b"a".to_vec()), A::D(v)], 0);
        cmp_pack(d, "arr-f-nonfinite", "[f]", &[A::D(v)], 0);
        cmp_pack(d, "arr-f-nonfinite*", "[f*]", &[A::D(v)], 0);
    }
}

// ===========================================================================
// ERRORS 157-172 — pack rejections
// ===========================================================================

#[test]
fn pack_error_paths() {
    let d = duo();
    let _g = lock();
    let k = |s: &str| A::S(s.as_bytes().to_vec());
    let bad_utf8 = |b: Vec<u8>| A::S(b);
    let cases: Vec<(&str, &str, Vec<A>)> = vec![
        // 157: NULL string, not optional
        ("157-s-null", "s", vec![A::SNull]),
        ("157-obj-key-null", "{s:i}", vec![A::SNull, A::I(1)]),
        ("157-obj-val-null", "{s:s}", vec![k("a"), A::SNull]),
        ("157-arr-null", "[s]", vec![A::SNull]),
        // 158: invalid UTF-8
        ("158-s-bad", "s", vec![bad_utf8(vec![0x80])]),
        ("158-s-bad2", "s", vec![bad_utf8(vec![0xC0, 0x80])]),
        ("158-s-bad3", "s", vec![bad_utf8(vec![0xED, 0xA0, 0x80])]),
        ("158-s-bad4", "s", vec![bad_utf8(vec![0xF5, 0x80, 0x80, 0x80])]),
        ("158-key-bad", "{s:i}", vec![bad_utf8(vec![0x80]), A::I(1)]),
        ("158-arr-bad", "[s]", vec![bad_utf8(vec![0xC2])]),
        // 159: '#' / '%' / '+' on an optional string
        ("159-s?#", "s?#", vec![k("ab"), A::I(1)]),
        ("159-s?%", "s?%", vec![k("ab"), A::Z(1)]),
        ("159-s?+", "s?+", vec![k("ab"), k("cd")]),
        ("159-s*#", "s*#", vec![k("ab"), A::I(1)]),
        ("159-s*%", "s*%", vec![k("ab"), A::Z(1)]),
        ("159-s*+", "s*+", vec![k("ab"), k("cd")]),
        ("159-obj-s?#", "{s:s?#}", vec![k("a"), k("ab"), A::I(1)]),
        // 160: concat path with a NULL part
        ("160-concat-null-1", "s+", vec![A::SNull, k("cd")]),
        ("160-concat-null-2", "s+", vec![k("ab"), A::SNull]),
        ("160-concat-null-both", "s+", vec![A::SNull, A::SNull]),
        ("160-concat-null-3", "s++", vec![k("a"), A::SNull, k("c")]),
        // 161: concat result invalid UTF-8
        (
            "161-concat-bad",
            "s+",
            vec![bad_utf8(vec![0xE2, 0x82]), k("x")],
        ),
        (
            "161-concat-bad2",
            "s+#",
            vec![k("a"), bad_utf8(vec![0x80, 0x80]), A::I(2)],
        ),
        // 162: unexpected end of format string in an object
        ("162-obj-eof", "{", vec![]),
        ("162-obj-eof-2", "{s", vec![k("a")]),
        ("162-obj-eof-3", "{s:", vec![k("a")]),
        ("162-obj-eof-4", "{s:i", vec![k("a"), A::I(1)]),
        ("162-obj-eof-nested", "{s:{", vec![k("a")]),
        // 163: object key spec is not 's'
        ("163-key-i", "{i:i}", vec![A::I(1), A::I(2)]),
        ("163-key-n", "{n:i}", vec![A::I(1)]),
        ("163-key-brace", "{{}:i}", vec![A::I(1)]),
        ("163-key-arr", "{[]:i}", vec![A::I(1)]),
        ("163-key-O", "{O:i}", vec![A::JO("1"), A::I(1)]),
        // 164: NULL object value without '*'
        ("164-obj-val-o-null", "{s:o}", vec![k("a"), A::JNull]),
        ("164-obj-val-O-null", "{s:O}", vec![k("a"), A::JNull]),
        // 165: unexpected end of format string in an array
        ("165-arr-eof", "[", vec![]),
        ("165-arr-eof-2", "[i", vec![A::I(1)]),
        ("165-arr-eof-nested", "[[", vec![]),
        ("165-arr-eof-obj", "[{", vec![]),
        // 166: NULL array value without '*'
        ("166-arr-o-null", "[o]", vec![A::JNull]),
        ("166-arr-O-null", "[O]", vec![A::JNull]),
        // 167: NULL o/O at top level
        ("167-o-null", "o", vec![A::JNull]),
        ("167-O-null", "O", vec![A::JNull]),
        // 169: unknown format characters
        ("169-at", "@", vec![]),
        ("169-hash", "#", vec![]),
        ("169-percent", "%", vec![]),
        ("169-plus", "+", vec![]),
        ("169-question", "?", vec![]),
        ("169-star", "*", vec![]),
        ("169-bang", "!", vec![]),
        ("169-F", "F", vec![]),
        ("169-x", "x", vec![]),
        ("169-close-brace", "}", vec![]),
        ("169-close-bracket", "]", vec![]),
        ("169-digit", "1", vec![]),
        ("169-in-obj", "{s:@}", vec![k("a")]),
        ("169-in-arr", "[@]", vec![]),
        ("169-in-arr-2", "[i,@]", vec![A::I(1)]),
        // 172: garbage after the format string
        ("172-garbage", "{s:i}x", vec![k("a"), A::I(1)]),
        ("172-garbage-2", "[i]]", vec![A::I(1)]),
        ("172-garbage-3", "ii", vec![A::I(1), A::I(2)]),
        ("172-garbage-4", "s s", vec![k("a"), k("b")]),
        ("172-garbage-5", "{}{}", vec![]),
        ("172-garbage-brace", "{}}", vec![]),
        // separators only / whitespace only
        ("sep-only-space", " ", vec![]),
        ("sep-only-comma", ",", vec![]),
        ("sep-only-mixed", " , : \t\n", vec![]),
    ];
    for (tag, fmt, args) in &cases {
        for flags in [0usize, 1 << 20] {
            cmp_pack(d, tag, fmt, args, flags);
        }
    }
}

/// ERRORS 170-171: `fmt == NULL` and `fmt == ""`.
#[test]
fn pack_null_and_empty_format() {
    let d = duo();
    let _g = lock();
    unsafe {
        for flags in [0usize, 1, 2, usize::MAX] {
            // fmt == NULL
            for use_err in [true, false] {
                let mut ce = json_error_t::new();
                let mut re = json_error_t::new();
                let cep = if use_err { &mut ce as *mut json_error_t } else { ptr::null_mut() };
                let rep = if use_err { &mut re as *mut json_error_t } else { ptr::null_mut() };
                let mut cwords = vec![0u64; 16];
                let mut ctag = VaListTag {
                    gp_offset: 48,
                    fp_offset: 176,
                    overflow_arg_area: cwords.as_mut_ptr() as *mut c_void,
                    reg_save_area: ptr::null_mut(),
                };
                let mut rwords = vec![0u64; 16];
                let mut rtag = ctag;
                rtag.overflow_arg_area = rwords.as_mut_ptr() as *mut c_void;
                let cj = (d.c.json_vpack_ex)(cep, flags, ptr::null(), &mut ctag);
                let rj = (d.rs.json_vpack_ex)(rep, flags, ptr::null(), &mut rtag);
                eq("vpack(NULL fmt) null", cj.is_null(), rj.is_null());
                assert!(cj.is_null());
                if use_err {
                    eq_err("vpack(NULL fmt)", &ce, &re);
                    eq("vpack(NULL fmt) code", ce.code(), json_error_invalid_argument);
                }
            }
            // fmt == ""
            cmp_pack(d, "empty-fmt", "", &[], flags);
        }
        // and via the variadic wrappers
        let cj = (d.c.json_pack)(ptr::null::<c_char>());
        let rj = (d.rs.json_pack)(ptr::null::<c_char>());
        eq("json_pack(NULL)", cj.is_null(), rj.is_null());
        let empty = cs("");
        let cj = (d.c.json_pack)(empty.as_ptr());
        let rj = (d.rs.json_pack)(empty.as_ptr());
        eq("json_pack(\"\")", cj.is_null(), rj.is_null());
    }
}

/// Every single byte 0x01..0x7F as the sole format character.
#[test]
fn pack_every_single_byte_format() {
    let d = duo();
    let _g = lock();
    // Every byte 0x01..0x7F as the sole format character. The argument list must
    // match what that specifier actually consumes, otherwise a pointer-shaped
    // slot would be read as an integer and the two libraries would (correctly)
    // report different addresses.
    //
    // `#`, `%` and `+` are modifiers that only appear after `s`; on their own
    // they hit the `default:` branch and consume nothing, so they are included.
    for b in 1u8..0x80 {
        let ch = b as char;
        let fmt = format!("{}", ch);
        let argsets: Vec<Vec<A>> = match ch {
            's' => vec![
                vec![A::S(b"v".to_vec())],
                vec![A::SNull],
                vec![A::S(vec![0x80])],
            ],
            'b' | 'i' => vec![
                vec![A::I(0)],
                vec![A::I(1)],
                vec![A::I(-1)],
                vec![A::I(c_int::MAX)],
                vec![A::I(c_int::MIN)],
            ],
            'I' => vec![
                vec![A::L(0)],
                vec![A::L(7)],
                vec![A::L(i64::MAX)],
                vec![A::L(i64::MIN)],
            ],
            'f' => vec![
                vec![A::D(0.0)],
                vec![A::D(1.5)],
                vec![A::D(f64::NAN)],
                vec![A::D(f64::INFINITY)],
            ],
            'o' => vec![vec![A::JNull], vec![A::Jo("1")], vec![A::Jo(r#"{"a":1}"#)]],
            'O' => vec![vec![A::JNull], vec![A::JO("1")], vec![A::JO(r#"[1]"#)]],
            // everything else either consumes nothing ('n', '{', '[', '}', ']')
            // or lands in `pack`'s `default:` branch, which consumes nothing
            _ => vec![vec![]],
        };
        for args in &argsets {
            cmp_pack(d, "single-byte", &fmt, args, 0);
        }
    }
}

/// Randomized format strings. The alphabet excludes `#`, `%` and `+` (which
/// consume a length from the next slot) and every slot is a pointer to an
/// immortal `json_t{JSON_NULL, (size_t)-1}` that is ALSO the 1-byte C string
/// "\x07", so it is simultaneously safe as `const char*`, `json_t*`, `int`,
/// `long long` and `double`.
#[test]
fn pack_randomized_formats() {
    big_stack(pack_randomized_formats_impl)
}
fn pack_randomized_formats_impl() {
    let d = duo();
    let mut rng = Rng::new(0x9ACF_0271);
    // A json_t that is also a NUL-terminated 1-byte string.
    let mut dual = json_t {
        type_: JSON_NULL,
        refcount: usize::MAX,
    };
    let dualp = &mut dual as *mut json_t as usize as u64;
    debug_assert_eq!(JSON_NULL, 7);
    let alphabet: Vec<char> = "{}[]siIbfnoO?* ,:\t\nFxX@01".chars().collect();
    unsafe {
        for round in 0..6000 {
            let n = 1 + rng.below(10);
            let fmt: String = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
            let f = cs(&fmt);
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            let mut cwords = vec![dualp; 64];
            let mut rwords = vec![dualp; 64];
            let mut ctag = VaListTag {
                gp_offset: 48,
                fp_offset: 176,
                overflow_arg_area: cwords.as_mut_ptr() as *mut c_void,
                reg_save_area: ptr::null_mut(),
            };
            let mut rtag = VaListTag {
                gp_offset: 48,
                fp_offset: 176,
                overflow_arg_area: rwords.as_mut_ptr() as *mut c_void,
                reg_save_area: ptr::null_mut(),
            };
            let cj = (d.c.json_vpack_ex)(&mut ce, 0, f.as_ptr(), &mut ctag);
            let rj = (d.rs.json_vpack_ex)(&mut re, 0, f.as_ptr(), &mut rtag);
            let what = format!("rand-pack#{} fmt={:?}", round, fmt);
            eq(&format!("{} null", what), cj.is_null(), rj.is_null());
            eq_err(&what, &ce, &re);
            if !cj.is_null() {
                eq(&format!("{} tree", what), describe(&d.c, cj), describe(&d.rs, rj));
                let (cd, rd) = dumps_both(d, cj, rj, JSON_ENCODE_ANY | JSON_SORT_KEYS);
                eq(&format!("{} dump null", what), cd.is_none(), rd.is_none());
                if let (Some(a), Some(b)) = (&cd, &rd) {
                    eq_bytes(&format!("{} dump", what), a, b);
                }
            }
            decref(&d.c, cj);
            decref(&d.rs, rj);
        }
    }
}

// ===========================================================================
// CONFIGS 120-127 — unpack
// ===========================================================================

#[test]
fn unpack_every_format_char() {
    let d = duo();
    let _g = lock();
    // (tag, root, fmt, args, str_cells, json_cells)
    let cases: Vec<(&str, &str, &str, Vec<U>, Vec<usize>, Vec<usize>)> = vec![
        ("obj-empty", "{}", "{}", vec![], vec![], vec![]),
        ("arr-empty", "[]", "[]", vec![], vec![], vec![]),
        ("s", r#""hi""#, "s", vec![U::Out], vec![0], vec![]),
        ("s-empty", r#""""#, "s", vec![U::Out], vec![0], vec![]),
        ("s-utf8", r#""héllo""#, "s", vec![U::Out], vec![0], vec![]),
        (
            "s%",
            r#""hi""#,
            "s%",
            vec![U::Out, U::Out],
            vec![0],
            vec![],
        ),
        (
            "s%-nul",
            r#""a\u0000b""#,
            "s%",
            vec![U::Out, U::Out],
            vec![0],
            vec![],
        ),
        ("i", "42", "i", vec![U::Out], vec![], vec![]),
        ("i-neg", "-42", "i", vec![U::Out], vec![], vec![]),
        ("i-trunc", "9223372036854775807", "i", vec![U::Out], vec![], vec![]),
        ("I", "42", "I", vec![U::Out], vec![], vec![]),
        ("I-max", "9223372036854775807", "I", vec![U::Out], vec![], vec![]),
        ("I-min", "-9223372036854775808", "I", vec![U::Out], vec![], vec![]),
        ("b-true", "true", "b", vec![U::Out], vec![], vec![]),
        ("b-false", "false", "b", vec![U::Out], vec![], vec![]),
        ("f", "1.5", "f", vec![U::Out], vec![], vec![]),
        ("f-neg", "-1.5", "f", vec![U::Out], vec![], vec![]),
        ("F-real", "1.5", "F", vec![U::Out], vec![], vec![]),
        ("F-int", "42", "F", vec![U::Out], vec![], vec![]),
        ("n", "null", "n", vec![], vec![], vec![]),
        ("o", "42", "o", vec![U::Out], vec![], vec![0]),
        ("O", "42", "O", vec![U::Out], vec![], vec![0]),
        ("o-obj", r#"{"a":1}"#, "o", vec![U::Out], vec![], vec![0]),
        ("O-obj", r#"{"a":1}"#, "O", vec![U::Out], vec![], vec![0]),
        (
            "obj-1",
            r#"{"a":1}"#,
            "{s:i}",
            vec![U::Key("a"), U::Out],
            vec![],
            vec![],
        ),
        (
            "obj-many",
            r#"{"a":1,"b":"x","c":1.5,"d":true,"e":null,"f":[1],"g":{"h":2}}"#,
            "{s:i,s:s,s:f,s:b,s:n,s:[i],s:{s:i}}",
            vec![
                U::Key("a"), U::Out,
                U::Key("b"), U::Out,
                U::Key("c"), U::Out,
                U::Key("d"), U::Out,
                U::Key("e"),
                U::Key("f"), U::Out,
                U::Key("g"), U::Key("h"), U::Out,
            ],
            vec![1],
            vec![],
        ),
        (
            "arr-many",
            r#"[1,"x",1.5,true,null,[2],{"a":3}]"#,
            "[i,s,f,b,n,[i],{s:i}]",
            vec![U::Out, U::Out, U::Out, U::Out, U::Out, U::Key("a"), U::Out],
            vec![1],
            vec![],
        ),
        (
            "nested-3",
            r#"{"a":{"b":[1,2]}}"#,
            "{s:{s:[i,i]}}",
            vec![U::Key("a"), U::Key("b"), U::Out, U::Out],
            vec![],
            vec![],
        ),
        // separators and whitespace
        (
            "sep",
            r#"{"a":1}"#,
            "  {  s  :  i  }  ",
            vec![U::Key("a"), U::Out],
            vec![],
            vec![],
        ),
        (
            "sep-commas",
            r#"{"a":1}"#,
            "{,,s,,:,,i,,}",
            vec![U::Key("a"), U::Out],
            vec![],
            vec![],
        ),
        // the same key unpacked twice
        (
            "obj-key-twice",
            r#"{"a":1}"#,
            "{s:i,s:i}",
            vec![U::Key("a"), U::Out, U::Key("a"), U::Out],
            vec![],
            vec![],
        ),
    ];
    for (tag, root, fmt, args, sc, jc) in &cases {
        for flags in [
            0usize,
            JSON_VALIDATE_ONLY,
            JSON_STRICT,
            JSON_VALIDATE_ONLY | JSON_STRICT,
            1 << 20,
            usize::MAX,
        ] {
            cmp_unpack(d, tag, root, fmt, args, flags, sc, jc);
        }
    }
}

/// CONFIGS 122-125: `JSON_STRICT` and the in-format `!` / `*` markers.
#[test]
fn unpack_strictness() {
    let d = duo();
    let _g = lock();
    let cases: Vec<(&str, &str, &str, Vec<U>)> = vec![
        ("obj-exact", r#"{"a":1}"#, "{s:i}", vec![U::Key("a"), U::Out]),
        ("obj-extra", r#"{"a":1,"b":2}"#, "{s:i}", vec![U::Key("a"), U::Out]),
        (
            "obj-extra-2",
            r#"{"a":1,"b":2,"c":3}"#,
            "{s:i}",
            vec![U::Key("a"), U::Out],
        ),
        ("obj-bang", r#"{"a":1}"#, "{s:i!}", vec![U::Key("a"), U::Out]),
        (
            "obj-bang-extra",
            r#"{"a":1,"b":2}"#,
            "{s:i!}",
            vec![U::Key("a"), U::Out],
        ),
        ("obj-star", r#"{"a":1,"b":2}"#, "{s:i*}", vec![U::Key("a"), U::Out]),
        ("obj-bang-only", r#"{}"#, "{!}", vec![]),
        ("obj-bang-only-nonempty", r#"{"a":1}"#, "{!}", vec![]),
        ("obj-star-only", r#"{"a":1}"#, "{*}", vec![]),
        ("arr-exact", "[1]", "[i]", vec![U::Out]),
        ("arr-extra", "[1,2]", "[i]", vec![U::Out]),
        ("arr-extra-2", "[1,2,3]", "[i]", vec![U::Out]),
        ("arr-bang", "[1]", "[i!]", vec![U::Out]),
        ("arr-bang-extra", "[1,2]", "[i!]", vec![U::Out]),
        ("arr-star", "[1,2]", "[i*]", vec![U::Out]),
        ("arr-bang-only", "[]", "[!]", vec![]),
        ("arr-bang-only-nonempty", "[1]", "[!]", vec![]),
        ("arr-star-only", "[1]", "[*]", vec![]),
        // optional keys interact with strictness (`gotopt`)
        (
            "obj-opt-present",
            r#"{"a":1}"#,
            "{s?:i}",
            vec![U::Key("a"), U::Out],
        ),
        (
            "obj-opt-absent",
            r#"{"a":1}"#,
            "{s?:i}",
            vec![U::Key("zz"), U::Out],
        ),
        (
            "obj-opt-absent-strict",
            r#"{"a":1}"#,
            "{s?:i!}",
            vec![U::Key("zz"), U::Out],
        ),
        (
            "obj-opt-and-req",
            r#"{"a":1,"b":2}"#,
            "{s:i,s?:i}",
            vec![U::Key("a"), U::Out, U::Key("b"), U::Out],
        ),
        (
            "obj-opt-and-req-strict",
            r#"{"a":1,"b":2}"#,
            "{s:i,s?:i!}",
            vec![U::Key("a"), U::Out, U::Key("b"), U::Out],
        ),
        (
            "obj-opt-absent-container",
            r#"{"a":1}"#,
            "{s?:[i,i]}",
            vec![U::Key("zz"), U::Out, U::Out],
        ),
        (
            "obj-opt-absent-obj",
            r#"{"a":1}"#,
            "{s?:{s:i}}",
            vec![U::Key("zz"), U::Key("q"), U::Out],
        ),
        // nested strictness
        (
            "nested-strict",
            r#"{"a":{"b":1,"c":2}}"#,
            "{s:{s:i!}}",
            vec![U::Key("a"), U::Key("b"), U::Out],
        ),
        (
            "nested-strict-arr",
            r#"{"a":[1,2]}"#,
            "{s:[i!]}",
            vec![U::Key("a"), U::Out],
        ),
        // several markers / markers out of place (ERRORS 174, 181)
        ("obj-bang-then-key", r#"{"a":1}"#, "{!s:i}", vec![U::Key("a"), U::Out]),
        ("obj-star-then-key", r#"{"a":1}"#, "{*s:i}", vec![U::Key("a"), U::Out]),
        ("arr-bang-then-i", "[1]", "[!i]", vec![U::Out]),
        ("arr-star-then-i", "[1]", "[*i]", vec![U::Out]),
        ("obj-bang-bang", r#"{"a":1}"#, "{!!}", vec![]),
        ("obj-star-star", r#"{"a":1}"#, "{**}", vec![]),
        ("arr-bang-bang", "[1]", "[!!]", vec![]),
        ("obj-bang-star", r#"{"a":1}"#, "{!*}", vec![]),
        // many extra keys, so the unrecognized-key strbuffer grows
        (
            "obj-many-extra",
            r#"{"a":1,"bbbb":2,"cccccccc":3,"dddddddddddd":4,"e":5,"f":6,"g":7}"#,
            "{s:i!}",
            vec![U::Key("a"), U::Out],
        ),
    ];
    for (tag, root, fmt, args) in &cases {
        for flags in [
            0usize,
            JSON_STRICT,
            JSON_VALIDATE_ONLY,
            JSON_VALIDATE_ONLY | JSON_STRICT,
        ] {
            cmp_unpack(d, tag, root, fmt, args, flags, &[], &[]);
        }
    }
}

// ===========================================================================
// ERRORS 173-199 — unpack rejections
// ===========================================================================

#[test]
fn unpack_error_paths() {
    let d = duo();
    let _g = lock();
    let roots = [
        ("obj", r#"{"a":1}"#),
        ("arr", "[1]"),
        ("str", r#""s""#),
        ("int", "42"),
        ("real", "1.5"),
        ("true", "true"),
        ("false", "false"),
        ("null", "null"),
        ("obj-empty", "{}"),
        ("arr-empty", "[]"),
    ];
    // 173, 180, 186-194: wrong root type for every specifier
    for (rt, root) in roots {
        for (fmt, args, sc, jc) in [
            ("{s:i}", vec![U::Key("a"), U::Out], vec![], vec![]),
            ("{}", vec![], vec![], vec![]),
            ("[i]", vec![U::Out], vec![], vec![]),
            ("[]", vec![], vec![], vec![]),
            ("s", vec![U::Out], vec![0usize], vec![]),
            ("s%", vec![U::Out, U::Out], vec![0usize], vec![]),
            ("i", vec![U::Out], vec![], vec![]),
            ("I", vec![U::Out], vec![], vec![]),
            ("b", vec![U::Out], vec![], vec![]),
            ("f", vec![U::Out], vec![], vec![]),
            ("F", vec![U::Out], vec![], vec![]),
            ("n", vec![], vec![], vec![]),
            ("o", vec![U::Out], vec![], vec![0usize]),
            ("O", vec![U::Out], vec![], vec![0usize]),
        ] {
            for flags in [0usize, JSON_VALIDATE_ONLY, JSON_STRICT] {
                cmp_unpack(d, rt, root, fmt, &args, flags, &sc, &jc);
            }
        }
    }

    let cases: Vec<(&str, &str, &str, Vec<U>)> = vec![
        // 174: token after '!' / '*' is not '}'
        ("174-a", r#"{"a":1}"#, "{!s}", vec![U::Key("a")]),
        ("174-b", r#"{"a":1}"#, "{*i}", vec![U::Out]),
        ("174-c", r#"{"a":1}"#, "{!@}", vec![]),
        // 175: format ends before '}'
        ("175-a", r#"{"a":1}"#, "{", vec![]),
        ("175-b", r#"{"a":1}"#, "{s", vec![U::Key("a")]),
        ("175-c", r#"{"a":1}"#, "{s:", vec![U::Key("a")]),
        ("175-d", r#"{"a":1}"#, "{s:i", vec![U::Key("a"), U::Out]),
        // 176: key spec is not 's'
        ("176-a", r#"{"a":1}"#, "{i:i}", vec![U::Out, U::Out]),
        ("176-b", r#"{"a":1}"#, "{n:i}", vec![U::Out]),
        ("176-c", r#"{"a":1}"#, "{[]:i}", vec![U::Out]),
        ("176-d", r#"{"a":1}"#, "{O:i}", vec![U::Out, U::Out]),
        // 177: NULL key
        ("177-a", r#"{"a":1}"#, "{s:i}", vec![U::KeyNull, U::Out]),
        ("177-b", r#"{"a":1}"#, "{s?:i}", vec![U::KeyNull, U::Out]),
        // 178: required key not found
        ("178-a", r#"{"a":1}"#, "{s:i}", vec![U::Key("zz"), U::Out]),
        ("178-b", r#"{}"#, "{s:i}", vec![U::Key("a"), U::Out]),
        (
            "178-c",
            r#"{"a":{"b":1}}"#,
            "{s:{s:i}}",
            vec![U::Key("a"), U::Key("zz"), U::Out],
        ),
        // 182: format ends before ']'
        ("182-a", "[1]", "[", vec![]),
        ("182-b", "[1]", "[i", vec![U::Out]),
        ("182-c", "[1]", "[i,", vec![U::Out]),
        // 183: token not in "{[siIbfFOon"
        ("183-a", "[1]", "[@]", vec![]),
        ("183-b", "[1]", "[x]", vec![]),
        ("183-c", "[1]", "[#]", vec![]),
        ("183-d", "[1]", "[%]", vec![]),
        ("183-e", "[1]", "[?]", vec![]),
        ("183-f", "[1]", "[}]", vec![]),
        ("183-g", "[1,2]", "[i,@]", vec![U::Out]),
        // 184: more format items than array elements
        ("184-a", "[1]", "[i,i]", vec![U::Out, U::Out]),
        ("184-b", "[]", "[i]", vec![U::Out]),
        ("184-c", "[1,2]", "[i,i,i]", vec![U::Out, U::Out, U::Out]),
        (
            "184-d",
            r#"{"a":[1]}"#,
            "{s:[i,i]}",
            vec![U::Key("a"), U::Out, U::Out],
        ),
        // 187: NULL `const char **` target
        ("187-a", r#""s""#, "s", vec![U::OutNull]),
        ("187-b", r#"["s"]"#, "[s]", vec![U::OutNull]),
        (
            "187-c",
            r#"{"a":"s"}"#,
            "{s:s}",
            vec![U::Key("a"), U::OutNull],
        ),
        // 188: NULL `size_t *` length target
        ("188-a", r#""s""#, "s%", vec![U::Out, U::OutNull]),
        ("188-b", r#""s""#, "s%", vec![U::OutNull, U::OutNull]),
        // 195: unknown format character at top level
        ("195-at", r#"{"a":1}"#, "@", vec![]),
        ("195-hash", r#"{"a":1}"#, "#", vec![]),
        ("195-percent", r#"{"a":1}"#, "%", vec![]),
        ("195-bang", r#"{"a":1}"#, "!", vec![]),
        ("195-star", r#"{"a":1}"#, "*", vec![]),
        ("195-question", r#"{"a":1}"#, "?", vec![]),
        ("195-plus", r#"{"a":1}"#, "+", vec![]),
        ("195-close", r#"{"a":1}"#, "}", vec![]),
        ("195-close2", r#"{"a":1}"#, "]", vec![]),
        ("195-x", r#"{"a":1}"#, "x", vec![]),
        ("195-digit", r#"{"a":1}"#, "5", vec![]),
        // 199: garbage after the format string
        ("199-a", r#"{"a":1}"#, "{s:i}x", vec![U::Key("a"), U::Out]),
        ("199-b", "[1]", "[i]]", vec![U::Out]),
        ("199-c", "42", "ii", vec![U::Out, U::Out]),
        ("199-d", r#"{"a":1}"#, "{}{}", vec![]),
        ("199-e", "42", "i i", vec![U::Out, U::Out]),
        // separators only
        ("sep-space", r#"{"a":1}"#, " ", vec![]),
        ("sep-comma", r#"{"a":1}"#, ",", vec![]),
        ("sep-mixed", r#"{"a":1}"#, " , : \t\n", vec![]),
    ];
    for (tag, root, fmt, args) in &cases {
        for flags in [
            0usize,
            JSON_VALIDATE_ONLY,
            JSON_STRICT,
            JSON_VALIDATE_ONLY | JSON_STRICT,
        ] {
            cmp_unpack(d, tag, root, fmt, args, flags, &[], &[]);
        }
    }
}

/// ERRORS 179, 185: strict leftovers, with the exact message text.
#[test]
fn unpack_strict_leftovers_messages() {
    let d = duo();
    let _g = lock();
    let cases: Vec<(&str, &str, &str, Vec<U>)> = vec![
        ("obj-1-left", r#"{"a":1,"b":2}"#, "{s:i!}", vec![U::Key("a"), U::Out]),
        (
            "obj-2-left",
            r#"{"a":1,"b":2,"c":3}"#,
            "{s:i!}",
            vec![U::Key("a"), U::Out],
        ),
        (
            "obj-long-keys",
            r#"{"a":1,"bbbbbbbbbb":2,"cccccccccccccccccccc":3}"#,
            "{s:i!}",
            vec![U::Key("a"), U::Out],
        ),
        (
            "obj-utf8-keys",
            r#"{"a":1,"kéy":2,"€":3}"#,
            "{s:i!}",
            vec![U::Key("a"), U::Out],
        ),
        ("obj-all-left", r#"{"a":1,"b":2}"#, "{!}", vec![]),
        ("arr-1-left", "[1,2]", "[i!]", vec![U::Out]),
        ("arr-3-left", "[1,2,3,4]", "[i!]", vec![U::Out]),
        ("arr-all-left", "[1,2,3]", "[!]", vec![]),
        (
            "nested-obj-left",
            r#"{"a":{"b":1,"c":2}}"#,
            "{s:{s:i!}!}",
            vec![U::Key("a"), U::Key("b"), U::Out],
        ),
    ];
    for (tag, root, fmt, args) in &cases {
        for flags in [
            0usize,
            JSON_STRICT,
            JSON_VALIDATE_ONLY,
            JSON_VALIDATE_ONLY | JSON_STRICT,
        ] {
            cmp_unpack(d, tag, root, fmt, args, flags, &[], &[]);
        }
    }
    // JSON_STRICT applied without any in-format marker
    for (tag, root, fmt, args) in [
        ("strict-obj", r#"{"a":1,"b":2}"#, "{s:i}", vec![U::Key("a"), U::Out]),
        ("strict-arr", "[1,2]", "[i]", vec![U::Out]),
        ("strict-obj-exact", r#"{"a":1}"#, "{s:i}", vec![U::Key("a"), U::Out]),
        ("strict-arr-exact", "[1]", "[i]", vec![U::Out]),
    ] {
        cmp_unpack(d, tag, root, fmt, &args, JSON_STRICT, &[], &[]);
    }
}

/// ERRORS 196-198: `root == NULL`, `fmt == NULL`, `fmt == ""`.
#[test]
fn unpack_null_root_and_format() {
    let d = duo();
    let _g = lock();
    unsafe {
        let rz = cs(r#"{"a":1}"#);
        for flags in [0usize, JSON_STRICT, JSON_VALIDATE_ONLY, usize::MAX] {
            // root == NULL
            for use_err in [true, false] {
                let fmt = cs("{s:i}");
                let mut ce = json_error_t::new();
                let mut re = json_error_t::new();
                let cep = if use_err { &mut ce as *mut json_error_t } else { ptr::null_mut() };
                let rep = if use_err { &mut re as *mut json_error_t } else { ptr::null_mut() };
                let mut cw = vec![0u64; 32];
                let mut rw = vec![0u64; 32];
                let mut ct = VaListTag {
                    gp_offset: 48,
                    fp_offset: 176,
                    overflow_arg_area: cw.as_mut_ptr() as *mut c_void,
                    reg_save_area: ptr::null_mut(),
                };
                let mut rt = VaListTag {
                    gp_offset: 48,
                    fp_offset: 176,
                    overflow_arg_area: rw.as_mut_ptr() as *mut c_void,
                    reg_save_area: ptr::null_mut(),
                };
                let crc = (d.c.json_vunpack_ex)(ptr::null_mut(), cep, flags, fmt.as_ptr(), &mut ct);
                let rrc = (d.rs.json_vunpack_ex)(ptr::null_mut(), rep, flags, fmt.as_ptr(), &mut rt);
                eq("vunpack(NULL root)", crc, rrc);
                assert_eq!(crc, -1);
                if use_err {
                    eq_err("vunpack(NULL root)", &ce, &re);
                    eq("vunpack(NULL root) code", ce.code(), json_error_null_value);
                }
            }
            // fmt == NULL / ""
            for f in [ptr::null::<c_char>(), cs("").as_ptr()] {
                let empty_keeper = cs("");
                let fp = if f.is_null() { ptr::null() } else { empty_keeper.as_ptr() };
                let croot = (d.c.json_loads)(rz.as_ptr(), 0, ptr::null_mut());
                let rroot = (d.rs.json_loads)(rz.as_ptr(), 0, ptr::null_mut());
                let mut ce = json_error_t::new();
                let mut re = json_error_t::new();
                let mut cw = vec![0u64; 32];
                let mut rw = vec![0u64; 32];
                let mut ct = VaListTag {
                    gp_offset: 48,
                    fp_offset: 176,
                    overflow_arg_area: cw.as_mut_ptr() as *mut c_void,
                    reg_save_area: ptr::null_mut(),
                };
                let mut rt = VaListTag {
                    gp_offset: 48,
                    fp_offset: 176,
                    overflow_arg_area: rw.as_mut_ptr() as *mut c_void,
                    reg_save_area: ptr::null_mut(),
                };
                let crc = (d.c.json_vunpack_ex)(croot, &mut ce, flags, fp, &mut ct);
                let rrc = (d.rs.json_vunpack_ex)(rroot, &mut re, flags, fp, &mut rt);
                eq("vunpack(bad fmt)", crc, rrc);
                eq_err("vunpack(bad fmt)", &ce, &re);
                eq("vunpack(bad fmt) code", ce.code(), json_error_invalid_argument);
                decref(&d.c, croot);
                decref(&d.rs, rroot);
            }
            // both NULL
            let crc = (d.c.json_vunpack_ex)(
                ptr::null_mut(),
                ptr::null_mut(),
                flags,
                ptr::null(),
                ptr::null_mut(),
            );
            let rrc = (d.rs.json_vunpack_ex)(
                ptr::null_mut(),
                ptr::null_mut(),
                flags,
                ptr::null(),
                ptr::null_mut(),
            );
            eq("vunpack(NULL,NULL)", crc, rrc);
        }
    }
}

/// Randomized unpack format strings. Every slot is a pointer to a distinct
/// zeroed 8-byte cell, so it is safe as `int*`, `json_int_t*`, `double*`,
/// `size_t*`, `const char**`, `json_t**` and (read as `const char*`) an empty
/// string.
#[test]
fn unpack_randomized_formats() {
    big_stack(unpack_randomized_formats_impl)
}
fn unpack_randomized_formats_impl() {
    let d = duo();
    let mut rng = Rng::new(0x11AC_0271);
    let roots = [
        r#"{"a":1}"#,
        r#"{"a":1,"b":"s","c":[1,2],"d":{"e":3}}"#,
        "[1,2,3]",
        r#"["s",1.5,true,null]"#,
        "{}",
        "[]",
        "42",
        r#""s""#,
        "1.5",
        "true",
        "null",
    ];
    let alphabet: Vec<char> = "{}[]siIbfFnoO?!* ,:\t\n%#@x01".chars().collect();
    unsafe {
        for round in 0..6000 {
            let n = 1 + rng.below(10);
            let fmt: String = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
            let root = roots[rng.below(roots.len())];
            let f = cs(&fmt);
            let rz = cs(root);
            let croot = (d.c.json_loads)(rz.as_ptr(), JSON_DECODE_ANY, ptr::null_mut());
            let rroot = (d.rs.json_loads)(rz.as_ptr(), JSON_DECODE_ANY, ptr::null_mut());
            assert!(!croot.is_null() && !rroot.is_null());
            let flags = [0usize, JSON_STRICT, JSON_VALIDATE_ONLY, JSON_STRICT | JSON_VALIDATE_ONLY]
                [rng.below(4)];
            let mut ce = json_error_t::new();
            let mut re = json_error_t::new();
            let mut ccells = Box::new([0u64; 64]);
            let mut rcells = Box::new([0u64; 64]);
            let cw: Vec<u64> = (0..48).map(|i| ccells.as_ptr().wrapping_add(i) as usize as u64).collect();
            let rw: Vec<u64> = (0..48).map(|i| rcells.as_ptr().wrapping_add(i) as usize as u64).collect();
            let mut cw = cw;
            let mut rw = rw;
            let mut ct = VaListTag {
                gp_offset: 48,
                fp_offset: 176,
                overflow_arg_area: cw.as_mut_ptr() as *mut c_void,
                reg_save_area: ptr::null_mut(),
            };
            let mut rt = VaListTag {
                gp_offset: 48,
                fp_offset: 176,
                overflow_arg_area: rw.as_mut_ptr() as *mut c_void,
                reg_save_area: ptr::null_mut(),
            };
            let crc = (d.c.json_vunpack_ex)(croot, &mut ce, flags, f.as_ptr(), &mut ct);
            let rrc = (d.rs.json_vunpack_ex)(rroot, &mut re, flags, f.as_ptr(), &mut rt);
            let what = format!(
                "rand-unpack#{} fmt={:?} root={} flags={:#x}",
                round, fmt, root, flags
            );
            eq(&format!("{} ret", what), crc, rrc);
            eq_err(&what, &ce, &re);
            // Cells hold either raw scalars or pointers into the respective
            // root; compare "was it written at all" plus the raw value for the
            // non-pointer cases. A written pointer differs between libraries, so
            // compare only NULL-ness/writtenness there.
            for i in 0..48 {
                let cv = ccells[i];
                let rv = rcells[i];
                let cwritten = cv != 0;
                let rwritten = rv != 0;
                eq(&format!("{} cell[{}] written", what, i), cwritten, rwritten);
                // if the value is not a pointer into either root it must match exactly
                let looks_ptr = cv > 0x1000 && rv > 0x1000;
                if !looks_ptr {
                    eq(&format!("{} cell[{}]", what, i), cv, rv);
                }
            }
            eq(
                &format!("{} root after", what),
                describe(&d.c, croot),
                describe(&d.rs, rroot),
            );
            decref(&d.c, croot);
            decref(&d.rs, rroot);
        }
    }
}

// ===========================================================================
// CONFIGS 113, 128 — the variadic wrappers (naked-asm exports in src/va.rs)
// ===========================================================================

#[test]
fn variadic_wrappers() {
    let d = duo();
    let _g = lock();
    unsafe {
        // ---- json_pack ---------------------------------------------------
        let fmt = cs("{s:i,s:s,s:f,s:b,s:I,s:n,s:[i,i],s:{s:i}}");
        let ka = cs("a");
        let kb = cs("b");
        let kc = cs("c");
        let kd = cs("d");
        let ke = cs("e");
        let kf = cs("f");
        let kg = cs("g");
        let kh = cs("h");
        let ki = cs("i");
        let sv = cs("text");
        let cj = (d.c.json_pack)(
            fmt.as_ptr(), ka.as_ptr(), 1i32, kb.as_ptr(), sv.as_ptr(), kc.as_ptr(), 2.5f64,
            kd.as_ptr(), 1i32, ke.as_ptr(), 42i64, kf.as_ptr(), kg.as_ptr(), 3i32, 4i32,
            kh.as_ptr(), ki.as_ptr(), 5i32,
        );
        let rj = (d.rs.json_pack)(
            fmt.as_ptr(), ka.as_ptr(), 1i32, kb.as_ptr(), sv.as_ptr(), kc.as_ptr(), 2.5f64,
            kd.as_ptr(), 1i32, ke.as_ptr(), 42i64, kf.as_ptr(), kg.as_ptr(), 3i32, 4i32,
            kh.as_ptr(), ki.as_ptr(), 5i32,
        );
        eq("json_pack null", cj.is_null(), rj.is_null());
        assert!(!cj.is_null(), "C json_pack failed");
        eq("json_pack tree", describe(&d.c, cj), describe(&d.rs, rj));
        let (cd, rd) = dumps_both(d, cj, rj, JSON_SORT_KEYS);
        eq_bytes("json_pack dump", cd.as_deref().unwrap(), rd.as_deref().unwrap());

        // ---- json_pack_ex ------------------------------------------------
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let f2 = cs("{s:i,s:f}");
        let cj2 = (d.c.json_pack_ex)(&mut ce, 0, f2.as_ptr(), ka.as_ptr(), 9i32, kb.as_ptr(), 1.25f64);
        let rj2 = (d.rs.json_pack_ex)(&mut re, 0, f2.as_ptr(), ka.as_ptr(), 9i32, kb.as_ptr(), 1.25f64);
        eq("json_pack_ex null", cj2.is_null(), rj2.is_null());
        eq_err("json_pack_ex", &ce, &re);
        eq("json_pack_ex tree", describe(&d.c, cj2), describe(&d.rs, rj2));

        // json_pack_ex on an error path
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let f3 = cs("{s:@}");
        let cj3 = (d.c.json_pack_ex)(&mut ce, 0, f3.as_ptr(), ka.as_ptr());
        let rj3 = (d.rs.json_pack_ex)(&mut re, 0, f3.as_ptr(), ka.as_ptr());
        eq("json_pack_ex err null", cj3.is_null(), rj3.is_null());
        eq_err("json_pack_ex err", &ce, &re);

        // ---- json_unpack -------------------------------------------------
        let uf = cs("{s:i,s:s,s:f,s:b,s:I,s:[i,i]}");
        let mut ci: c_int = 0;
        let mut cstr: *const c_char = ptr::null();
        let mut cf: f64 = 0.0;
        let mut cbv: c_int = 0;
        let mut cl: i64 = 0;
        let mut ca1: c_int = 0;
        let mut ca2: c_int = 0;
        let crc = (d.c.json_unpack)(
            cj, uf.as_ptr(), ka.as_ptr(), &mut ci, kb.as_ptr(), &mut cstr, kc.as_ptr(),
            &mut cf, kd.as_ptr(), &mut cbv, ke.as_ptr(), &mut cl, kg.as_ptr(), &mut ca1,
            &mut ca2,
        );
        let mut ri: c_int = 0;
        let mut rstr: *const c_char = ptr::null();
        let mut rf: f64 = 0.0;
        let mut rbv: c_int = 0;
        let mut rl: i64 = 0;
        let mut ra1: c_int = 0;
        let mut ra2: c_int = 0;
        let rrc = (d.rs.json_unpack)(
            rj, uf.as_ptr(), ka.as_ptr(), &mut ri, kb.as_ptr(), &mut rstr, kc.as_ptr(),
            &mut rf, kd.as_ptr(), &mut rbv, ke.as_ptr(), &mut rl, kg.as_ptr(), &mut ra1,
            &mut ra2,
        );
        eq("json_unpack ret", crc, rrc);
        eq("json_unpack i", ci, ri);
        eq_bytes("json_unpack s", &cstr_bytes(cstr), &cstr_bytes(rstr));
        eq("json_unpack f", cf.to_bits(), rf.to_bits());
        eq("json_unpack b", cbv, rbv);
        eq("json_unpack I", cl, rl);
        eq("json_unpack arr", (ca1, ca2), (ra1, ra2));

        // ---- json_unpack_ex ----------------------------------------------
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let uf2 = cs("{s:i!}");
        let mut ci: c_int = 0;
        let mut ri: c_int = 0;
        let crc = (d.c.json_unpack_ex)(cj, &mut ce, JSON_STRICT, uf2.as_ptr(), ka.as_ptr(), &mut ci);
        let rrc = (d.rs.json_unpack_ex)(rj, &mut re, JSON_STRICT, uf2.as_ptr(), ka.as_ptr(), &mut ri);
        eq("json_unpack_ex ret", crc, rrc);
        eq_err("json_unpack_ex", &ce, &re);
        eq("json_unpack_ex i", ci, ri);

        // ---- json_unpack_ex with JSON_VALIDATE_ONLY (no varargs consumed) --
        let mut ce = json_error_t::new();
        let mut re = json_error_t::new();
        let uf3 = cs("{s:i,s:s,s:f}");
        let crc = (d.c.json_unpack_ex)(
            cj, &mut ce, JSON_VALIDATE_ONLY, uf3.as_ptr(), ka.as_ptr(), kb.as_ptr(),
            kc.as_ptr(),
        );
        let rrc = (d.rs.json_unpack_ex)(
            rj, &mut re, JSON_VALIDATE_ONLY, uf3.as_ptr(), ka.as_ptr(), kb.as_ptr(),
            kc.as_ptr(),
        );
        eq("json_unpack_ex validate-only ret", crc, rrc);
        eq_err("json_unpack_ex validate-only", &ce, &re);

        decref(&d.c, cj2);
        decref(&d.rs, rj2);
        decref(&d.c, cj3);
        decref(&d.rs, rj3);
        decref(&d.c, cj);
        decref(&d.rs, rj);
    }
}

/// CONFIGS 126: `o` / `O` refcount semantics observed through the root.
#[test]
fn unpack_o_and_O_refcounts() {
    let d = duo();
    let _g = lock();
    unsafe {
        for (fmt, args) in [
            ("o", vec![U::Out]),
            ("O", vec![U::Out]),
            ("{s:o}", vec![U::Key("a"), U::Out]),
            ("{s:O}", vec![U::Key("a"), U::Out]),
            ("[o]", vec![U::Out]),
            ("[O]", vec![U::Out]),
            ("{s:O,s:O}", vec![U::Key("a"), U::Out, U::Key("a"), U::Out]),
        ] {
            for flags in [0usize, JSON_VALIDATE_ONLY, JSON_STRICT] {
                for root in [r#"{"a":1}"#, r#"{"a":[1,2]}"#, r#"{"a":{"b":1}}"#] {
                    cmp_unpack(d, "refcount", root, fmt, &args, flags, &[], &[0, 1]);
                }
            }
        }
    }
}

/// CONFIGS 128: pack -> unpack round trip over randomized values.
#[test]
fn pack_unpack_roundtrip() {
    let d = duo();
    let _g = lock();
    let mut rng = Rng::new(0x8007_1234);
    unsafe {
        for round in 0..1500 {
            // build a random flat object with random specifiers
            let n = 1 + rng.below(6);
            let mut fmt = String::from("{");
            let mut args: Vec<A> = Vec::new();
            let mut keys: Vec<String> = Vec::new();
            for i in 0..n {
                let k = format!("k{}", i);
                keys.push(k.clone());
                args.push(A::S(k.clone().into_bytes()));
                fmt.push_str("s:");
                match rng.below(6) {
                    0 => {
                        fmt.push('i');
                        args.push(A::I(rng.next_u64() as c_int));
                    }
                    1 => {
                        fmt.push('I');
                        args.push(A::L(rng.next_u64() as i64));
                    }
                    2 => {
                        fmt.push('f');
                        args.push(A::D(rng.tame_f64()));
                    }
                    3 => {
                        fmt.push('s');
                        let ln = rng.below(8);
                        args.push(A::S(rng.utf8_string(ln)));
                    }
                    4 => {
                        fmt.push('b');
                        args.push(A::I(rng.next_u64() as c_int));
                    }
                    _ => fmt.push('n'),
                }
                if i + 1 < n {
                    fmt.push(',');
                }
            }
            fmt.push('}');
            cmp_pack(d, &format!("rt#{}", round), &fmt, &args, 0);

            // now unpack it back with matching specifiers
            let mut cb = bind(&d.c, &args);
            let (cap, _ck) = cb.va();
            let cf = cs(&fmt);
            let cj = (d.c.json_vpack_ex)(ptr::null_mut(), 0, cf.as_ptr(), cap);
            let mut rb = bind(&d.rs, &args);
            let (rap, _rk) = rb.va();
            let rj = (d.rs.json_vpack_ex)(ptr::null_mut(), 0, cf.as_ptr(), rap);
            if !cj.is_null() {
                // read every key back with `O`
                let mut ufmt = String::from("{");
                let mut uargs: Vec<U> = Vec::new();
                for (i, k) in keys.iter().enumerate() {
                    ufmt.push_str("s:O");
                    let sk: &'static str = Box::leak(k.clone().into_boxed_str());
                    uargs.push(U::Key(sk));
                    uargs.push(U::Out);
                    if i + 1 < keys.len() {
                        ufmt.push(',');
                    }
                }
                ufmt.push('}');
                let uf = cs(&ufmt);
                let mut cu = ubind(&uargs);
                let (cuap, _c2) = cu.va();
                let mut ce = json_error_t::new();
                let crc = (d.c.json_vunpack_ex)(cj, &mut ce, 0, uf.as_ptr(), cuap);
                let mut ru = ubind(&uargs);
                let (ruap, _r2) = ru.va();
                let mut re = json_error_t::new();
                let rrc = (d.rs.json_vunpack_ex)(rj, &mut re, 0, uf.as_ptr(), ruap);
                eq(&format!("rt#{} unpack ret", round), crc, rrc);
                eq_err(&format!("rt#{} unpack", round), &ce, &re);
                for i in 0..cu.n_out {
                    eq(
                        &format!("rt#{} unpacked cell[{}]", round, i),
                        cell_view(&d.c, cu.cells[i], false, true),
                        cell_view(&d.rs, ru.cells[i], false, true),
                    );
                }
                // `O` increffed each member; drop those references again
                for i in 0..cu.n_out {
                    if cu.cells[i] != 0 {
                        decref(&d.c, cu.cells[i] as usize as *mut json_t);
                    }
                    if ru.cells[i] != 0 {
                        decref(&d.rs, ru.cells[i] as usize as *mut json_t);
                    }
                }
            }
            decref(&d.c, cj);
            decref(&d.rs, rj);
            cb.release(&d.c);
            rb.release(&d.rs);
        }
    }
}

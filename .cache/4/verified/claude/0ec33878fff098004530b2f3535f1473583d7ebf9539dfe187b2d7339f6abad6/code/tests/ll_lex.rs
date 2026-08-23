//! Phase B/C: differential tests for the exported jslex.c helpers and the two
//! lexer modes, plus js_intern / js_isarrayindex / js_strdup / js_malloc.
//! CONFIGS.md rows 377-403; the jslex.c ERRORS.md rows 259-322.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void, CStr};

#[test]
fn t_char_classifiers() {
    let p = libs();
    unsafe {
        // C enums / int params accept any int, so sweep well past the ASCII range
        let mut vals: Vec<c_int> = (-300..600).collect();
        vals.extend([
            i32::MIN,
            i32::MIN + 1,
            -1,
            0,
            0x7f,
            0x80,
            0xff,
            0x100,
            0x2028,
            0x2029,
            0xa0,
            0xfeff,
            0x10ffff,
            i32::MAX,
            i32::MAX - 1,
        ]);
        for c in vals {
            for name in ["jsY_iswhite", "jsY_isnewline", "jsY_ishex", "jsY_tohex"] {
                assert_eq!(
                    p.c.int_pred(name, c),
                    p.rs.int_pred(name, c),
                    "{name}({c})"
                );
            }
        }
    }
}

#[test]
fn t_tokenstring() {
    let p = libs();
    unsafe {
        // token ids run 0..~157; out-of-range must give the same "<unknown>"
        let mut vals: Vec<c_int> = (-50..600).collect();
        vals.extend([i32::MIN, i32::MAX, -1, 0, 255, 256, 1000, 65536]);
        for t in vals {
            let a = from_c(p.c.jsY_tokenstring(t));
            let b = from_c(p.rs.jsY_tokenstring(t));
            assert_eq!(a, b, "jsY_tokenstring({t})");
        }
    }
}

#[test]
fn t_findword() {
    let p = libs();
    unsafe {
        // jsY_findword does a binary search over a sorted NUL-terminated list
        let words: Vec<&str> = vec![
            "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
        ];
        let cs: Vec<std::ffi::CString> = words.iter().map(|w| cstr(w)).collect();
        let list: Vec<*const c_char> = cs.iter().map(|c| c.as_ptr()).collect();
        let mut probes: Vec<String> = words.iter().map(|s| s.to_string()).collect();
        probes.extend(
            [
                "", "a", "z", "aa", "alph", "alphaa", "india", "Alpha", "ALPHA", "bravo ",
                " bravo", "hotel", "hotels", "hote",
            ]
            .iter()
            .map(|s| s.to_string()),
        );
        let mut rng = Rng::new(0x1e77);
        for _ in 0..3000 {
            probes.push(rng.ascii_string(8));
        }
        for probe in &probes {
            let ps = cstr(probe);
            // vary the list length, including 0 and 1 (empty / one / many)
            for n in 0..=list.len() {
                let a = p.c.jsY_findword(ps.as_ptr(), list.as_ptr(), n as c_int);
                let b = p.rs.jsY_findword(ps.as_ptr(), list.as_ptr(), n as c_int);
                assert_eq!(a, b, "jsY_findword({probe:?}, n={n})");
            }
        }
    }
}

/// Drive both lexer modes end-to-end by parsing sources; the token stream is
/// observable through the resulting behaviour and the syntax-error messages.
fn lex_sources() -> Vec<String> {
    let mut v: Vec<String> = vec![];
    for s in [
        // numeric literal forms
        "0", "1", "007", "0x1f", "0X1F", "0xdeadBEEF", "1e5", "1E5", "1e+5", "1e-5",
        ".5", "5.", "5.5", "0.0", "1.e3", ".5e3", "1_0", "0b11", "0o17", "09", "08.5",
        "0xg", "1e", "1e+", "0x", "1..2", "1.2.3", "3in", "3 in [3]",
        // string literal forms and every escape
        "'a'", "\"a\"", "''", "\"\"", "'\\n'", "'\\r'", "'\\t'", "'\\b'", "'\\f'",
        "'\\v'", "'\\0'", "'\\x41'", "'\\u0041'", "'\\u{41}'", "'\\\\'", "'\\''",
        "'\\\"'", "'\\a'", "'\\1'", "'\\8'", "'\\x'", "'\\xg1'", "'\\u12'", "'\\uzzzz'",
        "'a\\\nb'", "'unterminated", "\"unterminated", "'a\nb'",
        // regexp literal vs division
        "/a/", "/a/g", "/a/gi", "/a/gim", "/a/x", "/a/gg", "1/2", "a/b/c",
        "var re = /a/; 1/2", "/[/]/", "/\\//", "/a", "/*/", "//", "/ /",
        "x = y / z / w", "(1)/2/3",
        // comments
        "// line", "/* block */", "/* multi\nline */", "/* unterminated",
        "1 // trailing", "1 /* c */ + 2", "<!-- html comment", "--> not a comment",
        // operators, all multi-char forms
        "a<=b", "a>=b", "a==b", "a!=b", "a===b", "a!==b", "a<<b", "a>>b", "a>>>b",
        "a+=b", "a-=b", "a*=b", "a/=b", "a%=b", "a<<=b", "a>>=b", "a>>>=b",
        "a&=b", "a|=b", "a^=b", "a&&b", "a||b", "a++", "a--", "++a", "--a",
        "a?b:c", "a,b", "a.b", "a[b]", "~a", "!a", "a%b", "a^b", "a&b", "a|b",
        // keywords / reserved / future reserved
        "var x", "function f(){}", "if(1){}else{}", "for(;;)break", "while(0);",
        "do;while(0)", "switch(1){case 1:break;default:}", "try{}catch(e){}",
        "try{}finally{}", "throw 1", "return", "new Object", "delete x", "void 0",
        "typeof x", "instanceof", "in", "this", "null", "true", "false",
        "with({}){}", "debugger", "class", "const", "enum", "export", "extends",
        "import", "super", "implements", "interface", "let", "package", "private",
        "protected", "public", "static", "yield",
        // line terminators and whitespace
        "1\n2", "1\r2", "1\r\n2", "1\u{2028}2", "1\u{2029}2", "1\u{a0}2",
        "1\u{feff}2", "1\t2", "1\u{b}2", "1\u{c}2",
        // identifiers
        "abc", "_abc", "$abc", "a1", "\u{e9}", "a\u{e9}b", "\\u0041", "\u{4e2d}",
        // ASI cases
        "return\n1", "a=1\nb=2", "a\n++b", "throw\n1",
        // JSON mode (via JSON.parse)
        "JSON.parse('1')", "JSON.parse('\"a\"')", "JSON.parse('[1,2]')",
        "JSON.parse('{\"a\":1}')", "JSON.parse('true')", "JSON.parse('false')",
        "JSON.parse('null')", "JSON.parse('-1.5e3')", "JSON.parse('\"\\\\u0041\"')",
        "JSON.parse('\"\\\\n\"')", "JSON.parse('01')", "JSON.parse('+1')",
        "JSON.parse('.5')", "JSON.parse('1.')", "JSON.parse(\"'a'\")",
        "JSON.parse('{a:1}')", "JSON.parse('[1,]')", "JSON.parse('{\"a\":1,}')",
        "JSON.parse('')", "JSON.parse('nul')", "JSON.parse('tru')",
        "JSON.parse('fals')", "JSON.parse('NaN')", "JSON.parse('Infinity')",
        "JSON.parse('\"\\\\x41\"')", "JSON.parse('\"\\\\uzzzz\"')",
        "JSON.parse('\"\\\\u00\"')", "JSON.parse('[')", "JSON.parse('{')",
        "JSON.parse('}')", "JSON.parse(']')", "JSON.parse('1 2')",
        "JSON.parse('\"unterminated')",
    ] {
        v.push(s.to_string());
        v.push(format!("print({s})"));
    }
    v
}

#[test]
fn t_lexer_modes() {
    for src in lex_sources() {
        diff_dostring(0, &src);
        diff_dostring(JS_STRICT, &src);
        diff_eval(0, &src);
        diff_eval(JS_STRICT, &src);
    }
}

#[test]
fn t_lexer_fuzz() {
    let mut rng = Rng::new(0x1E7F_1234_5678_9ABD);
    let toks = [
        "1", "0x1f", "'s'", "\"s\"", "a", "+", "-", "*", "/", "%", "(", ")", "{", "}",
        "[", "]", ";", ",", ".", "?", ":", "=", "==", "===", "!", "<", ">", "<=", ">=",
        "&&", "||", "++", "--", "~", "&", "|", "^", "<<", ">>", ">>>", "var", "function",
        "if", "else", "for", "while", "return", "new", "typeof", "delete", "void", "in",
        "instanceof", "this", "null", "true", "false", "/re/", "//c\n", "/*c*/", "\n",
        " ", "\t", "\\u0041", "$", "_",
    ];
    for _ in 0..4000 {
        let n = 1 + rng.below(10) as usize;
        let src: String = (0..n)
            .map(|_| toks[rng.below(toks.len() as u32) as usize])
            .collect::<Vec<_>>()
            .join("");
        diff_dostring(0, &src);
        diff_dostring(JS_STRICT, &src);
    }
}

#[test]
fn t_intern_and_arrayindex() {
    let p = libs();
    unsafe {
        let jc = new_state(&p.c, 0);
        set_cur(&p.rs);
        let jr = new_state(&p.rs, 0);
        let mut names: Vec<String> = vec![
            "".into(),
            "0".into(),
            "1".into(),
            "-1".into(),
            "+1".into(),
            "00".into(),
            "01".into(),
            "0.0".into(),
            "1.0".into(),
            " 1".into(),
            "1 ".into(),
            "1e3".into(),
            "0x10".into(),
            "length".into(),
            "abc".into(),
            "4294967294".into(),
            "4294967295".into(),
            "4294967296".into(),
            "2147483647".into(),
            "2147483648".into(),
            "2147483649".into(),
            "9999999999".into(),
            "99999999999999999999".into(),
            "1e21".into(),
            "-0".into(),
            "NaN".into(),
            "Infinity".into(),
        ];
        let mut rng = Rng::new(0xAB1E);
        for _ in 0..4000 {
            names.push(format!("{}", rng.next_u32()));
        }
        for _ in 0..2000 {
            names.push(rng.ascii_string(12));
        }
        for _ in 0..500 {
            names.push("9".repeat(1 + rng.below(24) as usize));
        }
        for name in &names {
            let cs = cstr(name);
            // js_isarrayindex
            let mut ia: c_int = -12345;
            let mut ib: c_int = -12345;
            set_cur(&p.c);
            let ra = p.c.js_isarrayindex(jc, cs.as_ptr(), &mut ia);
            set_cur(&p.rs);
            let rb = p.rs.js_isarrayindex(jr, cs.as_ptr(), &mut ib);
            assert_eq!((ra, ia), (rb, ib), "js_isarrayindex({name:?})");
            // js_intern must return equal *contents* and be idempotent
            set_cur(&p.c);
            let sa = p.c.js_intern(jc, cs.as_ptr());
            let sa2 = p.c.js_intern(jc, cs.as_ptr());
            set_cur(&p.rs);
            let sb = p.rs.js_intern(jr, cs.as_ptr());
            let sb2 = p.rs.js_intern(jr, cs.as_ptr());
            assert_eq!(from_c(sa), from_c(sb), "js_intern({name:?})");
            assert_eq!(sa, sa2, "C js_intern not idempotent for {name:?}");
            assert_eq!(sb, sb2, "RUST js_intern not idempotent for {name:?}");
        }
        set_cur(&p.c);
        p.c.js_freestate(jc);
        set_cur(&p.rs);
        p.rs.js_freestate(jr);
    }
}

#[test]
fn t_strdup_malloc_realloc_free() {
    let p = libs();
    unsafe {
        for l in [&p.c, &p.rs] {
            set_cur(l);
            let j = new_state(l, 0);
            // js_strdup
            for s in ["", "a", "hello", &"x".repeat(1000)] {
                let cs = cstr(s);
                let d = l.js_strdup(j, cs.as_ptr());
                assert!(!d.is_null());
                assert_eq!(
                    CStr::from_ptr(d).to_bytes(),
                    cs.as_bytes(),
                    "{}: js_strdup({s:?})",
                    l.name
                );
                l.js_free(j, d as *mut c_void);
            }
            // js_malloc / js_realloc / js_free
            for n in [1, 8, 16, 4096, 65536] {
                let m = l.js_malloc(j, n);
                assert!(!m.is_null(), "{}: js_malloc({n})", l.name);
                let m2 = l.js_realloc(j, m, n * 2);
                assert!(!m2.is_null(), "{}: js_realloc({n})", l.name);
                l.js_free(j, m2);
            }
            // js_free(NULL) must be a no-op
            l.js_free(j, std::ptr::null_mut());
            // js_realloc(NULL, n) behaves like malloc
            let m = l.js_realloc(j, std::ptr::null_mut(), 32);
            assert!(!m.is_null());
            l.js_free(j, m);
            l.js_freestate(j);
        }
    }
}

/// js_freestate(NULL) is an explicit no-op (ERRORS row 258).
#[test]
fn t_freestate_null() {
    let p = libs();
    unsafe {
        p.c.js_freestate(std::ptr::null_mut());
        p.rs.js_freestate(std::ptr::null_mut());
    }
}

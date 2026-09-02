//! Phase B rows 24-26: lexer character-class helpers and token/word tables.
mod common;
use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

#[test]
fn row24_char_class_helpers() {
    let p = pair();
    let mut cases: Vec<c_int> = (-0x400..0x3000).collect();
    for c in [
        0xfeff, 0x2028, 0x2029, 0x00a0, 0x1680, 0x180e, 0x2000, 0x200a, 0x202f, 0x205f, 0x3000,
        0x10000, i32::MAX, i32::MIN, -1, 0,
    ] {
        cases.push(c);
    }
    let mut rng = Rng::new(0x2424);
    for _ in 0..20000 {
        cases.push(rng.i32());
    }
    for c in cases {
        let a = unsafe {
            (
                (p.c.jsY_iswhite)(c),
                (p.c.jsY_isnewline)(c),
                (p.c.jsY_ishex)(c),
                (p.c.jsY_tohex)(c),
            )
        };
        let b = unsafe {
            (
                (p.r.jsY_iswhite)(c),
                (p.r.jsY_isnewline)(c),
                (p.r.jsY_ishex)(c),
                (p.r.jsY_tohex)(c),
            )
        };
        assert_eq!(a, b, "lex char-class helpers for {c:#x}");
    }
}

#[test]
fn row25_tokenstring() {
    let p = pair();
    let mut cases: Vec<c_int> = (-16..400).collect();
    for c in [i32::MAX, i32::MIN, 1000, 100000] {
        cases.push(c);
    }
    for t in cases {
        let a = unsafe { rstr((p.c.jsY_tokenstring)(t)) };
        let b = unsafe { rstr((p.r.jsY_tokenstring)(t)) };
        assert_eq!(a, b, "jsY_tokenstring({t})");
    }
}

#[test]
fn row26_findword() {
    let p = pair();
    // Sorted word lists of several lengths, mirroring how jslex.c uses the
    // function (binary search over a sorted keyword table).
    let lists: Vec<Vec<&str>> = vec![
        vec![],
        vec!["a"],
        vec!["a", "b"],
        vec!["a", "b", "c"],
        vec![
            "break", "case", "catch", "continue", "debugger", "default", "delete", "do", "else",
            "false", "finally", "for", "function", "if", "in", "instanceof", "new", "null",
            "return", "switch", "this", "throw", "true", "try", "typeof", "var", "void", "while",
            "with",
        ],
        vec![
            "abstract", "boolean", "byte", "char", "class", "const", "double", "enum", "export",
            "extends", "final", "float", "goto", "implements", "import", "int", "interface",
            "long", "native", "package", "private", "protected", "public", "short", "static",
            "super", "synchronized", "throws", "transient", "volatile",
        ],
    ];
    let mut probes: Vec<String> = Vec::new();
    for l in &lists {
        for w in l {
            probes.push((*w).to_string());
            probes.push(format!("{w}x"));
            probes.push(w[..w.len() - 1].to_string());
            probes.push(w.to_uppercase());
        }
    }
    for extra in ["", " ", "zzz", "aaa", "\u{7f}", "A", "~"] {
        probes.push(extra.to_string());
    }
    let mut rng = Rng::new(0x2626);
    for _ in 0..3000 {
        let n = rng.below(9) as usize;
        probes.push(
            (0..n)
                .map(|_| b"abcdefinorstuvwxyz"[rng.below(18) as usize] as char)
                .collect(),
        );
    }

    for l in &lists {
        let owned: Vec<CString> = l.iter().map(|s| cs(s)).collect();
        let ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        let base = if ptrs.is_empty() {
            std::ptr::null()
        } else {
            ptrs.as_ptr()
        };
        for probe in &probes {
            let pb = cs(probe);
            let a = unsafe { (p.c.jsY_findword)(pb.as_ptr(), base, l.len() as c_int) };
            let b = unsafe { (p.r.jsY_findword)(pb.as_ptr(), base, l.len() as c_int) };
            assert_eq!(a, b, "jsY_findword({probe:?}, len={})", l.len());
        }
        // num = 0 and negative num terminate immediately in the C loop
        let pb = cs("a");
        for n in [0, -1, -100] {
            let a = unsafe { (p.c.jsY_findword)(pb.as_ptr(), base, n) };
            let b = unsafe { (p.r.jsY_findword)(pb.as_ptr(), base, n) };
            assert_eq!(a, b, "jsY_findword num={n}");
        }
    }
}

/// The lexer itself is driven through the public compile path so that
/// `jsY_initlex` / `jsY_lex` / `jsY_lexjson` are exercised on every token
/// shape.  Divergence shows up as a different SyntaxError message or a
/// different parsed value.
#[test]
fn lexer_token_shapes() {
    let sources = [
        "1", "1.", ".1", "1.5", "1e5", "1E5", "1e+5", "1e-5", "0", "00", "01", "08", "0x0",
        "0xFF", "0Xff", "0x", "1_0", "1n", "0b1", "0o7", "5.e3", ".e3", "1e", "1e+",
        "'a'", "\"a\"", "'\\n'", "'\\x41'", "'\\u0041'", "'\\u{41}'", "'\\0'", "'\\7'", "'\\8'",
        "'unterminated", "'a\nb'", "'\\\n'", "\"\\\r\n\"",
        "/re/", "/re/g", "/re/gi", "/re/gim", "/re/x", "/re/gg", "/re", "/[/]/", "/a\\/b/",
        "// line comment\n1", "/* block */1", "/* unterminated", "/**/1",
        "a", "$a", "_a", "a$_1", "\u{5}", "\u{e9}x", "\\u0041",
        "true", "false", "null", "this", "typeof void 0",
        "1<2", "1<=2", "1>>2", "1>>>2", "1&2", "1|2", "1^2", "~1", "!1",
        "a+=1", "a-=1", "a*=1", "a/=1", "a%=1", "a<<=1", "a>>=1", "a>>>=1", "a&=1", "a|=1", "a^=1",
        "a===b", "a!==b", "a==b", "a!=b", "a&&b", "a||b", "a++", "a--", "++a", "--a",
        "\u{feff}1", "\u{2028}1", "\u{2029}1", "\u{a0}1", "\t\u{b}\u{c} 1",
        "#!/shebang\n1", "1;;;", "{}", "({})", "[,]", "[,,]", "(1,2)",
    ];
    for s in sources {
        diff_eval_both_modes(s);
    }
}

#[test]
fn lexer_json_mode() {
    let sources = [
        "JSON.parse('1')",
        "JSON.parse('1.5')",
        "JSON.parse('-1.5e3')",
        "JSON.parse('1e')",
        "JSON.parse('01')",
        "JSON.parse('+1')",
        "JSON.parse('.5')",
        "JSON.parse('5.')",
        "JSON.parse('\"a\"')",
        "JSON.parse('\"\\\\u0041\"')",
        "JSON.parse('\"\\\\x41\"')",
        "JSON.parse('\"\\\\/\"')",
        "JSON.parse('\"\\\\b\\\\f\\\\n\\\\r\\\\t\"')",
        "JSON.parse('\"a\\u0001b\"')",
        "JSON.parse('true')",
        "JSON.parse('False')",
        "JSON.parse('null')",
        "JSON.parse('[]')",
        "JSON.parse('[1,2,3]')",
        "JSON.parse('[1,]')",
        "JSON.parse('{}')",
        "JSON.parse('{\"a\":1}')",
        "JSON.parse('{a:1}')",
        "JSON.parse('{\"a\":1,}')",
        "JSON.parse('')",
        "JSON.parse('  ')",
        "JSON.parse('1 2')",
        "JSON.parse('nul')",
        "JSON.parse('\\'a\\'')",
        "JSON.parse('[1,[2,[3,[4]]]]')",
        "JSON.stringify(JSON.parse('{\"a\":[1,2,{\"b\":null}]}'))",
    ];
    for s in sources {
        diff_eval_both_modes(s);
    }
}

use LTRE::ltre::*;

static mut FAIL_COUNT: i32 = 0;

struct Test {
    regex: &'static str,
    input: &'static [u8],
    matches: bool,
    errors: bool,
    partial: bool,
    ignorecase: bool,
    complement: bool,
    quick: bool,
}

fn run_test(args: Test) {
    run_test_str(args.regex, args.input, args.matches, args.errors, args.partial, args.ignorecase, args.complement, args.quick);
}

fn test_str(regex: &str, input: &[u8], matches: bool, partial: bool, ignorecase: bool, complement: bool, quick: bool) {
    run_test_str(regex, input, matches, false, partial, ignorecase, complement, quick);
}

fn run_test_str(regex: &str, input: &[u8], expected_matches: bool, errors: bool, partial: bool, ignorecase: bool, complement: bool, quick: bool) {
    let parse_result = ltre_parse(regex);

    if errors {
        if parse_result.is_ok() {
            eprintln!("test failed: /{}/ parse (expected error)", regex);
            unsafe { FAIL_COUNT += 1; }
        }
        return;
    }

    let mut nfa = match parse_result {
        Ok(n) => n,
        Err(e) => {
            eprintln!("test failed: /{}/ parse error: {}", regex, e);
            unsafe { FAIL_COUNT += 1; }
            return;
        }
    };

    if partial { ltre_partial(&mut nfa).unwrap(); }
    if ignorecase { ltre_ignorecase(&mut nfa).unwrap(); }
    if complement { ltre_complement(&mut nfa); }

    // NFA -> DFA
    let dfa = ltre_compile(nfa.clone());

    // DFA -> BUF -> DFA -> NFA -> DFA
    let buf = dfa_serialize(&dfa);
    let (dfa2, _) = dfa_deserialize(&buf).unwrap();
    let nfa2 = ltre_uncompile(&dfa2);
    let dfa3 = ltre_compile(nfa2);

    if !quick {
        // DFA -> RE -> NFA -> DFA
        let re = ltre_decompile(&dfa3);
        match ltre_parse(&re) {
            Ok(nfa3) => {
                let dfa4 = ltre_compile(nfa3);
                let r = ltre_matches(&dfa4, input);
                if r != expected_matches {
                    eprintln!("test failed (decompile): /{}/ -> /{}/ against '{}'", regex, re, String::from_utf8_lossy(input));
                    unsafe { FAIL_COUNT += 1; }
                    return;
                }
            }
            Err(e) => {
                eprintln!("test failed (decompile parse): /{}/ -> /{}/ error: {}", regex, re, e);
                unsafe { FAIL_COUNT += 1; }
                return;
            }
        }
    }

    // Check matches
    let r1 = ltre_matches(&dfa3, input);
    let mut ldfa = None;
    let r2 = ltre_matches_lazy(&mut ldfa, &nfa, input);
    if r1 != expected_matches || r2 != expected_matches {
        eprintln!("test failed: /{}/ against '{}' (dfa={}, lazy={}, expected={})",
            regex, String::from_utf8_lossy(input), r1, r2, expected_matches);
        unsafe { FAIL_COUNT += 1; }
    }
}

macro_rules! test {
    ($regex:expr $(, $input:expr, $matches:expr)? $(, .errors = $errors:expr)? $(, .partial = $partial:expr)? $(, .ignorecase = $ic:expr)? $(, .complement = $comp:expr)? $(, .quick = $quick:expr)?) => {
        {
            #[allow(unused_mut, unused_assignments)]
            let mut t = Test {
                regex: $regex,
                input: b"",
                matches: false,
                errors: false,
                partial: false,
                ignorecase: false,
                complement: false,
                quick: false,
            };
            $(t.input = $input; t.matches = $matches;)?
            $(t.errors = $errors;)?
            $(t.partial = $partial;)?
            $(t.ignorecase = $ic;)?
            $(t.complement = $comp;)?
            $(t.quick = $quick;)?
            run_test(t);
        }
    };
}

fn main() {
    // catastrophic backtracking
    test!("(a*)*c", b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", false);
    test!("(x+x+)+y", b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", false);

    // determinization state blowout
    test!("[01]*1[01]{8}", b"11011100011100", true, .quick = true);
    test!("[01]*1[01]{8}", b"01010010010010", false, .quick = true);

    // powerset construction state blowout
    test!(".*0.*|.*1.*|.*2.*|.*3.*|.*4.*|.*5.*", b"", false);
    test!(".*0.*|.*1.*|.*2.*|.*3.*|.*4.*|.*5.*", b"123", true);

    // potential edge cases
    test!("abba", b"abba", true);
    test!("[ab]+", b"abba", true);
    test!("[ab]+", b"abc", false);
    test!(".*", b"abba", true);
    test!("(a|b+){3}", b"abbba", true);
    test!("(a|b+){3}", b"abbab", false);
    test!("\\x61\\+", b"a+", true);
    test!("", b"", true);
    test!("[]", b"", false);
    test!("[]*", b"", true);
    test!("[]+", b"", false);
    test!("[]?", b"", true);
    test!("()", b"", true);
    test!("()*", b"", true);
    test!("()+", b"", true);
    test!("()?", b"", true);
    test!(" ", b" ", true);
    test!("", b"\n", false);
    test!("\\n", b"\n", true);
    test!(".", b"\n", false);
    test!("\\\\n", b"\n", false);
    test!("(|n)(\\n)", b"\n", true);
    test!("\\r?\\n", b"\n", true);
    test!("\\r?\\n", b"\r\n", true);
    test!("(a*)*", b"a", true);
    test!("(a+)+", b"aa", true);
    test!("(a?)?", b"", true);
    test!("a+", b"aa", true);
    test!("a?", b"aa", false);
    test!("(a+)?", b"aa", true);
    test!("(ba+)?", b"baa", true);
    test!("(ab+)?", b"b", false);
    test!("(a+b)?", b"a", false);
    test!("(a+a+)+", b"a", false);
    test!("a+", b"", false);
    test!("(a+|)+", b"aa", true);
    test!("(a+|)+", b"", true);
    test!("(a|b)?", b"a", true);
    test!("(a|b)?", b"b", true);
    test!("x*|", b"xx", true);
    test!("x*|", b"", true);
    test!("x+|", b"xx", true);
    test!("x+|", b"", true);
    test!("x?|", b"x", true);
    test!("x?|", b"", true);
    test!("x*y*", b"yx", false);
    test!("x+y+", b"yx", false);
    test!("x?y?", b"yx", false);
    test!("x+y*", b"xyx", false);
    test!("x*y+", b"yxy", false);
    test!("x*|y*", b"xy", false);
    test!("x+|y+", b"xy", false);
    test!("x?|y?", b"xy", false);
    test!("x+|y*", b"xy", false);
    test!("x*|y+", b"xy", false);
    test!("a{1,2}", b"", false);
    test!("a{1,2}", b"a", true);
    test!("a{1,2}", b"aa", true);
    test!("a{1,2}", b"aaa", false);
    test!("a{0,}", b"", true);
    test!("a{0,}", b"a", true);
    test!("a{0,}", b"aa", true);
    test!("a{0,}", b"aaa", true);
    test!("a{1,}", b"", false);
    test!("a{1,}", b"a", true);
    test!("a{1,}", b"aa", true);
    test!("a{1,}", b"aaa", true);
    test!("a{3,}", b"aa", false);
    test!("a{3,}", b"aaa", true);
    test!("a{3,}", b"aaaa", true);
    test!("a{3,}", b"aaaaa", true);
    test!("a{0,2}", b"", true);
    test!("a{0,2}", b"a", true);
    test!("a{0,2}", b"aa", true);
    test!("a{0,2}", b"aaa", false);
    test!("a{2}", b"a", false);
    test!("a{2}", b"aa", true);
    test!("a{2}", b"aaa", false);
    test!("a{0}", b"", true);
    test!("a{0}", b"a", false);

    // partial, ignorecase, complement
    test!("", b"", true, .partial = true);
    test!("", b"abc", true, .partial = true);
    test!("b", b"abc", true, .partial = true);
    test!("ba", b"abc", false, .partial = true);
    test!("abc", b"abc", true, .partial = true);
    test!("[]", b"", false, .partial = true);
    test!("", b"", true, .ignorecase = true);
    test!("abCdEF", b"aBCdEf", true, .ignorecase = true);
    test!("ab", b"abc", false, .ignorecase = true);
    test!("a", b"", true, .complement = true);
    test!("a", b"aa", true, .complement = true);
    test!("a", b"a", false, .complement = true);
    test!("ab*", b"ac", true, .complement = true);
    test!("ab*", b"abb", false, .complement = true);

    // decompilation edge cases
    test!("^aa*", b"ba", true);
    test!("a-zz*", b"abc", false);
    test!("\\x0a(0a)*", b"\x0a", true);
    test!("\\x0aa*", b"\x0a\x0a", false);

    // parse errors
    test!("abc]", .errors = true);
    test!("[abc", .errors = true);
    test!("abc)", .errors = true);
    test!("(abc", .errors = true);
    test!("+a", .errors = true);
    test!("a|*", .errors = true);
    test!("\\x0", .errors = true);
    test!("\\zzz", .errors = true);
    test!("[a\\x]", .errors = true);
    test!("\x08", .errors = true);
    test!("\t", .errors = true);
    test!("^^a", .errors = true);
    test!("a**", .errors = true);
    test!("a*+", .errors = true);
    test!("a*?", .errors = true);
    test!("a+*", .errors = true);
    test!("a++", .errors = true);
    test!("a+?", .errors = true);
    test!("a?*", .errors = true);
    test!("a?+", .errors = true);
    test!("a??", .errors = true);
    test!("a{9999999999999999999999999999999999999999}", .errors = true);
    test!("a{9999999999999999999999999999999999999999,}", .errors = true);
    test!("a{,9999999999999999999999999999999999999999}", .errors = true);
    test!("a{9999999999999999999999999999999999999999,9999999999999999999999999999999999999999}", .errors = true);

    // nonstandard features
    test!("^a", b"z", true);
    test!("^a", b"a", false);
    test!("^\\n", b"\r", true);
    test!("^\\n", b"\n", false);
    test!("^.", b"\n", true);
    test!("^.", b"a", false);
    test!("\\d+", b"0123456789", true);
    test!("\\s+", b" \x0c\n\r\t\x0b", true);
    test!("\\w+", b"azAZ09_", true);
    test!("^a-z*", b"1A!2$B", true);
    test!("^a-z*", b"1aA", false);
    test!("a-z*", b"abc", true);
    test!("^[\\d^\\w]+", b"abcABC", true);
    test!("^[\\d^\\w]+", b"abc123", false);
    test!("^[\\d\\W]+", b"abcABC", true);
    test!("^[\\d^\\W]+", b"abc123", false);
    test!("[[abc]]+", b"abc", true);
    test!("[a[bc]]+", b"abc", true);
    test!("[a[b]c]+", b"abc", true);
    test!("[a][b][c]", b"abc", true);
    test!("^[^a^b]", b"a", false);
    test!("^[^a^b]", b"b", false);
    test!("^[^a^b]", b"", false);
    test!("<ab>", b"a", false);
    test!("<ab>", b"b", false);
    test!("<ab>", b"", false);
    test!("\\^", b"^", true);
    test!("^\\^", b"^", false);
    test!("^[^\\^]", b"^", true);
    test!("^[ ^[a b c]]+", b"abc", true);
    test!("^[ ^[a b c]]+", b"a c", false);
    test!("<[a b c]^ >+", b"abc", true);
    test!("<[a b c]^ >+", b"a c", false);
    test!("^[^0-74]+", b"0123567", true);
    test!("^[^0-74]+", b"89", false);
    test!("^[^0-74]+", b"4", false);
    test!("<0-7^4>+", b"0123567", true);
    test!("<0-7^4>+", b"89", false);
    test!("<0-7^4>+", b"4", false);
    test!("[]", b" ", false);
    test!("^[]", b" ", true);
    test!("<>", b" ", true);
    test!("^<>", b" ", false);
    test!("9-0*", b"abc", true);
    test!("9-0*", b"18", false);
    test!("9-0*", b"09", true);
    test!("9-0*", b"/:", true);
    test!("b-a*", b"ab", true);
    test!("a-b*", b"ab", true);
    test!("a-a*", b"ab", false);
    test!("a-a*", b"aa", true);
    test!("a{,2}", b"", true);
    test!("a{,2}", b"a", true);
    test!("a{,2}", b"aa", true);
    test!("a{,2}", b"aaa", false);
    test!("a{}", b"", true);
    test!("a{}", b"a", false);
    test!("a{,}", b"", true);
    test!("a{,}", b"a", true);
    test!("~0*", b"", false);
    test!("~0*", b"0", false);
    test!("~0*", b"00", false);
    test!("~0*", b"001", true);
    test!("ab&cd", b"", false);
    test!("ab&cd", b"ab", false);
    test!("ab&cd", b"cd", false);
    test!("\\w+&~\\d+", b"", false);
    test!("\\w+&~\\d+", b"abc", true);
    test!("\\w+&~\\d+", b"abc123", true);
    test!("\\w+&~\\d+", b"1a2b3c", true);
    test!("\\w+&~\\d+", b"123", false);
    test!("0x(~[0-9a-f]+)", b"0yz", false);
    test!("0x(~[0-9a-f]+)", b"0x12", false);
    test!("0x(~[0-9a-f]+)", b"0x", true);
    test!("0x(~[0-9a-f]+)", b"0xy", true);
    test!("0x(~[0-9a-f]+)", b"0xyz", true);
    test!("b(~a*)", b"", false);
    test!("b(~a*)", b"b", false);
    test!("b(~a*)", b"ba", false);
    test!("b(~a*)", b"bbaa", true);
    test!("abc>", .errors = true);
    test!("<abc", .errors = true);
    test!("[a?b]", .errors = true);
    test!("[a-]", .errors = true);
    test!("[--]", .errors = true);
    test!("[-]", .errors = true);
    test!("-", .errors = true);
    test!("a-", .errors = true);
    test!("a*{}", .errors = true);
    test!("a+{}", .errors = true);
    test!("a?{}", .errors = true);
    test!("a{}*", .errors = true);
    test!("a{}+", .errors = true);
    test!("a{}?", .errors = true);
    test!("a{}{}", .errors = true);
    test!("a{2,1}", .errors = true);
    test!("a{1 2}", .errors = true);
    test!("a{1, 2}", .errors = true);
    test!("a{a}", .errors = true);
    test!("a~b", .errors = true);

    // realistic regexes
    test!("#([0-9a-fA-F]{3}){1,2}", b"000", false);
    test!("#([0-9a-fA-F]{3}){1,2}", b"#0aA", true);
    test!("#([0-9a-fA-F]{3}){1,2}", b"#00ff", false);
    test!("#([0-9a-fA-F]{3}){1,2}", b"#abcdef", true);
    test!("#([0-9a-fA-F]{3}){1,2}", b"#abcdeff", false);

    // JSON number
    test!("\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?", b"e", false);
    test!("\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?", b"1", true);
    test!("\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?", b"10", true);
    test!("\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?", b"01", false);
    test!("\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?", b"-5", true);
    test!("\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?", b"+5", false);
    test!("\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?", b".3", false);
    test!("\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?", b"2.", false);
    test!("\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?", b"2.3", true);
    test!("\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?", b"1e0", true);
    test!("\\-?(0|1-90-9*)(\\.0-9+)?([eE][\\+\\-]?0-9+)?", b"1E+0", true);

    // JSON bool/null
    test!("true|false", b"true", true);
    test!("true|false", b"false", true);
    test!("null", b"null", true);
    test!("null", b"nul", false);

    // printf format spec tests
    let field_width = "(\\*|1-90-9*)?";
    let precision = "(\\.|\\.\\*|\\.1-90-9*)?";
    let di = format!("[\\-\\+ 0]*{}{}([hljzt]|hh|ll)?[di]", field_width, precision);
    let u = format!("[\\-0]*{}{}([hljzt]|hh|ll)?u", field_width, precision);
    let ox = format!("[\\-#0]*{}{}([hljzt]|hh|ll)?[oxX]", field_width, precision);
    let fega = format!("[\\-\\+ #0]*{}{}[lL]?[fFeEgGaA]", field_width, precision);
    let c = format!("\\-*{}l?c", field_width);
    let s = format!("\\-*{}{}l?s", field_width, precision);
    let p = format!("\\-*{}p", field_width);
    let n = format!("{}([hljzt]|hh|ll)?n", field_width);
    let conv_spec = format!("%({}|{}|{}|{}|{}|{}|{}|{}|%)", di, u, ox, fega, c, s, p, n);
    let cs = &conv_spec;
    test_str(cs, b"%", false, false, false, false, false);
    test_str(cs, b"%*", false, false, false, false, false);
    test_str(cs, b"%%", true, false, false, false, false);
    test_str(cs, b"%5%", false, false, false, false, false);
    test_str(cs, b"%p", true, false, false, false, false);
    test_str(cs, b"%*p", true, false, false, false, false);
    test_str(cs, b"% *p", false, false, false, false, false);
    test_str(cs, b"%5p", true, false, false, false, false);
    test_str(cs, b"d", false, false, false, false, false);
    test_str(cs, b"%d", true, false, false, false, false);
    test_str(cs, b"%.16s", true, false, false, false, false);
    test_str(cs, b"% 5.3f", true, false, false, false, false);
    test_str(cs, b"%*32.4g", false, false, false, false, false);
    test_str(cs, b"%-#65.4g", true, false, false, false, false);
    test_str(cs, b"%03c", false, false, false, false, false);
    test_str(cs, b"%06i", true, false, false, false, false);
    test_str(cs, b"%lu", true, false, false, false, false);
    test_str(cs, b"%hhu", true, false, false, false, false);
    test_str(cs, b"%Lu", false, false, false, false, false);
    test_str(cs, b"%-*p", true, false, false, false, false);
    test_str(cs, b"%-.*p", false, false, false, false, false);
    test_str(cs, b"%id", false, false, false, false, false);
    test_str(cs, b"%%d", false, false, false, false, false);
    test_str(cs, b"i%d", false, false, false, false, false);
    test_str(cs, b"%c%s", false, false, false, false, false);
    test_str(cs, b"%0n", false, false, false, false, false);
    test_str(cs, b"% u", false, false, false, false, false);
    test_str(cs, b"%+c", false, false, false, false, false);
    test_str(cs, b"%0-++ 0i", true, false, false, false, false);
    test_str(cs, b"%30c", true, false, false, false, false);
    test_str(cs, b"%03c", false, false, false, false, false);

    // C identifier tests
    let hex_quad = "[0-9a-fA-F]{4}";
    let keyword = "(auto|break|case|char|const|continue|default|do|double|else|enum|extern|float|for|goto|if|inline|int|long|register|restrict|return|short|signed|sizeof|static|struct|switch|typedef|union|unsigned|void|volatile|while|_Bool|_Complex|_Imaginary)";
    let identifier = format!("(\\w|\\\\u{}|\\\\U{}{})*&~\\d.*&~{}", hex_quad, hex_quad, hex_quad, keyword);
    let id = &identifier;
    test_str(id, b"_", true, false, false, false, false);
    test_str(id, b"_foo", true, false, false, false, false);
    test_str(id, b"_Bool", false, false, false, false, false);
    test_str(id, b"a1", true, false, false, false, false);
    test_str(id, b"5b", false, false, false, false, false);
    test_str(id, b"if", false, false, false, false, false);
    test_str(id, b"ifa", true, false, false, false, false);
    test_str(id, b"bif", true, false, false, false, false);
    test_str(id, b"if2", true, false, false, false, false);
    test_str(id, b"1if", false, false, false, false, false);
    test_str(id, b"\\u12", false, false, false, false, false);
    test_str(id, b"\\u1A2b", true, false, false, false, false);
    test_str(id, b"\\u1234", true, false, false, false, false);
    test_str(id, b"\\u123x", false, false, false, false, false);
    test_str(id, b"\\u1234x", true, false, false, false, false);
    test_str(id, b"\\U12345678", true, false, false, false, false);
    test_str(id, b"\\U1234567y", false, false, false, false, false);
    test_str(id, b"\\U12345678y", true, false, false, false, false);

    // JSON string tests
    let json_str = format!("\"(^[\\x00-\\x1f\"\\\\]|\\\\[\"\\\\/bfnrt]|\\\\u{})*\"", hex_quad);
    let js = &json_str;
    test_str(js, b"foo", false, false, false, false, false);
    test_str(js, b"\"foo", false, false, false, false, false);
    test_str(js, b"\"\"", true, false, false, false, false);
    test_str(js, b"\"foo\"", true, false, false, false, false);
    test_str(js, b"\"foo\\\"\"", true, false, false, false, false);
    test_str(js, b"\"foo\\\\\"", true, false, false, false, false);
    test_str(js, b"\"\\nbar\"", true, false, false, false, false);
    test_str(js, b"\"\nbar\"", false, false, false, false, false);
    test_str(js, b"\"\\abar\"", false, false, false, false, false);
    test_str(js, b"\"\\u1A2b\"", true, false, false, false, false);
    test_str(js, b"\"\\uDEAD\"", true, false, false, false, false);
    test_str(js, b"\"\\uF00\"", false, false, false, false, false);
    test_str(js, b"\"\\uF00BAR\"", true, false, false, false, false);
    test_str(js, b"\"foo\\/\"", true, false, false, false, false);
    test_str(js, b"\"\xcf\x84\"", true, false, false, false, false);
    test_str(js, b"\"\x80\"", true, false, false, false, false);
    test_str(js, b"\"\x88x/\"", true, false, false, false, false);

    // UTF-8 tests
    let tail = "\\x80-\\xbf";
    let utf8_1 = "\\x00-\\x7f";
    let utf8_2 = format!("\\xc2-\\xdf{}", tail);
    let utf8_3 = format!("\\xe0\\xa0-\\xbf{}|\\xe1-\\xec{}{}|\\xed\\x80-\\x9f{}|\\xee-\\xef{}{}", tail, tail, tail, tail, tail, tail);
    let utf8_4 = format!("\\xf0\\x90-\\xbf{}{}|\\xf1-\\xf3{}{}{}|\\xf4\\x80-\\x8f{}{}", tail, tail, tail, tail, tail, tail, tail);
    let utf8_char_2 = format!("({}|{}|{}|{})", utf8_1, utf8_2, utf8_3, utf8_4);
    let utf8_chars_2 = format!("{}*", utf8_char_2);
    let uc2 = &utf8_chars_2;
    test_str(uc2, b"\xc2\x7f", false, false, false, false, false);
    test_str(uc2, b"\xe2\x28\xa1", false, false, false, false, false);
    test_str(uc2, b"\x80x/", false, false, false, false, false);
    test_str(uc2, b"\x41\xe2\x89\xa2\xce\x91\x2e", true, false, false, false, false);
    test_str(uc2, b"\xed\x95\x9c\xea\xb5\xad\xec\x96\xb4", true, false, false, false, false);
    test_str(uc2, b"abcABC123<=>", true, false, false, false, false);
    test_str(uc2, b"\xc2\x80", true, false, false, false, false);

    unsafe {
        if FAIL_COUNT > 0 {
            eprintln!("{} tests failed", FAIL_COUNT);
            std::process::exit(1);
        } else {
            println!("All tests passed!");
        }
    }
}

#![allow(non_snake_case)]
use LTRE::ltre::*;

#[derive(Default, Clone)]
struct TArgs {
    regex: &'static str,
    input: &'static str,
    matches: bool,
    errors: bool,
    partial: bool,
    ignorecase: bool,
    complement: bool,
    quick: bool,
}

fn t(args: TArgs) {
    let parsed = ltre_parse(args.regex);
    if parsed.is_err() != args.errors {
        eprintln!(
            "FAIL: /{}/ parse (err={:?}, expected_err={})",
            args.regex,
            parsed.as_ref().err(),
            args.errors
        );
        return;
    }
    let mut nfa = match parsed {
        Ok(n) => n,
        Err(_) => return,
    };
    if args.partial {
        ltre_partial(&mut nfa).unwrap();
    }
    if args.ignorecase {
        ltre_ignorecase(&mut nfa).unwrap();
    }
    if args.complement {
        ltre_complement(&mut nfa);
    }
    let dfa = ltre_compile(nfa.clone());

    // Round-trip via serialize
    let buf = dfa_serialize(&dfa);
    let (dfa2, _) = dfa_deserialize(&buf).unwrap();
    let nfa2 = ltre_uncompile(&dfa2);
    let dfa3 = ltre_compile(nfa2.clone());

    let final_dfa = if !args.quick {
        let re = ltre_decompile(&dfa3);
        match ltre_parse(&re) {
            Ok(n) => ltre_compile(n),
            Err(e) => {
                eprintln!("FAIL: /{}/ decompile-reparse: {} for {:?}", args.regex, e, re);
                dfa3
            }
        }
    } else {
        dfa3
    };

    let actual = ltre_matches(&final_dfa, args.input.as_bytes());
    let mut ldfa = None;
    let lazy_actual = ltre_matches_lazy(&mut ldfa, &nfa, args.input.as_bytes());
    if actual != args.matches {
        eprintln!(
            "FAIL: /{}/ against '{}' expected {} got {}",
            args.regex, args.input, args.matches, actual
        );
    }
    if lazy_actual != args.matches {
        eprintln!(
            "FAIL LAZY: /{}/ against '{}' expected {} got {}",
            args.regex, args.input, args.matches, lazy_actual
        );
    }
}

macro_rules! test {
    ($($field:ident: $val:expr),* $(,)?) => {
        t(TArgs { $($field: $val,)* ..Default::default() })
    };
}

fn main() {
    // catastrophic backtracking
    test!(regex: "(a*)*c", input: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", matches: false);
    test!(regex: "(x+x+)+y", input: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", matches: false);

    // determinization state blowout
    test!(regex: "[01]*1[01]{8}", input: "11011100011100", matches: true, quick: true);
    test!(regex: "[01]*1[01]{8}", input: "01010010010010", matches: false, quick: true);

    // powerset construction state blowout
    test!(regex: ".*0.*|.*1.*|.*2.*|.*3.*|.*4.*|.*5.*", input: "", matches: false);
    test!(regex: ".*0.*|.*1.*|.*2.*|.*3.*|.*4.*|.*5.*", input: "123", matches: true);

    // potential edge cases
    test!(regex: "abba", input: "abba", matches: true);
    test!(regex: "[ab]+", input: "abba", matches: true);
    test!(regex: "[ab]+", input: "abc", matches: false);
    test!(regex: ".*", input: "abba", matches: true);
    test!(regex: "(a|b+){3}", input: "abbba", matches: true);
    test!(regex: "(a|b+){3}", input: "abbab", matches: false);
    test!(regex: "\\x61\\+", input: "a+", matches: true);
    test!(regex: "", input: "", matches: true);
    test!(regex: "[]", input: "", matches: false);
    test!(regex: "[]*", input: "", matches: true);
    test!(regex: "[]+", input: "", matches: false);
    test!(regex: "[]?", input: "", matches: true);
    test!(regex: "()", input: "", matches: true);
    test!(regex: "()*", input: "", matches: true);
    test!(regex: "()+", input: "", matches: true);
    test!(regex: "()?", input: "", matches: true);
    test!(regex: " ", input: " ", matches: true);
    test!(regex: "", input: "\n", matches: false);
    test!(regex: "\\n", input: "\n", matches: true);
    test!(regex: ".", input: "\n", matches: false);
    test!(regex: "\\\\n", input: "\n", matches: false);
    test!(regex: "(|n)(\\n)", input: "\n", matches: true);
    test!(regex: "\\r?\\n", input: "\n", matches: true);
    test!(regex: "\\r?\\n", input: "\r\n", matches: true);
    test!(regex: "(a*)*", input: "a", matches: true);
    test!(regex: "(a+)+", input: "aa", matches: true);
    test!(regex: "(a?)?", input: "", matches: true);
    test!(regex: "a+", input: "aa", matches: true);
    test!(regex: "a?", input: "aa", matches: false);
    test!(regex: "(a+)?", input: "aa", matches: true);
    test!(regex: "(ba+)?", input: "baa", matches: true);
    test!(regex: "(ab+)?", input: "b", matches: false);
    test!(regex: "(a+b)?", input: "a", matches: false);
    test!(regex: "(a+a+)+", input: "a", matches: false);
    test!(regex: "a+", input: "", matches: false);
    test!(regex: "(a+|)+", input: "aa", matches: true);
    test!(regex: "(a+|)+", input: "", matches: true);
    test!(regex: "(a|b)?", input: "a", matches: true);
    test!(regex: "(a|b)?", input: "b", matches: true);
    test!(regex: "x*|", input: "xx", matches: true);
    test!(regex: "x*|", input: "", matches: true);
    test!(regex: "x+|", input: "xx", matches: true);
    test!(regex: "x+|", input: "", matches: true);
    test!(regex: "x?|", input: "x", matches: true);
    test!(regex: "x?|", input: "", matches: true);
    test!(regex: "x*y*", input: "yx", matches: false);
    test!(regex: "x+y+", input: "yx", matches: false);
    test!(regex: "x?y?", input: "yx", matches: false);
    test!(regex: "x+y*", input: "xyx", matches: false);
    test!(regex: "x*y+", input: "yxy", matches: false);
    test!(regex: "x*|y*", input: "xy", matches: false);
    test!(regex: "x+|y+", input: "xy", matches: false);
    test!(regex: "x?|y?", input: "xy", matches: false);
    test!(regex: "x+|y*", input: "xy", matches: false);
    test!(regex: "x*|y+", input: "xy", matches: false);
    test!(regex: "a{1,2}", input: "", matches: false);
    test!(regex: "a{1,2}", input: "a", matches: true);
    test!(regex: "a{1,2}", input: "aa", matches: true);
    test!(regex: "a{1,2}", input: "aaa", matches: false);
    test!(regex: "a{0,}", input: "", matches: true);
    test!(regex: "a{0,}", input: "a", matches: true);
    test!(regex: "a{0,}", input: "aa", matches: true);
    test!(regex: "a{0,}", input: "aaa", matches: true);
    test!(regex: "a{1,}", input: "", matches: false);
    test!(regex: "a{1,}", input: "a", matches: true);
    test!(regex: "a{1,}", input: "aa", matches: true);
    test!(regex: "a{1,}", input: "aaa", matches: true);
    test!(regex: "a{3,}", input: "aa", matches: false);
    test!(regex: "a{3,}", input: "aaa", matches: true);
    test!(regex: "a{3,}", input: "aaaa", matches: true);
    test!(regex: "a{3,}", input: "aaaaa", matches: true);
    test!(regex: "a{0,2}", input: "", matches: true);
    test!(regex: "a{0,2}", input: "a", matches: true);
    test!(regex: "a{0,2}", input: "aa", matches: true);
    test!(regex: "a{0,2}", input: "aaa", matches: false);
    test!(regex: "a{2}", input: "a", matches: false);
    test!(regex: "a{2}", input: "aa", matches: true);
    test!(regex: "a{2}", input: "aaa", matches: false);
    test!(regex: "a{0}", input: "", matches: true);
    test!(regex: "a{0}", input: "a", matches: false);

    // partial, ignorecase, complement
    test!(regex: "", input: "", matches: true, partial: true);
    test!(regex: "", input: "abc", matches: true, partial: true);
    test!(regex: "b", input: "abc", matches: true, partial: true);
    test!(regex: "ba", input: "abc", matches: false, partial: true);
    test!(regex: "abc", input: "abc", matches: true, partial: true);
    test!(regex: "[]", input: "", matches: false, partial: true);
    test!(regex: "", input: "", matches: true, ignorecase: true);
    test!(regex: "abCdEF", input: "aBCdEf", matches: true, ignorecase: true);
    test!(regex: "ab", input: "abc", matches: false, ignorecase: true);
    test!(regex: "a", input: "", matches: true, complement: true);
    test!(regex: "a", input: "aa", matches: true, complement: true);
    test!(regex: "a", input: "a", matches: false, complement: true);
    test!(regex: "ab*", input: "ac", matches: true, complement: true);
    test!(regex: "ab*", input: "abb", matches: false, complement: true);

    // decompilation edge cases
    test!(regex: "^aa*", input: "ba", matches: true);
    test!(regex: "a-zz*", input: "abc", matches: false);
    test!(regex: "\\x0a(0a)*", input: "\x0a", matches: true);
    test!(regex: "\\x0aa*", input: "\x0a\x0a", matches: false);

    // parse errors
    test!(regex: "abc]", errors: true);
    test!(regex: "[abc", errors: true);
    test!(regex: "abc)", errors: true);
    test!(regex: "(abc", errors: true);
    test!(regex: "+a", errors: true);
    test!(regex: "a|*", errors: true);
    test!(regex: "\\x0", errors: true);
    test!(regex: "\\zzz", errors: true);
    test!(regex: "[a\\x]", errors: true);
    test!(regex: "\x08", errors: true);
    test!(regex: "\t", errors: true);
    test!(regex: "^^a", errors: true);
    test!(regex: "a**", errors: true);
    test!(regex: "a*+", errors: true);
    test!(regex: "a*?", errors: true);
    test!(regex: "a+*", errors: true);
    test!(regex: "a++", errors: true);
    test!(regex: "a+?", errors: true);
    test!(regex: "a?*", errors: true);
    test!(regex: "a?+", errors: true);
    test!(regex: "a??", errors: true);
    test!(regex: "a{9999999999999999999999999999999999999999}", errors: true);
    test!(regex: "a{9999999999999999999999999999999999999999,}", errors: true);
    test!(regex: "a{,9999999999999999999999999999999999999999}", errors: true);
    test!(regex: "a{9999999999999999999999999999999999999999,9999999999999999999999999999999999999999}", errors: true);

    // nonstandard features
    test!(regex: "^a", input: "z", matches: true);
    test!(regex: "^a", input: "a", matches: false);
    test!(regex: "^\\n", input: "\r", matches: true);
    test!(regex: "^\\n", input: "\n", matches: false);
    test!(regex: "^.", input: "\n", matches: true);
    test!(regex: "^.", input: "a", matches: false);
    test!(regex: "\\d+", input: "0123456789", matches: true);
    test!(regex: "\\s+", input: " \x0c\n\r\t\x0b", matches: true);
    test!(regex: "\\w+", input: "azAZ09_", matches: true);
    test!(regex: "^a-z*", input: "1A!2$B", matches: true);
    test!(regex: "^a-z*", input: "1aA", matches: false);
    test!(regex: "a-z*", input: "abc", matches: true);
    test!(regex: "^[\\d^\\w]+", input: "abcABC", matches: true);
    test!(regex: "^[\\d^\\w]+", input: "abc123", matches: false);
    test!(regex: "^[\\d\\W]+", input: "abcABC", matches: true);
    test!(regex: "^[\\d^\\W]+", input: "abc123", matches: false);
    test!(regex: "[[abc]]+", input: "abc", matches: true);
    test!(regex: "[a[bc]]+", input: "abc", matches: true);
    test!(regex: "[a[b]c]+", input: "abc", matches: true);
    test!(regex: "[a][b][c]", input: "abc", matches: true);
    test!(regex: "^[^a^b]", input: "a", matches: false);
    test!(regex: "^[^a^b]", input: "b", matches: false);
    test!(regex: "^[^a^b]", input: "", matches: false);
    test!(regex: "<ab>", input: "a", matches: false);
    test!(regex: "<ab>", input: "b", matches: false);
    test!(regex: "<ab>", input: "", matches: false);
    test!(regex: "\\^", input: "^", matches: true);
    test!(regex: "^\\^", input: "^", matches: false);
    test!(regex: "^[^\\^]", input: "^", matches: true);
    test!(regex: "^[ ^[a b c]]+", input: "abc", matches: true);
    test!(regex: "^[ ^[a b c]]+", input: "a c", matches: false);
    test!(regex: "<[a b c]^ >+", input: "abc", matches: true);
    test!(regex: "<[a b c]^ >+", input: "a c", matches: false);
    test!(regex: "^[^0-74]+", input: "0123567", matches: true);
    test!(regex: "^[^0-74]+", input: "89", matches: false);
    test!(regex: "^[^0-74]+", input: "4", matches: false);
    test!(regex: "<0-7^4>+", input: "0123567", matches: true);
    test!(regex: "<0-7^4>+", input: "89", matches: false);
    test!(regex: "<0-7^4>+", input: "4", matches: false);
    test!(regex: "[]", input: " ", matches: false);
    test!(regex: "^[]", input: " ", matches: true);
    test!(regex: "<>", input: " ", matches: true);
    test!(regex: "^<>", input: " ", matches: false);
    test!(regex: "9-0*", input: "abc", matches: true);
    test!(regex: "9-0*", input: "18", matches: false);
    test!(regex: "9-0*", input: "09", matches: true);
    test!(regex: "9-0*", input: "/:", matches: true);
    test!(regex: "b-a*", input: "ab", matches: true);
    test!(regex: "a-b*", input: "ab", matches: true);
    test!(regex: "a-a*", input: "ab", matches: false);
    test!(regex: "a-a*", input: "aa", matches: true);
    test!(regex: "a{,2}", input: "", matches: true);
    test!(regex: "a{,2}", input: "a", matches: true);
    test!(regex: "a{,2}", input: "aa", matches: true);
    test!(regex: "a{,2}", input: "aaa", matches: false);
    test!(regex: "a{}", input: "", matches: true);
    test!(regex: "a{}", input: "a", matches: false);
    test!(regex: "a{,}", input: "", matches: true);
    test!(regex: "a{,}", input: "a", matches: true);
    test!(regex: "~0*", input: "", matches: false);
    test!(regex: "~0*", input: "0", matches: false);
    test!(regex: "~0*", input: "00", matches: false);
    test!(regex: "~0*", input: "001", matches: true);
    test!(regex: "ab&cd", input: "", matches: false);
    test!(regex: "ab&cd", input: "ab", matches: false);
    test!(regex: "ab&cd", input: "cd", matches: false);
    test!(regex: "\\w+&~\\d+", input: "", matches: false);
    test!(regex: "\\w+&~\\d+", input: "abc", matches: true);
    test!(regex: "\\w+&~\\d+", input: "abc123", matches: true);
    test!(regex: "\\w+&~\\d+", input: "1a2b3c", matches: true);
    test!(regex: "\\w+&~\\d+", input: "123", matches: false);
    test!(regex: "0x(~[0-9a-f]+)", input: "0yz", matches: false);
    test!(regex: "0x(~[0-9a-f]+)", input: "0x12", matches: false);
    test!(regex: "0x(~[0-9a-f]+)", input: "0x", matches: true);
    test!(regex: "0x(~[0-9a-f]+)", input: "0xy", matches: true);
    test!(regex: "0x(~[0-9a-f]+)", input: "0xyz", matches: true);
    test!(regex: "b(~a*)", input: "", matches: false);
    test!(regex: "b(~a*)", input: "b", matches: false);
    test!(regex: "b(~a*)", input: "ba", matches: false);
    test!(regex: "b(~a*)", input: "bbaa", matches: true);
    test!(regex: "abc>", errors: true);
    test!(regex: "<abc", errors: true);
    test!(regex: "[a?b]", errors: true);
    test!(regex: "[a-]", errors: true);
    test!(regex: "[--]", errors: true);
    test!(regex: "[-]", errors: true);
    test!(regex: "-", errors: true);
    test!(regex: "a-", errors: true);
    test!(regex: "a*{}", errors: true);
    test!(regex: "a+{}", errors: true);
    test!(regex: "a?{}", errors: true);
    test!(regex: "a{}*", errors: true);
    test!(regex: "a{}+", errors: true);
    test!(regex: "a{}?", errors: true);
    test!(regex: "a{}{}", errors: true);
    test!(regex: "a{2,1}", errors: true);
    test!(regex: "a{1 2}", errors: true);
    test!(regex: "a{1, 2}", errors: true);
    test!(regex: "a{a}", errors: true);
    test!(regex: "a~b", errors: true);

    // realistic regexes - HEX_RGB
    let hex_rgb = "#([0-9a-fA-F]{3}){1,2}";
    test!(regex: hex_rgb, input: "000", matches: false);
    test!(regex: hex_rgb, input: "#0aA", matches: true);
    test!(regex: hex_rgb, input: "#00ff", matches: false);
    test!(regex: hex_rgb, input: "#abcdef", matches: true);
    test!(regex: hex_rgb, input: "#abcdeff", matches: false);

    // CONV_SPEC (printf format spec)
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
    let conv_spec = format!(
        "%({}|{}|{}|{}|{}|{}|{}|{}|%)",
        di, u, ox, fega, c, s, p, n
    );
    let cs: &str = &conv_spec;
    let cs_static = Box::leak(cs.to_string().into_boxed_str()) as &'static str;
    test!(regex: cs_static, input: "%", matches: false);
    test!(regex: cs_static, input: "%*", matches: false);
    test!(regex: cs_static, input: "%%", matches: true);
    test!(regex: cs_static, input: "%5%", matches: false);
    test!(regex: cs_static, input: "%p", matches: true);
    test!(regex: cs_static, input: "%*p", matches: true);
    test!(regex: cs_static, input: "% *p", matches: false);
    test!(regex: cs_static, input: "%5p", matches: true);
    test!(regex: cs_static, input: "d", matches: false);
    test!(regex: cs_static, input: "%d", matches: true);
    test!(regex: cs_static, input: "%.16s", matches: true);
    test!(regex: cs_static, input: "% 5.3f", matches: true);
    test!(regex: cs_static, input: "%*32.4g", matches: false);
    test!(regex: cs_static, input: "%-#65.4g", matches: true);
    test!(regex: cs_static, input: "%03c", matches: false);
    test!(regex: cs_static, input: "%06i", matches: true);
    test!(regex: cs_static, input: "%lu", matches: true);
    test!(regex: cs_static, input: "%hhu", matches: true);
    test!(regex: cs_static, input: "%Lu", matches: false);
    test!(regex: cs_static, input: "%-*p", matches: true);
    test!(regex: cs_static, input: "%-.*p", matches: false);

    println!("done2");
}

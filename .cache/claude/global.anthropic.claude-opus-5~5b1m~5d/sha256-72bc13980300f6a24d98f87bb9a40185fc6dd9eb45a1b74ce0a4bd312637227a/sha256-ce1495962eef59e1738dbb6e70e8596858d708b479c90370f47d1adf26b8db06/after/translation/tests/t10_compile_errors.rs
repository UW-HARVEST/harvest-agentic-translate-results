//! Phase C — one differential case per distinct `pcre2_compile` rejection.
//!
//! ERRORS.md rows 1-13 (argument validation) and 14-104 (pattern syntax).
//! Every case asserts that C and Rust return the SAME error code AND the same
//! `*erroroffset`, and the suite additionally reports which of the 121 public
//! compile error codes (100..=220) were actually reached.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::c_int;

/// (label, pattern, compile options, extra options)
type Case = (&'static str, &'static [u8], u32, u32);

/// Cases engineered from the `ERR<nn>` assignments in `pcre2_compile.c`,
/// `pcre2_compile_class.c` and `pcre2_compile_cgroup.c`.
fn cases() -> Vec<Case> {
    let mut v: Vec<Case> = vec![
        // ERR1..ERR9
        ("end_backslash", br"\", 0, 0),
        ("end_backslash_c", br"\c", 0, 0),
        ("unknown_escape", br"\q", 0, 0),
        ("unknown_escape_lit", br"\q", 0, PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL),
        ("quant_out_of_order", b"a{3,2}", 0, 0),
        ("quant_too_big", b"a{100000}", 0, 0),
        ("quant_too_big2", b"a{1,100000}", 0, 0),
        ("missing_sq_bracket", b"[abc", 0, 0),
        ("missing_sq_bracket2", b"[", 0, 0),
        ("missing_sq_bracket3", b"[]", 0, 0),
        ("missing_sq_bracket4", b"[^]", 0, 0),
        ("escape_invalid_in_class", br"[a\Bc]", 0, 0),
        ("escape_invalid_in_class2", br"[\A]", 0, 0),
        ("escape_invalid_in_class3", br"[\Z]", 0, 0),
        ("escape_invalid_in_class4", br"[\z]", 0, 0),
        ("escape_invalid_in_class5", br"[\G]", 0, 0),
        ("escape_invalid_in_class6", br"[\C]", 0, 0),
        ("escape_invalid_in_class7", br"[\X]", 0, 0),
        ("escape_invalid_in_class8", br"[\R]", 0, 0),
        ("escape_invalid_in_class9", br"[\K]", 0, 0),
        ("class_range_order", b"[z-a]", 0, 0),
        ("class_range_order2", b"[\\x41-\\x30]", 0, 0),
        ("quant_invalid", b"*a", 0, 0),
        ("quant_invalid2", b"+", 0, 0),
        ("quant_invalid3", b"?", 0, 0),
        ("quant_invalid4", b"{2}", 0, 0),
        ("quant_invalid5", b"a**", 0, 0),
        ("quant_invalid6", b"(*FAIL)*", 0, 0),
        // ERR11..ERR20
        ("invalid_after_parens_query", b"(?~)", 0, 0),
        ("invalid_after_parens_query2", b"(?\x01)", 0, 0),
        ("posix_class_not_in_class", b"[:alpha:]", 0, 0),
        ("posix_collating", b"[[.ch.]]", 0, 0),
        ("posix_equivalence", b"[[=a=]]", 0, 0),
        ("missing_closing_paren", b"(abc", 0, 0),
        ("missing_closing_paren2", b"(", 0, 0),
        ("missing_closing_paren3", b"(?:", 0, 0),
        ("bad_subpattern_ref", br"(a)\2", 0, 0),
        ("bad_subpattern_ref2", br"\1", 0, 0),
        ("bad_subpattern_ref3", br"(?2)(a)", 0, 0),
        ("null_pattern_via_len", b"", 0, 0),
        ("bad_options_bit", b"a", 0x1000_0000, 0),
        ("bad_options_bit2", b"a", 0x0000_0000, 0x0000_4000),
        ("bad_options_bit3", b"a", 0x1000_0000 | 0x0400_0000, 0),
        ("missing_comment_closing", b"(?#comment", 0, 0),
        ("missing_comment_closing2", b"(?#", 0, 0),
        // ERR20..ERR30
        ("unmatched_closing_paren", b")", 0, 0),
        ("unmatched_closing_paren2", b"a)b", 0, 0),
        ("missing_condition_closing", b"(?(1)", 0, 0),
        ("missing_condition_closing2", b"(?(?=a)b", 0, 0),
        ("lookbehind_not_fixed", b"(?<=a+)b", 0, 0),
        ("lookbehind_not_fixed2", b"(?<=a*)b", 0, 0),
        ("lookbehind_not_fixed3", b"(?<=a|bc)d", 0, 0),
        ("lookbehind_not_fixed4", b"(?<=(?:a|bc))d", 0, 0),
        ("zero_relative_ref", b"(?+0)", 0, 0),
        ("zero_relative_ref2", br"\g{+0}", 0, 0),
        ("zero_relative_ref3", br"(a)\g{-0}", 0, 0),
        ("too_many_cond_branches", b"(?(1)a|b|c)", 0, 0),
        ("cond_assertion_expected", b"(?(a)b)", 0, 0),
        ("cond_assertion_expected2", b"(?(1a)b)", 0, 0),
        ("bad_relative_ref", b"(?-1)", 0, 0),
        ("bad_relative_ref2", b"(?-2)(a)", 0, 0),
        ("unknown_posix_class", b"[[:qqq:]]", 0, 0),
        ("unknown_posix_class2", b"[[:^qqq:]]", 0, 0),
        // ERR34..ERR40
        ("code_point_too_big", br"\x{110000}", 0, 0),
        ("code_point_too_big2", br"\x{ffffffff}", 0, 0),
        ("code_point_too_big3", br"\o{7777777777}", 0, 0),
        ("lookbehind_too_complicated", b"(?<=a{2,3}b)c", 0, 0),
        ("lookbehind_bad_backslash_c", br"(?<=\Ca)b", 0, 0),
        ("unsupported_escape", br"\L", 0, 0),
        ("unsupported_escape2", br"\l", 0, 0),
        ("unsupported_escape3", br"\U", 0, 0),
        ("unsupported_escape4", br"\u", 0, 0),
        ("callout_number_too_big", b"(?C300)", 0, 0),
        ("callout_number_too_big2", b"(?C99999)", 0, 0),
        ("missing_callout_closing", b"(?C1", 0, 0),
        ("missing_callout_closing2", b"(?C", 0, 0),
        ("escape_invalid_in_verb", br"(*MARK:a\db)", 0, 0),
        ("escape_invalid_in_verb2", br"(*MARK:\d)", 0, 0),
        ("unrecognized_after_query_p", b"(?Pz)", 0, 0),
        ("unrecognized_after_query_p2", b"(?P)", 0, 0),
        ("missing_name_terminator", b"(?<abc", 0, 0),
        ("missing_name_terminator2", b"(?'abc", 0, 0),
        ("missing_name_terminator3", b"(?P<abc", 0, 0),
        ("dup_subpattern_name", b"(?<a>x)(?<a>y)", 0, 0),
        ("invalid_subpattern_name", b"(?<1a>x)", 0, 0),
        ("invalid_subpattern_name2", b"(?<a-b>x)", 0, 0),
        ("invalid_subpattern_name3", b"(?<>x)", 0, 0),
        // ERR45..ERR55
        ("malformed_unicode_prop", br"\p{", 0, 0),
        ("malformed_unicode_prop2", br"\p", 0, 0),
        ("malformed_unicode_prop3", br"\p{L", 0, 0),
        ("unknown_unicode_prop", br"\p{Zzz}", 0, 0),
        ("unknown_unicode_prop2", br"\p{sc=Nope}", 0, 0),
        ("unknown_unicode_prop3", br"\p{scx:Nope}", 0, 0),
        (
            "subpattern_name_too_long",
            b"(?<aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa>x)",
            0,
            0,
        ),
        ("class_invalid_range", br"[\d-z]", 0, 0),
        ("class_invalid_range2", br"[a-\d]", 0, 0),
        ("class_invalid_range3", br"[\w-\d]", 0, 0),
        ("octal_byte_too_big", br"\400", 0, 0),
        ("octal_byte_too_big2", br"\777", 0, 0),
        ("define_too_many_branches", b"(?(DEFINE)a|b)", 0, 0),
        ("backslash_o_missing_brace", br"\o1", 0, 0),
        ("backslash_o_missing_brace2", br"\o", 0, 0),
        // ERR57..ERR65
        ("backslash_g_syntax", br"\g", 0, 0),
        ("backslash_g_syntax2", br"\g{", 0, 0),
        ("backslash_g_syntax3", br"\gz", 0, 0),
        ("backslash_g_syntax4", br"\g{a", 0, 0),
        ("parens_query_r_missing_closing", b"(?R", 0, 0),
        ("parens_query_r_missing_closing2", b"(?R1", 0, 0),
        ("verb_argument_not_allowed", b"(*ACCEPT:x)", 0, 0),
        ("verb_argument_not_allowed2", b"(*FAIL:x)", 0, 0),
        ("verb_argument_not_allowed3", b"(*COMMIT:x)", 0, 0),
        ("verb_unknown", b"(*ZZZ)", 0, 0),
        ("verb_unknown2", b"(*)", 0, 0),
        ("subpattern_number_too_big", b"(?99999999999)", 0, 0),
        ("subpattern_number_too_big2", br"\g{99999999999}", 0, 0),
        ("subpattern_name_expected", b"(?&)", 0, 0),
        ("subpattern_name_expected2", br"\k<>", 0, 0),
        ("subpattern_name_expected3", b"(?P>)", 0, 0),
        ("invalid_octal", br"\o{9}", 0, 0),
        ("invalid_octal2", br"\o{18}", 0, 0),
        ("subpattern_names_mismatch", b"(?|(?<a>x))(?|(?<b>y))", 0, 0),
        ("mark_missing_argument", b"(*MARK)", 0, 0),
        ("mark_missing_argument2", b"(*:)", 0, 0),
        // ERR67..ERR80
        ("invalid_hexadecimal", br"\x{zz}", 0, 0),
        ("invalid_hexadecimal2", br"\x{1g}", 0, 0),
        ("backslash_c_syntax", b"\\c\x80", 0, 0),
        ("backslash_c_syntax2", b"\\c\xff", 0, 0),
        ("backslash_k_syntax", br"\kx", 0, 0),
        ("backslash_k_syntax2", br"\k", 0, 0),
        ("backslash_k_syntax3", br"\k<a", 0, 0),
        ("backslash_n_in_class", br"[\N]", 0, 0),
        ("backslash_n_in_class2", br"[a\N]", 0, 0),
        ("unicode_disallowed_code_point", br"\x{d800}", PCRE2_UTF, 0),
        ("unicode_disallowed_code_point2", br"\x{dfff}", PCRE2_UTF, 0),
        (
            "verb_name_too_long",
            b"(*MARK:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa)",
            0,
            0,
        ),
        (
            "backslash_u_too_big",
            br"\u{110000}",
            PCRE2_ALT_BSUX,
            PCRE2_EXTRA_ALT_BSUX,
        ),
        ("missing_octal_or_hex_digits", br"\x{}", 0, 0),
        ("missing_octal_or_hex_digits2", br"\o{}", 0, 0),
        ("version_condition_syntax", b"(?(VERSION>=x))", 0, 0),
        ("version_condition_syntax2", b"(?(VERSION))", 0, 0),
        ("version_condition_syntax3", b"(?(VERSION>=10.0a))", 0, 0),
        ("callout_no_string_delimiter", b"(?C{", 0, 0),
        ("callout_bad_string_delimiter", b"(?Ca)", 0, 0),
        ("backslash_c_caller_disabled", br"\C", PCRE2_NEVER_BACKSLASH_C, 0),
        ("pattern_too_complicated", b"(?<=(?:a|bb|ccc){1,255})x", 0, 0),
        // Options interactions
        ("bad_literal_options", b"a", PCRE2_LITERAL | PCRE2_DOTALL, 0),
        ("bad_literal_options2", b"a", PCRE2_LITERAL | PCRE2_EXTENDED, 0),
        (
            "bad_literal_extra_options",
            b"a",
            PCRE2_LITERAL,
            PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
        ),
        ("utf_is_disabled", b"(*UTF)a", PCRE2_NEVER_UTF, 0),
        ("ucp_is_disabled", b"(*UCP)a", PCRE2_NEVER_UCP, 0),
        ("ucp_is_disabled2", br"\p{L}", PCRE2_NEVER_UCP, 0),
        (
            "turkish_requires_utf",
            b"a",
            0,
            PCRE2_EXTRA_TURKISH_CASING,
        ),
        (
            "extra_casing_incompatible",
            b"a",
            PCRE2_UTF,
            PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT,
        ),
        (
            "callout_caller_disabled",
            b"(?C1)a",
            0,
            PCRE2_EXTRA_NEVER_CALLOUT,
        ),
        (
            "oversize_python_octal",
            br"\400",
            0,
            PCRE2_EXTRA_PYTHON_OCTAL,
        ),
        (
            "backslash_k_in_lookaround",
            br"(?<=\Kx)",
            0,
            0,
        ),
        ("expected_capture_group", b"(?<a>)(?(<b>)x)", 0, 0),
        ("missing_opening_parenthesis", b"(?{", 0, 0),
        ("alpha_assertion_unknown", b"(*zzz:x)", 0, 0),
        ("alpha_assertion_unknown2", b"(*pl:x)", 0, 0),
        ("script_run_not_available", b"(*sr:a)", 0, 0),
        ("supported_only_in_unicode", br"\p{Xan}", 0, 0),
        ("invalid_hyphen_in_options", b"(?^-i)", 0, 0),
        ("invalid_hyphen_in_options2", b"(?i-)", 0, 0),
        ("missing_number_terminator", b"a{1", 0, 0),
        ("missing_number_terminator2", b"(?C1x)", 0, 0),
        ("missing_octal_digit", br"\o{", 0, 0),
        // extended classes (ERR107..ERR116 region)
        ("eclass_nest_too_deep", b"[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[a]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("eclass_invalid_operator", b"[a&&&b]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("eclass_unexpected_operator", b"[&&a]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("eclass_expected_operand", b"[[a]&&]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("eclass_mixed_operators", b"[[a]&&[b]||[c]]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("eclass_hint_square_bracket", b"(?[a])", 0, 0),
        ("perl_eclass_unexpected_expr", b"[[:alpha:]&&]", 0, 0),
        ("perl_eclass_empty_expr", b"[()]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("perl_eclass_missing_close", b"[(a", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("perl_eclass_unexpected_char", b"[a b]", PCRE2_ALT_EXTENDED_CLASS, 0),
        // things that should SUCCEED, as controls
        // --- triggers confirmed against the C source for the remaining
        // --- reachable error codes (see ERRORS.md section 2).
        ("no_bs0", br"\0", 0, PCRE2_EXTRA_NO_BS0),
        ("too_many_cond_branches_real", b"(a)(?(1)x|y|z)", 0, 0),
        ("too_many_cond_branches_assert", b"(?(?=a)x|y|z)", 0, 0),
        ("cond_assertion_expected_atomic", b"(?(*atomic:a)x)", 0, 0),
        ("cond_assertion_expected_comment", b"(?(?#c)x)", 0, 0),
        ("subpattern_names_mismatch_real", b"(?|(?<a>x)|(?<b>y))", 0, 0),
        ("missing_number_terminator_g", br"\g{1", 0, 0),
        ("lookbehind_invalid_bsc_utf", br"(?<=\Ca)b", PCRE2_UTF, 0),
        ("turkish_requires_utf_ucp", b"a", PCRE2_UCP, PCRE2_EXTRA_TURKISH_CASING),
        ("eclass_hint_sq_bracket_a", b"[[a]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("eclass_hint_sq_bracket_b", b"[[a][b]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("eclass_hint_sq_bracket_c", b"[[a]&&[b]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("perl_eclass_unexpected_expr_a", b"(?[[a] [b]])", 0, 0),
        ("perl_eclass_unexpected_expr_b", b"(?[[a][b]])", 0, 0),
        ("perl_eclass_unexpected_expr_c", b"(?[[:alpha:][:digit:]])", 0, 0),
        ("perl_eclass_empty_expr_a", b"(?[])", 0, 0),
        ("perl_eclass_empty_expr_b", b"(?[ ])", 0, 0),
        ("perl_eclass_missing_close_a", b"(?[[a]]", 0, 0),
        ("perl_eclass_missing_close_b", b"(?[[a]]x", 0, 0),
        ("perl_eclass_unexpected_char_a", b"(?[[a]&x])", 0, 0),
        ("perl_eclass_unexpected_char_b", b"(?[@])", 0, 0),
        ("eclass_expected_operand_a", b"[a&&]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("eclass_expected_operand_b", b"[[a]&&]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("eclass_expected_operand_c", b"[[a]--]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("escape_invalid_in_verb_altnames", br"(*MARK:a\db)", PCRE2_ALT_VERBNAMES, 0),
        ("escape_invalid_in_verb_altnames2", br"(*MARK:\d)", PCRE2_ALT_VERBNAMES, 0),
        ("missing_opening_paren_scs", b"(*scs:x)", 0, 0),
        ("missing_opening_paren_scs2", b"(*scs:)", 0, 0),
        ("expected_capture_group_scs", b"(*scs:(@))", 0, 0),
        ("expected_capture_group_scs2", b"(*scs:(", 0, 0),
        ("expected_capture_group_scs3", b"(*scs:()a)", 0, 0),
        ("expected_capture_group_rec", b"(a)(?1(@))", 0, 0),
        ("expected_capture_group_rec2", b"(a)(?1(", 0, 0),
        ("ok_scs", b"(a)(*scs:(1)a)", 0, 0),
        ("expected_cap_or_bad_ref", b"(?(<a>)x)", 0, 0),
        ("ok_simple", b"abc", 0, 0),
        ("ok_empty", b"", 0, 0),
        ("ok_class", b"[a-z]", 0, 0),
        ("ok_utf", b"a", PCRE2_UTF, 0),
        ("ok_dupnames", b"(?<a>x)(?<a>y)", PCRE2_DUPNAMES, 0),
        ("ok_empty_class", b"[]", PCRE2_ALLOW_EMPTY_CLASS, 0),
        ("ok_eclass", b"[[a]&&[b]]", PCRE2_ALT_EXTENDED_CLASS, 0),
        ("ok_bsk_lookaround", br"(?<=\Kx)", 0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
        ("ok_surrogate", br"\x{d800}", 0, PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES),
        ("ok_turkish", b"a", PCRE2_UTF, PCRE2_EXTRA_TURKISH_CASING),
    ];

    // Deeply nested groups -> PARENTHESES_NEST_TOO_DEEP (default limit 250).
    v.push(("parens_nest_too_deep", NEST_300.as_bytes(), 0, 0));
    // Many named groups -> TOO_MANY_NAMED_SUBPATTERNS / TOO_MANY_CAPTURES.
    v.push(("too_many_named", MANY_NAMES.as_bytes(), 0, 0));
    v.push(("too_many_captures", MANY_CAPS.as_bytes(), 0, 0));
    // A very long lookbehind -> LOOKBEHIND_TOO_LONG.
    v.push(("lookbehind_too_long", LONG_LB.as_bytes(), 0, 0));
    // A very long callout string -> CALLOUT_STRING_TOO_LONG.
    v.push(("callout_string_too_long", LONG_CALLOUT.as_bytes(), 0, 0));
    // Deep (?| nesting -> QUERY_BARJX_NEST_TOO_DEEP.
    v.push(("query_barjx_nest", DEEP_BARJX.as_bytes(), 0, 0));
    // A lookbehind whose length calculation recurses more than 2000 times ->
    // LOOKBEHIND_TOO_COMPLICATED (pcre2_compile.c:9602).
    v.push(("lookbehind_too_complicated_deep", DEEP_LB.as_bytes(), 0, 0));
    v
}

// Long generated patterns, built once so the `&'static [u8]` above is valid.
use std::sync::LazyLock;
static NEST_300: LazyLock<String> = LazyLock::new(|| {
    let mut s = String::new();
    for _ in 0..300 {
        s.push('(');
    }
    s.push('a');
    for _ in 0..300 {
        s.push(')');
    }
    s
});
static MANY_NAMES: LazyLock<String> = LazyLock::new(|| {
    let mut s = String::new();
    for i in 0..11000 {
        s.push_str(&format!("(?<n{i}>a)"));
    }
    s
});
static MANY_CAPS: LazyLock<String> = LazyLock::new(|| "(a)".repeat(70000));
static LONG_LB: LazyLock<String> = LazyLock::new(|| format!("(?<={})x", "a".repeat(70000)));
static LONG_CALLOUT: LazyLock<String> =
    LazyLock::new(|| format!("(?C{{{}}})a", "x".repeat(70000)));
static DEEP_BARJX: LazyLock<String> = LazyLock::new(|| {
    // 800 nested `(?|(` groups overflows the nest_save array
    // (pcre2_compile.c:4856) -> QUERY_BARJX_NEST_TOO_DEEP.
    let mut s = String::new();
    for _ in 0..800 {
        s.push_str("(?|(");
    }
    s.push('a');
    for _ in 0..800 {
        s.push_str("))");
    }
    s
});
static DEEP_LB: LazyLock<String> =
    LazyLock::new(|| format!("(?<={})x", "(?:a)".repeat(3000)));

/// Runs one case and returns the C-observed error code, after asserting that
/// C and Rust agree on everything observable.
fn run_case(label: &str, pat: &[u8], opts: u32, extra: u32) -> c_int {
    let mut codes: Vec<c_int> = Vec::new();
    for api in [&apis().0, &apis().1] {
        unsafe {
            let cc = (api.compile_context_create)(std::ptr::null_mut());
            (api.set_compile_extra_options)(cc, extra);
            // Deliberately generous structural limits so that a case reaches
            // the error it was designed for rather than a nesting limit.
            if label.starts_with("query_barjx") || label.starts_with("lookbehind_too_complicated") {
                (api.set_parens_nest_limit)(cc, 100_000);
                (api.set_max_varlookbehind)(cc, u32::MAX);
            }
            let mut ec: c_int = 0x7FFF;
            let mut eo: Sz = 0xDEAD;
            let p = if pat.is_empty() {
                b"".as_ptr()
            } else {
                pat.as_ptr()
            };
            let code = (api.compile)(p, pat.len(), opts, &mut ec, &mut eo, cc);
            codes.push(ec);
            // Also compare the error MESSAGE text for the code.
            let mut buf = [0u8; 512];
            let mrc = (api.get_error_message)(ec, buf.as_mut_ptr(), buf.len());
            let mut l = Log::new();
            l.i(code.is_null() as i64)
                .i(ec as i64)
                .u(eo as u64)
                .i(mrc as i64)
                .b(&buf[..(mrc.max(0) as usize).min(buf.len())]);
            if !code.is_null() {
                log_all_info(api, code, &mut l);
                (api.code_free)(code);
            }
            (api.compile_context_free)(cc);
            LOGS.with(|c| c.borrow_mut().push(l));
        }
    }
    let (lc, lr) = LOGS.with(|c| {
        let mut b = c.borrow_mut();
        let r = b.pop().unwrap();
        let c0 = b.pop().unwrap();
        (c0, r)
    });
    assert!(
        lc == lr,
        "DIVERGENCE in compile-error case {label:?} (pat={:?} opts={opts:#x} extra={extra:#x})\n\
         C   = {lc:?}\nRUST= {lr:?}",
        String::from_utf8_lossy(pat)
    );
    codes[0]
}

thread_local! {
    static LOGS: std::cell::RefCell<Vec<Log>> = std::cell::RefCell::new(Vec::new());
}

#[test]
fn every_compile_error_case_agrees() {
    let mut seen: std::collections::BTreeSet<c_int> = std::collections::BTreeSet::new();
    for (label, pat, opts, extra) in cases() {
        let ec = run_case(label, pat, opts, extra);
        seen.insert(ec);
    }
    // ERR0 (== 100) means success; everything else is a distinct rejection.
    let errors: Vec<c_int> = seen.iter().copied().filter(|&c| c != 100).collect();
    eprintln!(
        "compile error codes reached ({}): {:?}",
        errors.len(),
        errors
    );
    assert!(
        seen.contains(&100),
        "no case compiled successfully — the control cases are broken"
    );
    assert!(
        errors.len() >= 93,
        "only {} distinct compile error codes reached; expected >= 93",
        errors.len()
    );
}

/// ERRORS.md rows 1-3: the `errorptr` / `erroroffset` argument contract.
#[test]
fn compile_argument_pointer_contract() {
    diff("compile_ptr_contract", |api| {
        let mut l = Log::new();
        unsafe {
            // errorptr == NULL, erroroffset != NULL -> NULL, *erroroffset = 0
            let mut eo: Sz = 0xDEAD;
            let code = (api.compile)(
                b"abc".as_ptr(),
                3,
                0,
                std::ptr::null_mut(),
                &mut eo,
                std::ptr::null_mut(),
            );
            l.i(code.is_null() as i64).u(eo as u64);

            // both NULL -> NULL, no writes
            let code = (api.compile)(
                b"abc".as_ptr(),
                3,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            l.i(code.is_null() as i64);

            // erroroffset == NULL -> NULL_ERROROFFSET (220)
            let mut ec: c_int = 0x7FFF;
            let code = (api.compile)(
                b"abc".as_ptr(),
                3,
                0,
                &mut ec,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            l.i(code.is_null() as i64).i(ec as i64);

            // NULL pattern, various lengths
            for len in [0usize, 1, 2, PCRE2_ZERO_TERMINATED] {
                let mut ec: c_int = 0x7FFF;
                let mut eo: Sz = 0xDEAD;
                let code = (api.compile)(
                    std::ptr::null(),
                    len,
                    0,
                    &mut ec,
                    &mut eo,
                    std::ptr::null_mut(),
                );
                l.i(code.is_null() as i64).i(ec as i64).u(eo as u64);
                if !code.is_null() {
                    log_all_info(api, code, &mut l);
                    (api.code_free)(code);
                }
            }
        }
        l
    });
}

/// ERRORS.md rows 6/7: every single undefined option bit, and every single
/// undefined extra-option bit, passed across the FFI boundary.
#[test]
fn compile_every_undefined_option_bit() {
    const PUBLIC_COMPILE_OPTIONS: u32 = 0xE800_0000 | 0x0FFF_FFFF;
    for b in 0..32u32 {
        let bit = 1u32 << b;
        diff(&format!("undef_opt_bit {bit:#x}"), |api| {
            let mut l = Log::new();
            unsafe {
                let mut ec: c_int = 0x7FFF;
                let mut eo: Sz = 0xDEAD;
                let code = (api.compile)(
                    b"abc".as_ptr(),
                    3,
                    bit,
                    &mut ec,
                    &mut eo,
                    std::ptr::null_mut(),
                );
                l.i(code.is_null() as i64).i(ec as i64).u(eo as u64);
                if !code.is_null() {
                    log_all_info(api, code, &mut l);
                    (api.code_free)(code);
                }
                // and the same bit as an EXTRA option
                let cc = (api.compile_context_create)(std::ptr::null_mut());
                l.i((api.set_compile_extra_options)(cc, bit) as i64);
                let mut ec2: c_int = 0x7FFF;
                let mut eo2: Sz = 0xDEAD;
                let code2 = (api.compile)(b"abc".as_ptr(), 3, 0, &mut ec2, &mut eo2, cc);
                l.i(code2.is_null() as i64).i(ec2 as i64).u(eo2 as u64);
                if !code2.is_null() {
                    log_all_info(api, code2, &mut l);
                    (api.code_free)(code2);
                }
                (api.compile_context_free)(cc);
            }
            l
        });
        let _ = PUBLIC_COMPILE_OPTIONS;
    }
    // All-bits-set, and every 2-bit combination of the top control bits.
    for opts in [
        0xFFFF_FFFFu32,
        0x8000_0000,
        0x4000_0000,
        0x2000_0000,
        0x1000_0000,
        0x1000_0000 | 0x8000_0000,
    ] {
        diff(&format!("undef_opt {opts:#x}"), |api| {
            let mut l = Log::new();
            unsafe {
                let mut ec: c_int = 0x7FFF;
                let mut eo: Sz = 0xDEAD;
                let code = (api.compile)(
                    b"abc".as_ptr(),
                    3,
                    opts,
                    &mut ec,
                    &mut eo,
                    std::ptr::null_mut(),
                );
                l.i(code.is_null() as i64).i(ec as i64).u(eo as u64);
                if !code.is_null() {
                    log_all_info(api, code, &mut l);
                    (api.code_free)(code);
                }
            }
            l
        });
    }
}

/// The error MESSAGE for every code in the whole documented range must match,
/// which also proves the error-code tables are identical.
#[test]
fn error_messages_for_all_codes() {
    diff("error_messages", |api| {
        let mut l = Log::new();
        unsafe {
            for ec in -90i32..=230 {
                let mut buf = [0u8; 512];
                let rc = (api.get_error_message)(ec, buf.as_mut_ptr(), buf.len());
                l.i(ec as i64).i(rc as i64);
                if rc > 0 {
                    l.b(&buf[..rc as usize]);
                }
            }
            // buffer-size boundaries for a few codes
            for ec in [1i32, -1, 100, 101, 220, -51, -76] {
                for size in 0..40usize {
                    let mut buf = [0xAAu8; 64];
                    let rc = (api.get_error_message)(ec, buf.as_mut_ptr(), size);
                    l.i(ec as i64).u(size as u64).i(rc as i64).b(&buf);
                }
            }
        }
        l
    });
}

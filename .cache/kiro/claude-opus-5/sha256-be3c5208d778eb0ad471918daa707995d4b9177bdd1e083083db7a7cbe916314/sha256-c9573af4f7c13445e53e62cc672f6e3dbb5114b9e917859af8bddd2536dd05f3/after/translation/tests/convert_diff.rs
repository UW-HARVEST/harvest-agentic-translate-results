//! Phase B — `pcre2_pattern_convert` valid-path differential coverage
//! (glob and POSIX BRE/ERE conversion), across every convert-context setting.

mod common;
use common::*;

fn conv_cmp(
    p: &Pair,
    pat: &[u8],
    plen: Sz,
    opts: u32,
    cvc: Ctx,
    cvr: Ctx,
    label: &str,
) {
    // PCRE2_ZERO_TERMINATED makes the library call strlen(), so keep a real NUL.
    let owned;
    let sp: *const u8 = if plen == PCRE2_ZERO_TERMINATED {
        owned = {
            let mut v = pat.to_vec();
            v.push(0);
            v
        };
        owned.as_ptr()
    } else if pat.is_empty() {
        std::ptr::null()
    } else {
        pat.as_ptr()
    };
    let mut bc: *mut u8 = std::ptr::null_mut();
    let mut br: *mut u8 = std::ptr::null_mut();
    let mut lc: Sz = 0xAAAA;
    let mut lr: Sz = 0xAAAA;
    unsafe {
        let a = (p.c.pattern_convert)(sp, plen, opts, &mut bc, &mut lc, cvc);
        let b = (p.r.pattern_convert)(sp, plen, opts, &mut br, &mut lr, cvr);
        assert_eq!(a, b, "pattern_convert rc [{}]", label);
        assert_eq!(lc, lr, "pattern_convert length [{}]", label);
        if a == 0 {
            let sc = std::slice::from_raw_parts(bc, lc);
            let sr = std::slice::from_raw_parts(br, lr);
            assert_eq!(sc, sr, "pattern_convert output [{}]", label);
            // The produced pattern must also compile identically in both.
            if let Ok(cp) = compile_both(p, sc, lc, 0, std::ptr::null_mut(), std::ptr::null_mut(), label) {
                cmp_all_pattern_info(p, &cp, label);
                cmp_compiled_bytes(p, &cp, label);
                free_code_pair(p, cp);
            }
            (p.c.converted_pattern_free)(bc);
            (p.r.converted_pattern_free)(br);
        }
        // The "length only" form (buffer pointer supplied but PCRE2_SIZE query) is
        // not part of this API; instead check the NULL-length-pointer form.
        let a = (p.c.pattern_convert)(sp, plen, opts, &mut bc, std::ptr::null_mut(), cvc);
        let b = (p.r.pattern_convert)(sp, plen, opts, &mut br, std::ptr::null_mut(), cvr);
        assert_eq!(a, b, "pattern_convert(NULL len) rc [{}]", label);
        if a == 0 {
            (p.c.converted_pattern_free)(bc);
            (p.r.converted_pattern_free)(br);
        }
    }
}

static GLOBS: &[&[u8]] = &[
    b"",
    b"a",
    b"*",
    b"?",
    b"*.txt",
    b"a/b/*.c",
    b"**",
    b"a/**/b",
    b"**/b",
    b"a/**",
    b"[abc]",
    b"[!abc]",
    b"[a-z]",
    b"[]]",
    b"[!]]",
    b"[a-]",
    b"[[:alpha:]]",
    b"\\*",
    b"\\?",
    b"\\[",
    b"\\\\",
    b"a\\-b",
    b"/*/*",
    b"/**/",
    b".*",
    b"a.b",
    b"^a$",
    b"a+b",
    b"a{1}",
    b"(a)",
    b"|",
    b"\xC3\xA9*",
    b"\xE2\x82\xAC?",
    b"a\x00b",
    b"[/]",
    b"*/",
    b"/*",
    b"a**b",
    b"?*",
    b"[^a]",
];

static POSIX: &[&[u8]] = &[
    b"",
    b"a",
    b"a*",
    b"a\\*",
    b"^a$",
    b"a.b",
    b"[abc]",
    b"[^abc]",
    b"[]a]",
    b"[a-z]",
    b"[[:digit:]]",
    b"a\\(b\\)c",
    b"a(b)c",
    b"a\\{1,2\\}",
    b"a{1,2}",
    b"a\\|b",
    b"a|b",
    b"a\\+",
    b"a+",
    b"a\\?",
    b"a?",
    b"\\(a\\)\\1",
    b"a\\{2\\}",
    b"$a^",
    b"\\.",
    b"\\\\",
    b"[[.a.]]",
    b"[[=a=]]",
    b"a\nb",
    b"\xC3\xA9",
];

#[test]
fn convert_glob_matrix() {
    let p = libs();
    let cvc = unsafe { (p.c.convert_context_create)(std::ptr::null_mut()) };
    let cvr = unsafe { (p.r.convert_context_create)(std::ptr::null_mut()) };
    assert!(!cvc.is_null() && !cvr.is_null());
    let base = [
        o::CONVERT_GLOB,
        o::CONVERT_GLOB_NO_WILD_SEPARATOR,
        o::CONVERT_GLOB_NO_STARSTAR,
    ];
    for &b in &base {
        for extra in [0u32, o::CONVERT_UTF, o::CONVERT_UTF | o::CONVERT_NO_UTF_CHECK] {
            for pat in GLOBS {
                for plen in [pat.len(), PCRE2_ZERO_TERMINATED] {
                    if plen == PCRE2_ZERO_TERMINATED && pat.contains(&0) {
                        continue;
                    }
                    let label = format!(
                        "glob|{:#x}|{:?}|len={}",
                        b | extra,
                        String::from_utf8_lossy(pat),
                        plen as i64
                    );
                    conv_cmp(p, pat, plen, b | extra, cvc, cvr, &label);
                }
            }
        }
    }
    unsafe {
        (p.c.convert_context_free)(cvc);
        (p.r.convert_context_free)(cvr);
    }
}

#[test]
fn convert_glob_with_custom_escape_and_separator() {
    let p = libs();
    // Every value pcre2_set_glob_escape / _separator accepts, plus rejected ones.
    for esc in [0u32, b'\\' as u32, b'!' as u32, b'^' as u32, b'/' as u32, 0x100] {
        for sep in [b'/' as u32, b'.' as u32, b'\\' as u32, b':' as u32, 0u32, 0x100] {
            let cvc = unsafe { (p.c.convert_context_create)(std::ptr::null_mut()) };
            let cvr = unsafe { (p.r.convert_context_create)(std::ptr::null_mut()) };
            let a = unsafe { (p.c.set_glob_escape)(cvc, esc) };
            let b = unsafe { (p.r.set_glob_escape)(cvr, esc) };
            assert_eq!(a, b, "set_glob_escape({:#x})", esc);
            let a2 = unsafe { (p.c.set_glob_separator)(cvc, sep) };
            let b2 = unsafe { (p.r.set_glob_separator)(cvr, sep) };
            assert_eq!(a2, b2, "set_glob_separator({:#x})", sep);
            for pat in GLOBS {
                for opts in [
                    o::CONVERT_GLOB,
                    o::CONVERT_GLOB_NO_WILD_SEPARATOR,
                    o::CONVERT_GLOB_NO_STARSTAR,
                ] {
                    let label = format!(
                        "glob|esc={:#x}|sep={:#x}|{:#x}|{:?}",
                        esc,
                        sep,
                        opts,
                        String::from_utf8_lossy(pat)
                    );
                    conv_cmp(p, pat, pat.len(), opts, cvc, cvr, &label);
                }
            }
            // A copied convert context must behave identically too.
            let cvc2 = unsafe { (p.c.convert_context_copy)(cvc) };
            let cvr2 = unsafe { (p.r.convert_context_copy)(cvr) };
            assert!(!cvc2.is_null() && !cvr2.is_null());
            for pat in GLOBS.iter().take(12) {
                conv_cmp(
                    p,
                    pat,
                    pat.len(),
                    o::CONVERT_GLOB,
                    cvc2,
                    cvr2,
                    &format!("globcopy|{:?}", String::from_utf8_lossy(pat)),
                );
            }
            unsafe {
                (p.c.convert_context_free)(cvc2);
                (p.r.convert_context_free)(cvr2);
                (p.c.convert_context_free)(cvc);
                (p.r.convert_context_free)(cvr);
            }
        }
    }
}

#[test]
fn convert_posix_matrix() {
    let p = libs();
    for base in [o::CONVERT_POSIX_BASIC, o::CONVERT_POSIX_EXTENDED] {
        for extra in [0u32, o::CONVERT_UTF, o::CONVERT_UTF | o::CONVERT_NO_UTF_CHECK] {
            for pat in POSIX {
                for plen in [pat.len(), PCRE2_ZERO_TERMINATED] {
                    if plen == PCRE2_ZERO_TERMINATED && pat.contains(&0) {
                        continue;
                    }
                    let label = format!(
                        "posix|{:#x}|{:?}|len={}",
                        base | extra,
                        String::from_utf8_lossy(pat),
                        plen as i64
                    );
                    conv_cmp(p, pat, plen, base | extra, std::ptr::null_mut(), std::ptr::null_mut(), &label);
                }
            }
        }
    }
}

#[test]
fn convert_randomized() {
    let p = libs();
    let mut rng = Rng::new(0x0C07_0000_0001);
    let alphabet: &[u8] = b"ab*?[]!^-\\/.{}()|+$:=,0123456789 \t\xC3\xA9\xE2\x82\xAC\x80\xff";
    let optsets: &[u32] = &[
        o::CONVERT_GLOB,
        o::CONVERT_GLOB_NO_WILD_SEPARATOR,
        o::CONVERT_GLOB_NO_STARSTAR,
        o::CONVERT_POSIX_BASIC,
        o::CONVERT_POSIX_EXTENDED,
        o::CONVERT_GLOB | o::CONVERT_UTF,
        o::CONVERT_POSIX_BASIC | o::CONVERT_UTF,
        o::CONVERT_POSIX_EXTENDED | o::CONVERT_UTF,
    ];
    for _ in 0..40000 {
        let n = rng.below(12);
        let pat: Vec<u8> = (0..n).map(|_| *rng.pick(alphabet)).collect();
        let opts = *rng.pick(optsets);
        let label = format!("rndconv|{:#x}|{:02x?}", opts, pat);
        conv_cmp(p, &pat, pat.len(), opts, std::ptr::null_mut(), std::ptr::null_mut(), &label);
    }
}

#[test]
fn convert_long_inputs() {
    let p = libs();
    for n in [100usize, 1000, 5000] {
        for unit in [&b"a"[..], &b"*"[..], &b"a/"[..], &b"[a]"[..], &b"\\*"[..], &b"**/"[..]] {
            let mut pat = Vec::new();
            while pat.len() < n {
                pat.extend_from_slice(unit);
            }
            pat.truncate(n);
            for opts in [
                o::CONVERT_GLOB,
                o::CONVERT_GLOB_NO_STARSTAR,
                o::CONVERT_POSIX_BASIC,
                o::CONVERT_POSIX_EXTENDED,
            ] {
                let label = format!("longconv|{:#x}|n={}|u={:?}", opts, n, String::from_utf8_lossy(unit));
                conv_cmp(p, &pat, pat.len(), opts, std::ptr::null_mut(), std::ptr::null_mut(), &label);
            }
        }
    }
}

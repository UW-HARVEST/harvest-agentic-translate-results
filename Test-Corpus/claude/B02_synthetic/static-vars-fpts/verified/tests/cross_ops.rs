//! Phase B — `CONFIGS.md` row C26: the analyzer of one library driven through
//! the `tokenizer_ops_t` of the *other* library.
//!
//! This is what the whole C program is built around (analyzer.c only ever
//! touches the tokenizer through the function pointers installed by
//! `analyzer_init`), so the Rust translation must really dispatch through them
//! instead of reaching into its own tokenizer.
//!
//! Lives in its own test binary because it deliberately drives one library's
//! tokenizer twice as often as the other's, which desynchronises the
//! never-resettable `total_*_processed` counters.

mod common;

use common::*;
use std::ffi::c_char;

/// Everything of `analysis_result_t` except the two fields that are copied from
/// the *cumulative* tokenizer statistics.
fn per_call_fields(r: &CResult) -> [usize; 6] {
    [
        r.word_count,
        r.number_count,
        r.keyword_count,
        r.operator_count,
        r.comment_count,
        r.string_count,
    ]
}

fn texts() -> Vec<Vec<u8>> {
    let mut rng = Rng::new(0xC26);
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"int a = 1;".to_vec(),
        b"if (x >= 2) { return \"s\"; } // c\n".to_vec(),
        b"/* multi\nline */ a b c 1.5 ++ --\n".to_vec(),
        b"one\ntwo\nthree\n".to_vec(),
    ];
    for _ in 0..40 {
        v.push(random_source(&mut rng, 30));
    }
    for _ in 0..20 {
        v.push(random_soup(&mut rng, 200));
    }
    v
}

/// `ops` is installed into both analyzers, so both drive the *same* tokenizer.
fn cross_check(ops: COps, which: &str) {
    let p = libs();
    (p.c.analyzer_init)(ops);
    (p.rust.analyzer_init)(ops);

    // the tokenizer whose statistics both analyzers observe
    let stats = |lib: &Api| {
        let mut l = 0usize;
        let mut t = 0usize;
        let mut c = 0usize;
        (ops.get_stats.unwrap())(&mut l, &mut t, &mut c);
        let _ = lib;
        (l, t, c)
    };

    for text in texts() {
        let s0 = stats(&p.c);
        let rc = p.c.analyze(&text);
        let s1 = stats(&p.c);
        let rr = p.rust.analyze(&text);
        let s2 = stats(&p.c);

        assert_eq!(
            per_call_fields(&rc),
            per_call_fields(&rr),
            "[{}] per-call analysis fields differ for {}",
            which,
            show(&text)
        );

        // both took line_count/char_count straight from ops.get_stats ...
        assert_eq!(
            (rc.line_count, rc.char_count),
            (s1.0, s1.2),
            "[{}] C analyze_text did not report the shared stats",
            which
        );
        assert_eq!(
            (rr.line_count, rr.char_count),
            (s2.0, s2.2),
            "[{}] Rust analyze_text did not report the shared stats",
            which
        );
        // ... and both did exactly the same amount of tokenizing
        assert_eq!(
            (s1.0 - s0.0, s1.1 - s0.1, s1.2 - s0.2),
            (s2.0 - s1.0, s2.1 - s1.1, s2.2 - s1.2),
            "[{}] the two analyzers consumed different amounts for {}",
            which,
            show(&text)
        );

        // find_patterns always resets the shared tokenizer first, so its output
        // is directly comparable.
        for pat in [&b""[..], &b"a"[..], &b"1"[..], &b"//"[..]] {
            let s = cstring(pat);
            let c = p.c.captured(|| (p.c.find_patterns)(s.as_ptr() as *const c_char));
            let r = p
                .rust
                .captured(|| (p.rust.find_patterns)(s.as_ptr() as *const c_char));
            assert_eq!(
                show(&c),
                show(&r),
                "[{}] find_patterns differs for pattern {} on text {}",
                which,
                show(pat),
                show(&text)
            );
        }

        let c = p.c.captured(|| (p.c.print_token_distribution)());
        let r = p.rust.captured(|| (p.rust.print_token_distribution)());
        assert_eq!(
            show(&c),
            show(&r),
            "[{}] print_token_distribution differs after {}",
            which,
            show(&text)
        );

        assert_eq!(
            (p.c.calculate_complexity_score)(),
            (p.rust.calculate_complexity_score)(),
            "[{}] complexity score differs after {}",
            which,
            show(&text)
        );
    }
}

#[test]
fn c26a_both_analyzers_driven_by_the_c_tokenizer() {
    let _g = lock();
    let p = libs();
    let ops = (p.c.get_tokenizer_ops)();
    cross_check(ops, "C ops");
}

#[test]
fn c26b_both_analyzers_driven_by_the_rust_tokenizer() {
    let _g = lock();
    let p = libs();
    let ops = (p.rust.get_tokenizer_ops)();
    cross_check(ops, "Rust ops");
}

#[test]
fn c26c_interactive_tokenizer_accepts_foreign_ops() {
    let _g = lock();
    let p = libs();
    // stdin is at EOF in the test process for this scenario, so the fgets loop
    // stops immediately and an empty text is tokenized; what matters here is
    // that both libraries dispatch load_text/next_token through the given ops.
    let devnull = std::fs::File::open("/dev/null").expect("/dev/null");
    let saved = dup_stdin(&devnull);

    let c_ops = (p.c.get_tokenizer_ops)();
    let r_ops = (p.rust.get_tokenizer_ops)();

    let a = p.c.captured(|| (p.c.interactive_tokenizer)(r_ops));
    let b = p.rust.captured(|| (p.rust.interactive_tokenizer)(c_ops));
    assert_eq!(show(&a), show(&b), "interactive_tokenizer with foreign ops");

    restore_stdin(saved);
}

// -- tiny fd helpers --------------------------------------------------------

extern "C" {
    fn dup(fd: std::ffi::c_int) -> std::ffi::c_int;
    fn dup2(old: std::ffi::c_int, new: std::ffi::c_int) -> std::ffi::c_int;
    fn close(fd: std::ffi::c_int) -> std::ffi::c_int;
}

fn dup_stdin(f: &std::fs::File) -> std::ffi::c_int {
    use std::os::unix::io::AsRawFd;
    let saved = unsafe { dup(0) };
    assert!(unsafe { dup2(f.as_raw_fd(), 0) } >= 0);
    saved
}

fn restore_stdin(saved: std::ffi::c_int) {
    unsafe {
        dup2(saved, 0);
        close(saved);
    }
}

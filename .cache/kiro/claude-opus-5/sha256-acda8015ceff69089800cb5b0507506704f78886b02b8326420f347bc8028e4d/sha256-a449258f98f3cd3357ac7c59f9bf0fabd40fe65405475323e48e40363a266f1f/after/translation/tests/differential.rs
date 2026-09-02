// Differential tests: the C program in `c_src/` is the ground truth, and the
// Rust program in `translation/` must produce byte-identical stdout, stderr and
// the same exit status for the same stdin.
//
// Both are executed as subprocesses. See `common/mod.rs` for the harness.
//
// Input classes enumerated from `c_src/src/main.c`:
//
//   main()   char in[1000] = "";        -> whole buffer zero filled
//            fread(in, 1, 1000, stdin); -> up to 1000 bytes, newlines are NOT
//                                          delimiters (unlike fgets/scanf)
//     * EOF immediately            -> 0 bytes read, buffer stays empty
//     * 1..999 bytes               -> zero fill supplies the terminator
//     * exactly 1000 bytes         -> buffer completely filled, no terminator
//     * more than 1000 bytes       -> tail silently discarded
//     * input containing a NUL     -> the C string ends at that NUL
//
//   foo(in, c)  for (s = in; s = strchr(s, c); s++) res++;
//     * no match at all            -> strchr returns NULL on the first pass
//     * match at index 0           -> loop entered immediately
//     * match at the final byte    -> s++ lands on the NUL, next strchr NULLs
//     * adjacent / repeated matches
//     * bytes >= 0x80 present      -> plain `char` is signed on x86-64, but the
//                                     searched-for 'A'/'x' are positive
//
//   driver()  printf("A: %d\n") then printf("x: %d\n") -- fixed order/format.
//
// There is no error path in this program: it never writes to stderr and always
// returns 0. The tests assert that too, since an "exit 1 vs exit 0" difference
// is exactly what a stdout-only check would miss.

mod common;

use common::{
    assert_same, assert_same_auto, assert_same_chunked, assert_same_modal, assert_same_with_args,
    Rng,
};

const BUF: usize = 1000; // sizeof(in) in main()

// ---------------------------------------------------------------------------
// Empty / minimal inputs
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    // fread returns 0; `in` is still the zero-filled "" -> A: 0 / x: 0.
    assert_same("empty", b"");
}

#[test]
fn single_byte_inputs() {
    assert_same("single-A", b"A");
    assert_same("single-x", b"x");
    assert_same("single-newline", b"\n");
    assert_same("single-space", b" ");
    assert_same("single-nul", b"\0");
    assert_same("single-0xff", b"\xff");
}

#[test]
fn no_match_at_all() {
    // strchr returns NULL on the very first evaluation for both characters.
    assert_same("no-match", b"bcdefg");
    assert_same("lowercase-a-uppercase-X", b"aX aX aX");
}

// ---------------------------------------------------------------------------
// Match position / multiplicity inside foo()
// ---------------------------------------------------------------------------

#[test]
fn match_positions() {
    assert_same("match-first-byte", b"Azzz");
    assert_same("match-last-byte", b"zzzA");
    assert_same("match-only-byte", b"A");
    assert_same("x-first-byte", b"xzzz");
    assert_same("x-last-byte", b"zzzx");
    assert_same("both-at-ends", b"A......x");
    assert_same("both-reversed", b"x......A");
}

#[test]
fn adjacent_and_repeated_matches() {
    assert_same("AA", b"AA");
    assert_same("xx", b"xx");
    assert_same("AAAA-xxxx", b"AAAAxxxx");
    assert_same("alternating", b"AxAxAxAxAx");
    assert_same("ten-A", b"AAAAAAAAAA");
}

#[test]
fn case_sensitivity() {
    // 'a' and 'X' must not be counted.
    assert_same("mixed-case", b"aAxXaAxX");
}

// ---------------------------------------------------------------------------
// fread does not stop at newlines (the fgets/scanf distinction)
// ---------------------------------------------------------------------------

#[test]
fn reads_across_newlines() {
    assert_same("two-lines", b"A\nx\n");
    assert_same("many-lines", b"A\nx\nAAxx\n");
    assert_same("crlf", b"A\r\nx\r\n");
    assert_same("blank-lines-then-match", b"\n\n\n\nAx\n\n");
    assert_same("trailing-newline-only", b"Ax\n");
    assert_same("no-trailing-newline", b"Ax");
    assert_same("whitespace-separated", b"A \t x \t A\nx");
}

// ---------------------------------------------------------------------------
// NUL bytes: the C string ends at the first NUL, the rest is ignored
// ---------------------------------------------------------------------------

#[test]
fn nul_terminates_the_string() {
    assert_same("leading-nul", b"\0Ax");
    assert_same("nul-in-middle", b"A\0x");
    assert_same("nul-after-both", b"Ax\0AAAAxxxx");
    assert_same("many-nuls", b"A\0\0\0x\0\0A");
    assert_same("nul-then-nothing", b"Ax\0");
}

#[test]
fn nul_at_buffer_boundary() {
    let mut v = vec![b'A'; BUF - 1];
    v.push(0);
    v.extend(std::iter::repeat(b'A').take(50));
    assert_same("999A-nul-then-more", &v);

    let mut v = vec![b'x'; BUF];
    v[BUF - 1] = 0;
    assert_same("999x-then-nul-fills-buffer", &v);
}

// ---------------------------------------------------------------------------
// Buffer-size boundaries: 999 / 1000 / 1001 / far beyond
// ---------------------------------------------------------------------------

#[test]
fn length_boundaries_all_a() {
    // Deterministic in C: a NUL from the zero fill always terminates `in`.
    for n in [BUF - 2, BUF - 1] {
        assert_same(&format!("{n}xA"), &vec![b'A'; n]);
    }
    // Buffer completely filled -> C reads past the end (see ERRORS.md).
    for n in [BUF, BUF + 1, BUF + 2, 2 * BUF, 8 * BUF] {
        assert_same_modal(&format!("{n}xA"), &vec![b'A'; n]);
    }
}

#[test]
fn length_boundaries_all_x() {
    for n in [BUF - 2, BUF - 1] {
        assert_same(&format!("{n}xX"), &vec![b'x'; n]);
    }
    for n in [BUF, BUF + 1, BUF + 2, 2 * BUF] {
        assert_same_modal(&format!("{n}xX"), &vec![b'x'; n]);
    }
}

#[test]
fn truncation_discards_the_tail() {
    // Matches beyond byte 1000 must not be counted.
    let mut v = vec![b'.'; BUF];
    v.extend(std::iter::repeat(b'A').take(500));
    v.extend(std::iter::repeat(b'x').take(500));
    assert_same_modal("matches-only-past-1000", &v);

    // The 1000th byte is the last one read; the 1001st is not.
    let mut v = vec![b'.'; BUF - 1];
    v.push(b'A');
    v.push(b'x');
    assert_same_modal("A-at-1000-x-at-1001", &v);

    let mut v = vec![b'.'; BUF - 1];
    v.push(b'x');
    v.push(b'A');
    assert_same_modal("x-at-1000-A-at-1001", &v);
}

#[test]
fn exactly_full_buffer_mixed() {
    // Buffer completely filled, so the C string is not NUL terminated inside
    // `in` and strchr walks into the 8 uninitialised padding bytes above it and
    // then the saved frame pointer. Recorded in ERRORS.md; compared against the
    // C program's modal (stable) output.
    let mut v: Vec<u8> = Vec::with_capacity(BUF);
    while v.len() < BUF {
        v.extend_from_slice(b"Ax.");
    }
    v.truncate(BUF);
    assert_same_modal("full-buffer-mixed", &v);

    let mut v = vec![b'.'; BUF];
    v[0] = b'A';
    v[BUF - 1] = b'x';
    assert_same_modal("full-buffer-ends-with-x", &v);

    let mut v = vec![b'.'; BUF];
    v[BUF - 1] = b'A';
    assert_same_modal("full-buffer-ends-with-A", &v);
}

#[test]
fn full_buffer_with_a_nul_inside_is_deterministic() {
    // 1000 bytes read, but a NUL inside the buffer stops strchr before the end,
    // so there is no out-of-bounds read and the comparison is exact.
    let mut v = vec![b'A'; BUF];
    v[BUF - 1] = 0;
    assert_same("full-buffer-last-byte-nul", &v);

    let mut v = vec![b'x'; BUF];
    v[BUF / 2] = 0;
    assert_same("full-buffer-nul-midway", &v);

    // Over-long input whose first 1000 bytes contain a NUL: also exact.
    let mut v = vec![b'A'; 3 * BUF];
    v[500] = 0;
    assert_same("3000-bytes-nul-at-500", &v);
}

#[test]
fn halves_of_the_buffer() {
    let mut v = vec![b'x'; BUF / 2];
    v.extend(std::iter::repeat(b'A').take(BUF / 2));
    assert_same("500x-then-500A", &v);
}

// ---------------------------------------------------------------------------
// Non-ASCII / signed-char territory
// ---------------------------------------------------------------------------

#[test]
fn high_bit_bytes() {
    assert_same("high-bytes-around-matches", b"\xff\xfeA\x80x\x7f");
    assert_same("utf8-text", "héllo À xylophone ✓".as_bytes());

    // Every byte value except NUL, once.
    let all: Vec<u8> = (1u8..=255).collect();
    assert_same("all-nonzero-bytes", &all);

    // Every byte value including NUL: truncates at the first byte.
    let mut all0: Vec<u8> = vec![0];
    all0.extend(1u8..=255);
    assert_same("nul-then-all-bytes", &all0);
}

// ---------------------------------------------------------------------------
// Invocation shape: arguments are ignored, stdin may be unreadable
// ---------------------------------------------------------------------------

#[test]
fn arguments_are_ignored() {
    // main() takes no arguments, so argv must make no difference.
    assert_same_with_args("args-empty-stdin", &["foo", "bar"], b"");
    assert_same_with_args("args-with-stdin", &["-h", "--help", "1000"], b"AAx");
}

// ---------------------------------------------------------------------------
// Failing I/O: the paths where fread or printf cannot do their job
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn stdout_reader_gone() {
    // printf's write fails with EPIPE. The C program inherits the default
    // SIGPIPE disposition and is killed by the signal; the Rust program must do
    // the same rather than exiting 0 (see ERRORS.md).
    common::assert_same_dead_stdout_reader("dead-stdout-reader-empty", b"");
    common::assert_same_dead_stdout_reader("dead-stdout-reader-matches", b"AAAxxx\n");
}

#[cfg(unix)]
#[test]
fn stdout_closed() {
    // printf fails with EBADF; no signal is raised and main still returns 0.
    common::assert_same_closed_stdout("closed-stdout-empty", b"");
    common::assert_same_closed_stdout("closed-stdout-matches", b"Ax");
}

#[cfg(unix)]
#[test]
fn stdin_not_readable() {
    // fread on a directory fails immediately: `in` keeps its zero fill, so the
    // program still prints A: 0 / x: 0 and returns 0.
    common::assert_same_unreadable_stdin("stdin-is-a-directory", &common::repo_root());
}

// ---------------------------------------------------------------------------
// Slowly arriving stdin: fread must keep reading until the buffer is full
// ---------------------------------------------------------------------------

#[test]
fn stdin_delivered_in_chunks() {
    assert_same_chunked("chunked-short", b"A\nx\nAAxx\n", 1);
    assert_same_chunked("chunked-999", &vec![b'A'; BUF - 1], 137);

    // Exactly 1000 bytes with a NUL inside, so the comparison stays exact while
    // still proving the read loop keeps going until the buffer is full.
    let mut v = vec![b'x'; BUF];
    v[BUF - 1] = 0;
    assert_same_chunked("chunked-exact-1000", &v, 250);

    // More than the buffer holds, arriving in pieces: the tail is discarded and
    // the writer may see EPIPE once the child exits. A NUL at 900 keeps the C
    // program inside its buffer.
    let mut v = vec![b'A'; BUF];
    v[900] = 0;
    v.extend(std::iter::repeat(b'x').take(BUF));
    assert_same_chunked("chunked-2000", &v, 300);
}

// ---------------------------------------------------------------------------
// Deterministic fuzz across the same input classes
// ---------------------------------------------------------------------------
#[test]
fn seeded_fuzz() {
    const ALPHABETS: &[&[u8]] = &[
        b"Ax",
        b"AxaX .\n",
        b"Ax\0",
        b"A",
        b"x",
        b".",
        b"\xffA\x80x\x00",
    ];
    const LENGTHS: &[usize] = &[
        0, 1, 2, 3, 7, 31, 255, 256, 511, 512, 997, 998, 999, 1000, 1001, 1002, 1023, 1024, 1999,
        2000, 4096,
    ];

    let mut rng = Rng::new(0xC0FF_EE12_3456_789A);
    for i in 0..300 {
        let n = LENGTHS[rng.below(LENGTHS.len())];
        let alpha = ALPHABETS[rng.below(ALPHABETS.len())];
        let data: Vec<u8> = (0..n).map(|_| alpha[rng.below(alpha.len())]).collect();
        // Exact comparison, except for inputs that fill all 1000 bytes without a
        // NUL: those make the C program read out of bounds (see ERRORS.md), so
        // they are compared against its modal output.
        assert_same_auto(&format!("fuzz-{i}-len-{n}"), &data, 5);
    }
}

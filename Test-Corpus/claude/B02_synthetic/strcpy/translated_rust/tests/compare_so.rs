// Integration test: compare C and Rust implementations of `process_strings`
// by loading both as shared libraries via libloading and asserting outputs
// are byte-identical.
//
// Both libraries share the same C ABI:
//   int process_strings(char *input, size_t input_len,
//                       const char *reference, size_t ref_len,
//                       int operation, uint32_t flags);

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

type ProcessStringsFn = unsafe extern "C" fn(
    input: *mut c_char,
    input_len: usize,
    reference: *const c_char,
    ref_len: usize,
    operation: c_int,
    flags: u32,
) -> c_int;

const MAX_BUFFER_SIZE: usize = 1024;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is the directory containing this crate's Cargo.toml.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libcdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // Built with `cargo build --release` (cdylib).
    workspace_root().join("target/release/librustdriver.so")
}

struct Libs {
    c_lib: Library,
    rust_lib: Library,
}

impl Libs {
    fn load() -> Self {
        unsafe {
            let c_lib = Library::new(c_lib_path()).unwrap_or_else(|e| {
                panic!("Failed to load C lib at {:?}: {}", c_lib_path(), e)
            });
            let rust_lib = Library::new(rust_lib_path()).unwrap_or_else(|e| {
                panic!("Failed to load Rust lib at {:?}: {}", rust_lib_path(), e)
            });
            Libs { c_lib, rust_lib }
        }
    }

    fn call_both(
        &self,
        input: &[u8],
        input_len: usize,
        reference: &[u8],
        ref_len: usize,
        operation: i32,
        flags: u32,
    ) -> (i32, i32) {
        // Always use a fixed 1024-byte buffer for both calls so the C code's
        // out-of-bounds reads (it relies on the buffer being zero-padded)
        // see the same memory in both libraries.
        let mut input_buf_c = [0u8; MAX_BUFFER_SIZE];
        let mut input_buf_r = [0u8; MAX_BUFFER_SIZE];
        let mut ref_buf_c = [0u8; MAX_BUFFER_SIZE];
        let mut ref_buf_r = [0u8; MAX_BUFFER_SIZE];

        let copy_len = input.len().min(MAX_BUFFER_SIZE);
        input_buf_c[..copy_len].copy_from_slice(&input[..copy_len]);
        input_buf_r[..copy_len].copy_from_slice(&input[..copy_len]);

        let ref_copy = reference.len().min(MAX_BUFFER_SIZE);
        ref_buf_c[..ref_copy].copy_from_slice(&reference[..ref_copy]);
        ref_buf_r[..ref_copy].copy_from_slice(&reference[..ref_copy]);

        unsafe {
            let c_func: Symbol<ProcessStringsFn> =
                self.c_lib.get(b"process_strings\0").unwrap();
            let rust_func: Symbol<ProcessStringsFn> =
                self.rust_lib.get(b"process_strings\0").unwrap();

            // Critical: pass the same input_len/ref_len. The C code reads
            // past those lengths until NUL — that's part of the spec we're
            // matching.
            //
            // To match the C `main` driver exactly, our Rust wrapper sees
            // pointers to a 1024-byte buffer. But we also want to test
            // shorter slices to expose any divergence in handling lengths.
            // Here we hand each library the full 1024-byte buffer and pass
            // the user-supplied lengths.
            let c_res = c_func(
                input_buf_c.as_mut_ptr() as *mut c_char,
                input_len,
                ref_buf_c.as_ptr() as *const c_char,
                ref_len,
                operation as c_int,
                flags,
            );
            let r_res = rust_func(
                input_buf_r.as_mut_ptr() as *mut c_char,
                input_len,
                ref_buf_r.as_ptr() as *const c_char,
                ref_len,
                operation as c_int,
                flags,
            );
            (c_res, r_res)
        }
    }
}

fn assert_match(
    libs: &Libs,
    label: &str,
    input: &[u8],
    input_len: usize,
    reference: &[u8],
    ref_len: usize,
    operation: i32,
    flags: u32,
) {
    let (c, r) = libs.call_both(input, input_len, reference, ref_len, operation, flags);
    assert_eq!(
        c, r,
        "Mismatch [{}]: op={} flags={} input={:?} input_len={} ref={:?} ref_len={} -> C={}, Rust={}",
        label, operation, flags, input, input_len, reference, ref_len, c, r
    );
}

// ---------------------------------------------------------------------------
// Operation 0: validate_token
// ---------------------------------------------------------------------------

#[test]
fn op0_validate_token() {
    let libs = Libs::load();

    // Direct match: token equals expected
    assert_match(&libs, "match", b"hello\0", 6, b"hello\0", 6, 0, 0);
    // Special variant: VALID
    assert_match(&libs, "valid", b"VALID\0", 6, b"foo\0", 4, 0, 0);
    // Special variant: OK
    assert_match(&libs, "ok", b"OK\0", 3, b"foo\0", 4, 0, 0);
    // Mismatch
    assert_match(&libs, "mismatch", b"abc\0", 4, b"xyz\0", 4, 0, 0);
    // Empty token
    assert_match(&libs, "empty", b"\0", 1, b"\0", 1, 0, 0);
    // Token equals expected with different lengths
    assert_match(&libs, "long", b"AAAAAAAA\0", 9, b"AAAAAAAA\0", 9, 0, 0);
}

// ---------------------------------------------------------------------------
// Operation 1: parse_command
// ---------------------------------------------------------------------------

#[test]
fn op1_parse_command() {
    let libs = Libs::load();

    for (i, cmd) in [
        b"START\0".as_slice(),
        b"STOP\0".as_slice(),
        b"PAUSE\0".as_slice(),
        b"RESUME\0".as_slice(),
        b"RESET\0".as_slice(),
    ]
    .iter()
    .enumerate()
    {
        let label = format!("cmd_{}", i);
        assert_match(&libs, &label, cmd, cmd.len(), b"\0", 1, 1, 0);
    }

    // ADMIN special command
    assert_match(&libs, "admin", b"ADMIN\0", 6, b"\0", 1, 1, 0);

    // No match
    assert_match(&libs, "none", b"FOO\0", 4, b"\0", 1, 1, 0);

    // Command followed by space (trailing data)
    assert_match(
        &libs,
        "start_with_arg",
        b"START arg\0",
        10,
        b"\0",
        1,
        1,
        0,
    );

    // Empty buffer
    assert_match(&libs, "empty_buf", b"\0", 1, b"\0", 1, 1, 0);

    // Command shorter buf
    assert_match(&libs, "short_buf", b"S\0", 2, b"\0", 1, 1, 0);

    // Buffer-size shorter than the command (buf_size < cmd_len)
    // Make a buffer with "START" but pass buf_size=3
    assert_match(&libs, "size3", b"START\0", 3, b"\0", 1, 1, 0);
}

// ---------------------------------------------------------------------------
// Operation 2: compare_prefix
// ---------------------------------------------------------------------------

#[test]
fn op2_compare_prefix_safe() {
    let libs = Libs::load();
    // exact = 0 (flags & 0x01 == 0): prefix-only via strncmp
    assert_match(&libs, "prefix_match", b"foobar\0", 7, b"foo\0", 4, 2, 0);
    assert_match(&libs, "prefix_diff", b"barfoo\0", 7, b"foo\0", 4, 2, 0);
    assert_match(&libs, "prefix_eq", b"foo\0", 4, b"foo\0", 4, 2, 0);
}

#[test]
fn op2_compare_prefix_exact() {
    let libs = Libs::load();
    // flags & 0x01: exact match
    assert_match(&libs, "exact_match", b"foo\0", 4, b"foo\0", 4, 2, 1);
    assert_match(&libs, "exact_v1", b"foo_v1\0", 7, b"foo\0", 4, 2, 1);
    assert_match(&libs, "exact_v2", b"foo_v2\0", 7, b"foo\0", 4, 2, 1);
    assert_match(&libs, "exact_old", b"foo_old\0", 8, b"foo\0", 4, 2, 1);
    assert_match(&libs, "exact_new", b"foo_new\0", 8, b"foo\0", 4, 2, 1);
    assert_match(&libs, "exact_tmp", b"foo_tmp\0", 8, b"foo\0", 4, 2, 1);
    assert_match(&libs, "exact_no", b"baz\0", 4, b"foo\0", 4, 2, 1);
}

// ---------------------------------------------------------------------------
// Operation 3: find_delimiter
// ---------------------------------------------------------------------------

#[test]
fn op3_find_delimiter() {
    let libs = Libs::load();
    // Default delimiter ':' with empty reference (uses default)
    assert_match(&libs, "default_colon", b"a:b\0", 4, b"\0", 0, 3, 0);
    // Custom delimiter '|'
    assert_match(&libs, "pipe", b"a|b\0", 4, b"|", 1, 3, 0);
    // Special: NONE pattern with '|'
    assert_match(&libs, "none_pipe", b"NONE\0", 5, b"|", 1, 3, 0);
    // Special: EMPTY pattern with ':'
    assert_match(&libs, "empty_colon", b"EMPTY\0", 6, b":", 1, 3, 0);
    // Not found
    assert_match(&libs, "nf", b"abc\0", 4, b":", 1, 3, 0);
    // Length 0
    assert_match(&libs, "len0", b"\0", 0, b":", 1, 3, 0);
    // Embedded NUL
    assert_match(&libs, "earlynul", b"a\0:b\0", 5, b":", 1, 3, 0);
}

// ---------------------------------------------------------------------------
// Operation 4: match_pattern
// ---------------------------------------------------------------------------

#[test]
fn op4_match_pattern_case_insensitive() {
    let libs = Libs::load();
    // case_sensitive = (flags & 0x02). flag=0 => case_sens = false
    // exact match
    assert_match(&libs, "exact", b"hello\0", 6, b"hello\0", 6, 4, 0);
    // case-insensitive (different case)
    assert_match(&libs, "case", b"HeLLo\0", 6, b"hello\0", 6, 4, 0);
    // prefix-match (different lengths)
    assert_match(&libs, "prefix", b"helloworld\0", 11, b"hello\0", 6, 4, 0);
    // no match
    assert_match(&libs, "nomatch", b"abc\0", 4, b"xyz\0", 4, 4, 0);
}

#[test]
fn op4_match_pattern_case_sensitive() {
    let libs = Libs::load();
    // flag=2 => case_sens = true
    // exact
    assert_match(&libs, "exact", b"hello\0", 6, b"hello\0", 6, 4, 2);
    // wildcard "*hello*"
    assert_match(&libs, "wild_both", b"*hello*\0", 8, b"hello\0", 6, 4, 2);
    // wildcard "hello*"
    assert_match(&libs, "wild_suffix", b"hello*\0", 7, b"hello\0", 6, 4, 2);
    // wildcard "*hello"
    assert_match(&libs, "wild_prefix", b"*hello\0", 7, b"hello\0", 6, 4, 2);
    // contained substring
    assert_match(&libs, "substring", b"abchelloxyz\0", 12, b"hello\0", 6, 4, 2);
    // no match
    assert_match(&libs, "nomatch", b"foo\0", 4, b"bar\0", 4, 4, 2);
    // pattern equals text
    assert_match(&libs, "equal", b"x\0", 2, b"x\0", 2, 4, 2);
    // NOTE: when pattern is strictly longer than text and case_sensitive=true,
    // the C code does `for (size_t i = 0; i <= text_len - pattern_len; i++)`,
    // which underflows size_t and reads out of bounds — undefined behavior.
    // We don't test that path because results are non-deterministic.
}

// ---------------------------------------------------------------------------
// Default / unknown operations
// ---------------------------------------------------------------------------

#[test]
fn unknown_operation_returns_neg3() {
    let libs = Libs::load();
    assert_match(&libs, "op5", b"x\0", 2, b"y\0", 2, 5, 0);
    assert_match(&libs, "op_neg", b"x\0", 2, b"y\0", 2, -1, 0);
    assert_match(&libs, "op99", b"x\0", 2, b"y\0", 2, 99, 0);
}

// ---------------------------------------------------------------------------
// Cross-product fuzz-ish coverage on a small set of inputs
// ---------------------------------------------------------------------------

#[test]
fn fuzz_small_grid() {
    let libs = Libs::load();
    let inputs: &[&[u8]] = &[
        b"START\0",
        b"STOP\0",
        b"hello\0",
        b"\0",
        b"foo_v2\0",
        b"abc\0",
        b"AbC\0",
        b"NONE\0",
        b"EMPTY\0",
        b"VALID\0",
        b"ADMIN\0",
        b"some long string with content\0",
    ];
    let refs: &[&[u8]] = &[
        b"START\0",
        b"hello\0",
        b"foo\0",
        b":",
        b"|",
        b"\0",
        b"abc\0",
        b"missing\0",
    ];
    fn c_strlen(b: &[u8]) -> usize {
        b.iter().position(|&c| c == 0).unwrap_or(b.len())
    }

    for op in 0..6 {
        for flags in 0u32..4 {
            for input in inputs {
                for r in refs {
                    // Skip op=4 with case_sensitive=true (flags & 0x02) when
                    // pattern strlen > text strlen — the C code underflows
                    // size_t and reads OOB (undefined behavior).
                    if op == 4 && (flags & 0x02) != 0 {
                        let tlen = c_strlen(input);
                        let plen = c_strlen(r);
                        if plen > tlen {
                            continue;
                        }
                    }
                    let label = format!("fuzz op={} flags={}", op, flags);
                    assert_match(&libs, &label, input, input.len(), r, r.len(), op, flags);
                }
            }
        }
    }
}

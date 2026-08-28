# Verification log — `c_src/src/container_of.c` → `translation/`

Ground truth: `c_src/build/driver`, built with
`cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
(GNU C 11.5.0, no `CMAKE_BUILD_TYPE`, so no optimisation flags).

Program under test: `translation/target/{debug,release}/driver`, built with
`cd translation && cargo build [--release]`.

Both are compared by execution only — spawned as subprocesses with the same
`argv`, then stdout, stderr and exit status (normal code *and* terminating
signal) are diffed. See `tests/differential.rs`.

## What the C program actually does

```c
int main(int argc, char** argv) {
    int a = atoi(argv[1]);
    int b = atoi(argv[2]);
    struct test t;                       /* { int a; int b; } */
    memset(&t, 0, sizeof(t));
    t.a = a; t.b = b;
    printf("%d\n", find_container_of_a(&t.a)->a + find_container_of_b(&t.b)->b);
}
```

The `container_of()` round-trips are pure identity: they subtract a member
offset from `&t.<member>` and immediately re-read the same member, so the
printed value is always `(int)(t.a + t.b)`. There is no `argc` check and no
stdin use. `main` falls off the end, which in C99+ means `return 0`.

## Input classes enumerated from the source

| Class | Reached by | Observable result |
|---|---|---|
| `argc == 1` (no args) | `driver` | `argv[1]` is `NULL`; `atoi` dereferences it → SIGSEGV, empty stdout/stderr |
| `argc == 2` (one arg) | `driver 1` | `argv[1]` parses, then `argv[2]` is `NULL` → SIGSEGV, empty stdout/stderr |
| `argc >= 3` | `driver 1 2` | prints `3\n`, exit 0 |
| `argc > 3` | `driver 1 2 3 4` | extra args ignored, prints `3\n`, exit 0 |
| `atoi` no digits | `""`, `abc`, `-`, `+`, `--5`, `- 5` | 0 |
| `atoi` leading C-locale whitespace | `" \t\n\v\f\r-123"` | whitespace skipped, then parsed |
| `atoi` stops at first non-digit | `12abc`, `3.9`, `1e3`, `1,000`, `0x10` | prefix only |
| `atoi` `long` → `int` truncation | `2147483648`, `4294967296`, `4294967295` | low 32 bits, sign-reinterpreted |
| `atoi` `strtol` overflow saturation | `99999999999999999999`, `-99999999999999999999` | `LONG_MAX`→`-1`, `LONG_MIN`→`0` |
| `int` addition overflow | `2147483647 1`, `-2147483648 -1` | wraps (gcc at `-O0`) |
| non-UTF-8 `argv` bytes | `$'\xff5'` | bytes passed through verbatim |

## Mismatches found

**None.** Every enumerated input — plus a 17×17 numeric edge matrix, a
300-case deterministic pseudo-random `argv` sweep inside `cargo test`, and a
separate ~4200-case out-of-band fuzz sweep — produced byte-identical stdout,
byte-identical stderr and an identical exit status (including `signal == 11`
for the two `NULL`-dereference cases) for both the debug and the release Rust
build.

The existing translation already handled the four behaviours that a naive
translation gets wrong. They are recorded here because they are the mismatches
this suite exists to catch, and each one is now pinned by a named test:

1. **Missing arguments must fault, not exit cleanly.**
   `atoi(argv[1])` on a `NULL` `argv` slot segfaults; it does not print a usage
   message and does not exit 1 or 2. A translation that used
   `args.get(1).unwrap_or("0")`, `expect(...)`, or `eprintln!` + `exit(1)` would
   match stdout (empty) while differing on stderr and on exit status.
   Pinned by `no_arguments_null_derefs_in_atoi`,
   `one_argument_null_derefs_on_argv2` and
   `missing_arguments_die_by_signal_not_clean_exit` — the last asserts
   `code == None` and `signal == Some(SIGSEGV)` on *both* sides, so a clean exit
   on the Rust side cannot slip through.
   Reproduced in `main.rs` by `null_dereference()`, a
   `read_volatile(null::<u8>())`; verified to raise SIGSEGV in both the
   `dev` and `release` profiles.

2. **`atoi` is not `str::parse`.** glibc's `atoi` is
   `(int) strtol(nptr, NULL, 10)`: it skips leading whitespace, accepts an
   optional sign, consumes the leading digit run and *ignores the rest*, and
   yields 0 when there are no digits. `"12abc".parse::<i32>()` errors instead.
   Pinned by `atoi_empty_and_non_numeric_yield_zero`,
   `atoi_skips_leading_c_locale_whitespace`, `atoi_stops_at_first_non_digit`.
   Note the whitespace set is the C-locale one (`' '`, `\t`, `\n`, `\v`, `\f`,
   `\r`); U+00A0 is *not* skipped, so `"\xc2\xa07"` is 0, not 7.

3. **Overflow saturates in `long`, then truncates to `int`.** `strtol` clamps
   to `LONG_MAX`/`LONG_MIN` and the cast keeps the low 32 bits, so
   `atoi("99999999999999999999") == -1` (not `i32::MAX`, not 0) and
   `atoi("-99999999999999999999") == 0` (not `i32::MIN`). Values that fit in
   `long` but not `int` simply truncate: `atoi("4294967296") == 0`,
   `atoi("2147483648") == -2147483648`. A saturating-to-`i32` translation would
   differ on all of these. Pinned by
   `atoi_truncates_the_long_result_to_int`,
   `atoi_saturates_long_on_overflow_then_truncates`,
   `atoi_handles_very_long_digit_runs`.

4. **The sum wraps.** Signed overflow is UB in C, but gcc at `-O0` wraps, and
   that is the ground truth being compared against. `t.a + t.b` must therefore
   be `wrapping_add`; a plain `+` would panic in a debug build and an
   `i32::saturating_add` would print `2147483647` where C prints `-2`.
   Pinned by `sum_wraps_like_c_int_arithmetic`.

Two lower-risk behaviours are also pinned, since both are easy to get wrong and
neither shows up on the happy path:

5. **`argv` is bytes, not UTF-8.** `std::env::args()` panics on non-UTF-8
   arguments; the translation uses `args_os()` + `OsStrExt::as_bytes()`, so
   `driver $'\xff5' 6` behaves like C. Pinned by
   `arguments_need_not_be_valid_utf8`.

6. **stdin is never read.** Neither program touches stdin, so piping data in
   must change nothing, including when the process is about to segfault.
   Pinned by `stdin_is_never_read`.

## Phase D status

- `cd c_src/build && cmake .. && cmake --build .` — succeeds, no errors.
- `cd translation && cargo build --release` — succeeds, no errors, no warnings.
- `cd translation && cargo test` — 19 tests, all pass.
- `cargo test` re-run with `RUST_DRIVER_BIN=$PWD/target/release/driver` — all
  pass, so the release binary is verified too, not just the test-profile one.
- No test is `#[ignore]`d, skipped or otherwise disabled.
- Nothing under `c_src/` was modified; only the `build/` output directory the
  task instructions asked for was created there.

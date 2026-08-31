# ERRORS.md — differential verification of the Rust translation

Ground truth: `c_src/src/main.c`. Comparison method: build both executables, feed
identical bytes on stdin, diff stdout, stderr and exit status.

- C binary: `c_src/build/driver` (`cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`)
- Rust binary: `translation/target/release/driver` (`cd translation && cargo build --release`)
- Test suite: `translation/tests/differential.rs`, run with `cd translation && cargo test`

## Result

**No mismatches were found.** The translation as delivered is byte-identical to
the C program on every input class exercised: 20 test functions covering several
hundred distinct inputs, plus an ad-hoc fuzz run of 1167 additional integer
inputs, all agreeing on stdout, stderr and exit status. Nothing in the Rust
source needed to change, and no test is disabled or `#[ignore]`d.

This file therefore records the behaviors that *could* have diverged, why they
do not, and how each is pinned by a test — so the next reader can re-check the
reasoning rather than take the green suite on faith.

## Program shape

`main()` is straight-line: one `scanf("%d", &x)` on a variable initialized to
`0`, then `run(x)` twice, then `return 0`. There are no conditionals and no
early returns. The only branching in the whole program is inside `scanf`, which
has three outcomes:

1. successful conversion — `x` takes the converted value;
2. matching failure (no convertible digits) — `scanf` returns `0` and **leaves
   `x` at its initializer `0`**;
3. input failure (EOF or a read error before any conversion) — `scanf` returns
   `EOF` and likewise leaves `x` at `0`.

Because outcomes 2 and 3 leave `x` untouched rather than zeroing it, they are
indistinguishable from an explicit input of `0` in this program. The exit status
is always `0` and stderr is always empty; there is no error path that changes
either.

The other source of behavior is the mutable global `the_house`, which is **not**
reset between the two `run()` calls, so the second call starts from the state
the first left behind (floors 4 not 3, bathrooms 4.5 not 3.5, bedrooms with
`extra_bedrooms` added twice). A translation that made the house a local, or
re-initialized it per call, would produce eight plausible-looking lines that are
wrong from line 5 onward.

## Risk areas checked, and why they match

### 1. `scanf` crosses newlines (unlike `fgets`)

`%d` skips leading whitespace of every kind — space, tab, newline, vertical tab,
form feed, carriage return — before converting. The Rust `Scanner::scan_i32`
skips exactly that set via `is_space`. Verified with inputs such as
`"\n\n\n\n42"`, `"\t\t7"`, `"  \t\r\n  12\n"`.
Test: `scanf_skips_leading_whitespace_across_newlines`, `whitespace_only_input`.

### 2. Only one integer is consumed; the rest of stdin is ignored

`scanf` is called once. Trailing content — a second number, letters, a megabyte
of junk — is never read, and the process exits with data still in the pipe. The
test writer tolerates the resulting `EPIPE` on its side so that this does not
masquerade as a failure.
Tests: `only_the_first_number_is_read`, `large_input_is_not_fully_consumed`.

### 3. Out-of-range conversion: clamp in `long`, then truncate to `int`

This is the subtlest agreement. glibc's `%d` converts through a `long`,
saturating at `LONG_MAX` / `LONG_MIN` on overflow, and only then assigns to the
`int*`, truncating. So:

| input | glibc `long` | stored `int` |
| --- | --- | --- |
| `9223372036854775807` and up | `LONG_MAX` | `-1` |
| `-9223372036854775808` and below | `LONG_MIN` | `0` |

`Scanner::scan_i32` reproduces this by accumulating into `i64` with
`saturating_mul`/`saturating_add` and then casting `as i32`. Saturating at each
step and clamping once at the end coincide here because both land on exactly
`i64::MAX` / `i64::MIN`. A naive translation using `i32` accumulation, or
`str::parse::<i32>()` with a fallback, would give `0`, `i32::MAX`, or a parse
error instead of `-1`.
Tests: `out_of_range_conversion_is_clamped_then_truncated`,
`very_long_digit_strings` (4000-digit inputs).

### 4. Base 10 only

`%d` is decimal, so `0x10` converts as `0` and stops at `x`; `1e5` converts as
`1`; `5.9` converts as `5`; `0755` converts as decimal 755, not octal.
Test: `base_ten_only`.

### 5. Signed-overflow wrapping in `bedrooms += extra_bedrooms`

`bedrooms` starts at 5 and the addition runs twice, so `extra_bedrooms` near
`INT_MAX` overflows a signed `int`. This is UB in C, but the CMakeLists sets no
optimization level, so gcc emits a plain `addl` and it wraps two's-complement.
`wrapping_add` in the Rust matches: for input `2147483647` both print
`-2147483644` bedrooms on lines 4–7 and `3` on line 8.

Note this agreement rests on how the compiler happens to lower UB, not on the
standard. It is pinned by a test rather than by the language, and a different
compiler or optimization level could in principle change the C side.
Tests: `int_boundaries_and_signed_overflow`, `deterministic_sweep_over_int_range`.

### 6. `%.1f` formatting

Every value `bathrooms` ever takes (2.5, 3.5, 4.5) is exactly representable as a
double and already has one decimal digit, so `%.1f` performs no rounding and
Rust's `{:.1}` cannot disagree. The `format_f64_1` helper's nan/inf and
round-half handling is therefore dead code for this program, correct but
unexercised — `bathrooms` is only ever `+= 1.0` from 2.5.
Tests: `golden_output_for_one`, `bathrooms_formatting_is_stable_across_both_runs`.

### 7. Global state persists across both `run()` calls

Pinned byte-for-byte by `golden_output_for_one`, and independently by
`bathrooms_formatting_is_stable_across_both_runs`, which asserts the bathroom
sequence is `2.5 2.5 3.5 3.5 3.5 3.5 4.5 4.5` — the `3.5` repeating across the
call boundary is the signature of carried-over state.

### 8. Read-error paths on stdin

Closed stdin (`Stdio::null`) and stdin pointing at a directory (`read(2)` fails
`EISDIR`) both leave `x` at `0`, print the normal eight lines, and exit `0`, in
both programs.
Tests: `stdin_closed_entirely`, `stdin_is_a_directory`.

### 9. Non-ASCII and embedded NUL bytes

Raw byte inputs (`\x00`, `\xff`, invalid UTF-8) behave identically. The Rust
reader works on bytes rather than `String`, so it does not reject non-UTF-8
stdin the way a `read_to_string` based translation would.
Test: `non_ascii_and_embedded_nul_bytes`.

### 10. `argv` is ignored

C `main()` is declared with no parameters, so arguments cannot affect it.
Confirmed the Rust binary likewise ignores them.
Test: `argv_is_ignored`.

## Checked and found equivalent, though outside the input space

- **Closed stdout / `SIGPIPE`:** piping into a reader that closes immediately
  yields exit `0` from both.
- **Write failure (`> /dev/full`):** C's `printf` buffers and `exit` does not
  check the flush result, so it exits `0` with empty stderr. The Rust ignores
  its `write!` and `flush` results, so it also exits `0` with empty stderr.
  Had the Rust used `println!`, it would have panicked on flush failure and
  exited non-zero with a message on stderr; the explicit `let _ =` is what keeps
  these aligned.

## Untested residue

- Behavior under an optimizing C compiler for the signed-overflow inputs of
  §5, which is UB and not contractually stable.
- `format_f64_1`'s nan/inf branches, unreachable from any input (§6).

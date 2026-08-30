# Differential verification report

Verification of `translation/` against `c_src/` by running both binaries as
subprocesses and diffing stdout, stderr and exit status.

- C reference: `c_src/build/driver` (built with `cmake .. && cmake --build .`)
- Rust binary: `translation/target/release/driver` (`cargo build --release`)
- Test suite: `translation/tests/differential.rs` (17 tests, none skipped or
  `#[ignore]`d)

## Outcome

**No mismatches were found.** Every input class enumerated below produced
byte-identical stdout, byte-identical stderr and an identical exit status.

- Phase A: both programs build clean. The Rust crate compiled with no errors
  and no warnings on the first `cargo build --release`; no source changes were
  required to make it build.
- Phase B/C: ~3,000 ad-hoc differential cases plus the 17 committed tests all
  agree. `src/main.rs` was **not modified** during verification.

Because no defect was found, the sections below record the behaviours that were
*at risk* of divergence — the places where a naive translation would have
broken — and the evidence that this translation gets each one right. These are
the checks a future reader should re-run after any change.

## The program under test

```c
void driver(int x) {
    auto int y = 2*x;      // `auto` is just a storage-class keyword; no effect
    y += 300;
    printf("%d\n", y);
}

int main() {
    int x = 0;
    scanf("%d", &x);       // return value ignored
    driver(x);
    return 0;
}
```

Exit status is always 0. stderr is always empty. stdout is always exactly
`2*x + 300` followed by one `\n`. All the interesting behaviour is in how `x`
is obtained and how the arithmetic overflows.

## Risk 1 — `scanf` failure leaves `x` at 0

`scanf`'s return value is discarded. On EOF (input failure) or a matching
failure, C leaves `*&x` untouched, so `x` keeps its initialiser `0` and the
program prints `300`.

A translation that defaulted to a parse error path, exited non-zero, or wrote a
diagnostic to stderr would diverge. The Rust models this correctly: `scanf_d`
returns `Option<i32>` and `main` only assigns on `Some`.

Verified identical for: empty input, `\n`, `\n\n\n`, a single space, a lone tab,
`" \t\n\x0b\x0c\r"`, `abc`, `.5`, `-`, `+`, `--5`, `+-5`, `+ 5`, `" - 5"`,
`,;!`, a leading NUL byte, high bytes (`\xff\xfe`), invalid UTF-8 (`\xc3\x28`),
and a full-width UTF-8 digit `５`. All print `300`, exit 0, empty stderr.

## Risk 2 — `%d` skips whitespace *including newlines*

`scanf("%d")` skips arbitrary leading whitespace and reads across newlines,
unlike `fgets`. A line-oriented translation built on `read_line` would return
`300` for `"\n\n7"` where C returns `314`.

The Rust skips the full C whitespace set (space, `\t`, `\n`, `\v`, `\f`, `\r`)
in a loop before parsing. Verified with `"   7"`, `"\n\n7"`,
`" \t\n \r\n 7"`, `"\x0b\x0c9"`, and 1 MB of newlines followed by `123`.

*Mutation check:* making the skip loop stop at `\n` made 3 tests fail, including
`leading_whitespace_is_skipped_across_newlines` and `randomized_sweep`.

## Risk 3 — signed overflow in `2*x + 300`

`2*x` overflows `int` for `x > 2^30`. This is UB in C, but the CMake build uses
no optimisation flags, so gcc emits a plain `imull`/`addl` pair that wraps
two's-complement. The Rust must reproduce the wrap, not panic and not saturate.

The Rust uses `wrapping_mul` / `wrapping_add`. Note the release profile sets
`panic = "abort"`, so plain `*`/`+` would have aborted with a non-zero status
and a stderr message in a debug build — `wrapping_*` is load-bearing here.

Verified identical for `INT_MAX` (`2147483647` → `298`), `INT_MAX-1`,
`INT_MIN` (`-2147483648` → `300`), `INT_MIN+1`, `2^30` (`1073741824` →
`-2147483348`), `2^30±1`, `-2^30`, `-2^30-1`, and the values `1073741673` /
`1073741674` that straddle the boundary where `2*x+300` overflows.

*Mutation check:* replacing `2i32.wrapping_mul(x)` with `x.saturating_mul(2)`
made `int_overflow_in_arithmetic_wraps` fail on `2147483647`.

## Risk 4 — `%d` converts as `long`, then truncates to `int`

This is the subtlest behaviour and the easiest to get wrong. glibc's `%d`
converts the digit string with `strtol` semantics into a 64-bit `long`, then
assigns it to an `int`, discarding the high bits. So:

| input | `long` value | truncated to `int` | printed |
|---|---|---|---|
| `2147483648` (INT_MAX+1) | 2147483648 | -2147483648 | `300` |
| `4294967296` (2^32) | 4294967296 | 0 | `300` |
| `-2147483649` (INT_MIN-1) | -2147483649 | 2147483647 | `298` |

And when the value exceeds `long`, `strtol` **saturates** to `LONG_MAX` /
`LONG_MIN` first, and *then* the low 32 bits are taken:

| input | saturates to | low 32 bits | printed |
|---|---|---|---|
| `99999999999999999999` | `LONG_MAX` | -1 | `298` |
| `-99999999999999999999` | `LONG_MIN` | 0 | `300` |
| `9223372036854775808` | `LONG_MAX` | -1 | `298` |

A translation that clamped to `i32::MIN..=i32::MAX`, or that used
`i64::saturating_*` without the final truncation, would diverge on every row
above. The Rust accumulates into an `i64` with saturation on overflow and then
performs `acc as i32`, which is exactly this two-step behaviour.

Verified identical for all rows above plus `2147483649`, `4294967295`,
`4294967297`, `LONG_MAX`, `LONG_MIN`, `LONG_MIN-1`, `10^25` and `-10^25`.

*Mutation check:* changing `Some(acc as i32)` to a `clamp` to the `i32` range
made `scanf_out_of_int_range_truncates`, `very_long_inputs` and
`randomized_sweep` fail, first on input `2147483648`.

## Risk 5 — conversion stops at the first non-digit; `%d` is decimal only

`%d` is not `%i`, so `0x10` is read as `0` and stops at `x` (printing `300`, not
`316`). Leading zeros are decimal, not octal, so `010` is 10 and prints `320`.
Trailing input after the number is simply never read.

Verified: `5abc`, `0x10`, `5 6` (only the first value is read → `310`), `42abc`,
`5\x00`, `3.9`, `1e9`, `007`, `010`, `000000000000000000001`.

## Risk 6 — the Rust reader's buffer boundary and one-byte pushback

The Rust hand-rolls a 4096-byte buffered reader with an `ungetc` that decrements
`pos`. Two hazards: `fill()` clears the buffer on refill, so a pushback that
would cross a refill boundary is silently dropped; and `ungetc` at `pos == 0`
does nothing. Neither is observable in *this* program, because the pushed-back
byte is never read again — `scanf` is called exactly once and nothing else
touches stdin. This is a latent trap for any future change that reads a second
value, not a current mismatch.

Exercised anyway at and around the boundary: N spaces then `7`, a value
straddling the boundary followed by junk, and a digit run ending exactly at the
boundary (with and without a following `x`), for N in 4093–4098, 8191–8193 and
65536. Also 100k `9`s, `-` + 100k `9`s, 5000 zeros then `3`, 4090 zeros then
`2147483648`, 1 MB of spaces then `8`, and 200k newlines.

## Risk 7 — stream and environment edge cases

All verified identical:

- stdin closed outright (`0<&-`) and stdin from `/dev/null` → `300`, exit 0
- stdin from a **regular file** rather than a pipe (different read block sizes)
- stdout to a closed pipe (`| head -0`) → both exit 0; `printf` failure is
  ignored by both, matching C's discarded `printf` return value
- stdout to `/dev/full` (write error) → both exit 0 with empty stderr
- extra `argv` entries (`driver a b c`) → ignored by both, since `main` takes no
  parameters

## Randomised sweep

`randomized_sweep` runs a deterministic xorshift generator (no external crates)
over 400 random tokens drawn from digits/signs/whitespace/junk, 200 random raw
byte strings over the full 0–255 range, and 200 random integers spread across
and beyond the `int` range. Separately, an ad-hoc Python sweep of 2,961 cases
during investigation found 0 mismatches.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd ../../translation && cargo build --release && cargo test --release
```

The test harness builds the C binary via CMake automatically if
`c_src/build/driver` is absent, so `cargo test` alone is sufficient.
`c_src/` was not modified during verification.

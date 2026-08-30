# Differential verification log — `driver`

C ground truth: `c_src/src/main.c`, built with CMake to `c_src/build/driver`.
Rust under test: `translation/src/main.rs`, built to `translation/target/{debug,release}/driver`.
Comparison: both programs run as subprocesses with identical stdin; stdout, stderr
and exit status compared byte for byte (`translation/tests/differential.rs`).

## Commands

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test                                            # differential suite
```

## Result

**No mismatches were found.** Every input class enumerated below produced
identical stdout, identical stderr (always empty) and exit status 0 from both
programs, in both the debug and release Rust profiles. In addition to the 19
table-driven tests, a randomized differential fuzz of ~3000 inputs drawn from the
alphabet `0-9 + - . x X a b e E SPACE TAB NL CR VT FF NUL 0xFF` (lengths 0–25)
produced zero divergences.

Because nothing diverged, the Rust program was not changed during verification.
The sections below record the behaviors that *were* the candidate mismatches —
the places where a naive translation would have diverged — and the observed C
behavior each one was checked against.

## Program shape and its branch points

```c
void driver(int x) { register int y = 2*x; y += 300; printf("%d\n", y); }
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

There is no argument handling, no explicit error path and no `return` other than
`return 0`, so the exit status is always 0 and stderr is always empty. All
branching lives inside the `%d` conversion and in the 32-bit arithmetic.

## Candidate mismatches checked

### 1. Failed conversion must leave `x` at its initializer

`scanf` returns `EOF` on input failure and `0` on matching failure; in both cases
it assigns nothing, so `x` keeps the `= 0` from its declaration and the program
prints `300` rather than reporting an error. Verified for: empty stdin, `/dev/null`,
closed stdin, whitespace-only input, non-numeric first character, and a sign with
no digits after it (`-`, `+`, `- 5`, `--5`).

A translation that treated a parse failure as an error would print nothing and
exit non-zero here. The Rust code returns `None` from `scan_int` and leaves `x`
untouched, matching.

### 2. `%d` skips whitespace across newlines

Directive whitespace is unbounded and includes `\n`, `\t`, `\r`, `\v`, `\f`, so
`"\n\n\t 7\n"` yields 7, not a failure. This is the `scanf`-vs-`fgets`
distinction: a line-oriented translation would have failed on leading blank
lines. Verified up to 70,000 leading newlines.

### 3. The conversion stops at the first byte that cannot extend the number

`12abc` → 12, `12.75` → 12, `0x10` → 0, `1e3` → 1, `1,000` → 1, `5\0` → 5.
Leading zeros are decimal, not octal: `007` → 7, `010` → 10.

### 4. Out-of-range magnitudes clamp to `long`, then truncate to `int`

glibc parses `%d` into a `long` (clamping to `LONG_MAX` / `LONG_MIN` on overflow)
and stores it through the `int *`, keeping the low 32 bits. So:

| input | long value | stored `int` | printed |
|---|---|---|---|
| `9223372036854775807` (`LONG_MAX`) | `LONG_MAX` | `-1` | `298` |
| `9223372036854775808` (clamped) | `LONG_MAX` | `-1` | `298` |
| `99999999999999999999` (clamped) | `LONG_MAX` | `-1` | `298` |
| `-9223372036854775808` (`LONG_MIN`) | `LONG_MIN` | `0` | `300` |
| `-99999999999999999999` (clamped) | `LONG_MIN` | `0` | `300` |
| `4294967296` (2^32) | `4294967296` | `0` | `300` |
| `3000000000` | `3000000000` | `-1294967296` | `1705033004` |

Two distinct wrong answers were possible here and both are avoided: saturating to
`i32::MAX`/`i32::MIN` instead of truncating, and detecting overflow at 32 bits
instead of at 64. The Rust `scan_int` accumulates in `u128`, saturates at the
`long` boundary, then casts `i64 as i32`. Note this is glibc/LP64-specific
behavior, which is what the C ground truth exhibits on this platform.

### 5. `2*x + 300` is wrapping 32-bit signed arithmetic

`INT_MAX` → `2*x` wraps to `-2`, printed `298`. `INT_MIN` → `2*x` wraps to `0`,
printed `300`. `2^30` → `2*x` is exactly `INT_MIN`, printed `-2147483348`.
A debug-profile Rust build using plain `*` and `+` would panic with "attempt to
multiply with overflow" where the C wraps; `wrapping_mul` / `wrapping_add` are
used instead, and the suite is run in the debug profile precisely to catch that.

### 6. Output formatting

`printf("%d\n", y)` — decimal, no padding, single trailing newline, nothing else
on stdout. The Rust `write!(out, "{}\n", y)` matches byte for byte, including for
negative results.

## Input classes covered by the suite

Empty; `/dev/null`; closed stdin; each whitespace character alone and combined;
70k-newline prefix; every one of the 256 possible leading bytes; non-numeric
leads (`abc`, `.5`, `e5`, `,5`, `\0`, `\xff`); lone and repeated signs; single
digit; small values around the `2*x+300 == 0` point (`-150`, `-151`, `149`); no
trailing newline; `-0`; leading-zero padding (20 and 4096 zeros); digits followed
by non-digits; multiple numbers on one line; `INT_MAX`/`INT_MIN` and their
neighbours; `2^30`, `1073741674` (result lands exactly on `INT_MIN`); values
between 2^31 and 2^64; `LONG_MAX`/`LONG_MIN` ±1; magnitudes far past 2^64;
100,000-digit runs (positive and negative); and a sweep of `±(2^k − 1, 2^k, 2^k + 1)`
for k = 0..63.

## Completion gate

- Both programs build with no errors: yes.
- Every enumerated input produces identical stdout, stderr and exit status: yes.
- `cargo test` passes in `translation/` (debug and `--release`): yes, 19/19.
- No test disabled, skipped or `#[ignore]`d: none.
- Nothing in `c_src/` modified: `c_src/src/main.c` and `c_src/CMakeLists.txt`
  untouched; only the generated `c_src/build/` directory was added by the
  prescribed CMake build.

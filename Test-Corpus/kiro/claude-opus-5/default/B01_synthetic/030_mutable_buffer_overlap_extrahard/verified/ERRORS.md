# Differential verification: `c_src/src/main.c` vs `translation/src/main.rs`

Status: **no mismatches found.** Every input class enumerated below produced
byte-identical stdout, byte-identical stderr and an identical exit status.
No changes to `translation/src/main.rs` were required.

## How it was verified

- C reference: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver` (CMake 3.22.2, no `CMAKE_BUILD_TYPE`, so unoptimized).
- Rust: `cd translation && cargo build --release` → `translation/target/release/driver`.
- Tests: `translation/tests/differential.rs` spawns **both** binaries as
  subprocesses, writes the same bytes to each one's stdin, and compares stdout,
  stderr and exit status. The Rust crate is never loaded as a library.
- Both `cargo test` and `cargo test --release` pass: 18 tests, 0 ignored,
  ~1,700 distinct inputs including two seeded random sweeps.

## The whole surface of the C program

```c
void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)
    for (i = 0; i < len; i++) out[i] = mul1[i] * mul2[i] + add[i];

void driver(int *out, int len)
    fma_array(out, out, out, out, len);      // all four pointers alias `out`
    for (i = 0; i < len; i++) printf("%d\n", out[i]);

int main(void)
    int data[100];
    for (i = 0; i < 100; i++) if (scanf("%d", &data[i]) != 1) break;
    driver(data, i);
    return 0;
```

There is no `stderr` write and no non-zero `return` anywhere: the program
**always** exits 0 with empty stderr. So the only observable is stdout, and the
only branches are the read loop's two exits (`i == 100`, or `scanf != 1`) and
the two `i < len` loops.

## Semantic hazards checked, and why the Rust already matched

These are the places a translation of this program is most likely to diverge.
Each was probed directly; none diverged.

| # | Hazard | C behaviour | Rust behaviour | Result |
|---|--------|-------------|----------------|--------|
| 1 | `scanf("%d")` crosses newlines | `%d` skips *all* leading whitespace, so `"1 2 3"`, `"1\n2\n3"` and `"1\r\n\t\x0b2 3"` are the same input | `Scanner::is_space` covers `' ' \t \n \x0b \x0c \r`, i.e. C-locale `isspace`, and is skipped before the sign | match |
| 2 | Matching failure vs. EOF | `scanf` returns `0` on junk and `EOF` at end of input; **both** fail `!= 1` and `break` identically, so the distinction is unobservable and the stream position after a failure never matters | `ScanInt::MatchFailure` and `ScanInt::Eof` are both `break` arms | match |
| 3 | Value larger than `int` | glibc's `%d` converts via `long` then narrows: `4294967296` → `0`, `2147483648` → `-2147483648` | parse into `i64`, then `as i32` (a wrapping truncation) | match |
| 4 | Value larger than `long` | glibc clamps to `LONG_MAX`/`LONG_MIN` before narrowing, so `99999999999999999999` → `-1` and `-99999999999999999999` → `0` | `saturating_mul`/`saturating_add`/`saturating_sub` on `i64`, then `as i32`; `i64::MAX as i32 == -1`, `i64::MIN as i32 == 0` | match |
| 5 | Very long digit runs | glibc buffers the whole token; 400- and 5000-digit inputs still clamp | saturation is monotone, so extra digits change nothing once clamped | match |
| 6 | Signed multiply overflow | `out[i]*out[i]` is UB in the standard but this compiler emits a wrapping `imul`; `46341` → `-2147432674` | `wrapping_mul` then `wrapping_add` | match |
| 7 | Signed add overflow | the `+ add[i]` can wrap a second time (e.g. `46349`, `92682`) | `wrapping_add` | match |
| 8 | Aliased pointers | all four parameters are the same buffer, so the C reads `out[i]` three times *after* earlier iterations already wrote `out[0..i]` — but element `i` only ever depends on element `i`, so no cross-element hazard exists | `fma_array_aliased` reads `out[i]` three times then writes it, in place | match |
| 9 | Leading zeros | `%d` is decimal, not octal: `010` → `10`, and `08`/`09` are valid | plain base-10 accumulation | match |
| 10 | `int data[100]` is uninitialized | only `data[0..i]` is read, and each of those was written by a successful `scanf` | zero-initialized array, same `0..i` window read | match |
| 11 | Capacity cut-off | the loop stops at `i == 100`; input beyond the 100th integer is never consumed and never affects output | `while i < 100` | match |
| 12 | `printf("%d\n")` | no padding, no grouping, one trailing newline per item, nothing extra at end of output | `writeln!(w, "{}", ...)`; `i32` `Display` matches `%d` including `-2147483648` | match |
| 13 | Empty output flushing | `len == 0` prints nothing | `BufWriter` flushed at the end; nothing written | match |

## Input classes covered by `tests/differential.rs`

- **empty / whitespace-only**: `""`, `" "`, `"\n"`, `"\n\n\n\n"`, `" \t\r\n\x0b\x0c "`, `"\t\t\t"`
- **single item** (`len == 1`): with and without a trailing newline, leading and
  trailing whitespace, `0`, `1`, `-3`, `+7`, `-0`, `+0`
- **layout independence**: one-per-line, all-on-one-line, CRLF, vertical-tab /
  form-feed separated, ragged whitespace, no trailing newline, leading blank lines
- **every matching-failure path** (`scanf` → `0`, `break`): junk only, junk
  first / middle / last, `12abc`, `0x10`, `0X1F`, lone `-`, lone `+`, `-x`, `+x`,
  `--5`, `+-5`, `- 5`, `1.5`, `.5`, `1,2,3`, `1e3`, `1_000`, embedded `NUL`,
  leading `NUL`, a high (non-ASCII) byte, `1/2`, `*`
- **`int` extremes**: `INT_MAX`, `INT_MIN`, and their neighbours
- **out-of-range conversion**: `2147483648`, `-2147483649`, `2^32`, `2^32+1`,
  `LONG_MAX`, `LONG_MIN`, `LONG_MAX+1`, `LONG_MIN-1`, twenty nines, 400 nines
- **long digit runs**: 1, 9, 10, 18, 19, 20, 21, 25, 50, 200, 1000 and 5000
  digits, in nines / ones / zero-padded / signed variants
- **arithmetic overflow**: dense scans around `±46340`, `±65535`, `2^15`, `2^16`,
  `2^20`, `2^30`, `INT_MAX`/`INT_MIN`, plus values chosen so the *addition*
  wraps after the multiply
- **capacity boundary**: 98, 99, 100 and 101 items; 150 and 1000 items (excess
  ignored); exactly 100 followed by junk; 100 followed by trailing whitespace;
  100 copies of `100000` / `INT_MAX` / `INT_MIN`
- **every reachable `len`**: a case for each of `len == 0 ..= 100`
- **sweeps**: 600 random byte strings over the alphabet the parser branches on,
  400 random integer lists (0–120 items, mixed magnitudes and separators), and
  valid-prefix-then-junk at 10 prefix lengths × 9 junk tokens

## Notes on the test harness

- Comparison is on raw bytes, not lines, so a missing or extra trailing newline
  would fail.
- Exit status comparison distinguishes a normal exit code from death by signal,
  so a Rust panic (`101`) or an abort could not be mistaken for the C's `0`.
- Writing to the child's stdin ignores `EPIPE`, because the C program legitimately
  stops reading after the 100th integer; the child's own output and status are
  still compared in full.
- `c_src` is read-only for the tests. They prefer the `c_src/build/driver` the
  task instructions produce; if it is absent they configure an out-of-source
  CMake build into `translation/target/c_reference_build/` rather than writing
  anything into `c_src/`. Nothing under `c_src/` was modified.

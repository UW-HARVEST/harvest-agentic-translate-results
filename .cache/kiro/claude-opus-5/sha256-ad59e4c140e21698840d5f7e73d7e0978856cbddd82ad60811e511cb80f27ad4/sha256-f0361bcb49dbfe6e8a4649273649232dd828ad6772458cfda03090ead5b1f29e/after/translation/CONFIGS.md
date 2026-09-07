# CONFIGS.md — configuration / valid-input surface table (Phase B gate)

Mechanically derived from `c_src/include/driver.h` (the public header) plus every
`if` / branch in `c_src/src/driver.c`. There are **no** runtime options, no
global mode flags, no `#ifdef`s, no setters, and no opaque context struct in this
library, so the configuration surface is entirely **(entry point) x (input
shape)**.

## Axes the C actually branches on

**Axis E — entry point.** The `.so` exports five symbols. `driver.h` declares
only `driver()`, but the other four have external linkage and are therefore part
of the ABI surface a real consumer can (and the test harness does) call
directly. The call hierarchy, lowest level first:

```
printLine(const char*)      <- leaf, null-guarded
printIntLine(int)           <- leaf, unguarded
bad(float)                  -> printIntLine
goodG2B(void)     [static]  -> printIntLine        (constant divisor 2.0F)
goodB2G(float)    [static]  -> printIntLine | printLine
good(float)                 -> goodG2B, goodB2G
driver(float,float)         -> printLine, good, printLine, printLine, bad, printLine
```

Tests exercise the leaves (`printLine`, `printIntLine`) directly, then `bad`,
then `good` (which is the only way to reach the two `static` functions), then the
composed `driver` — not just the top-level convenience entry point.

**Axis P — pointer shape** (`printLine` only): NULL / empty string / short ASCII
/ long string (>4 KiB, crosses the stdio buffer) / string containing `%d`,
`%s`, `%n` (must be printed verbatim because it is an *argument* to `"%s\n"`,
never a format) / embedded newlines / non-ASCII (UTF-8) bytes / 0xFF high bytes.

**Axis I — int shape** (`printIntLine` only): `0`, `1`, `-1`, `INT_MAX`,
`INT_MIN`, random `i32`.

**Axis F — float shape** (`bad`, `good`, `driver`). The code distinguishes:

| class | why the C treats it differently |
|-------|--------------------------------|
| `+0.0` / `-0.0` | `goodB2G` guard rejects; `bad` divides -> `±inf` -> `cvttsd2si` UB |
| subnormal (`1e-45f`, `FLT_TRUE_MIN`) | rejected by guard; overflows the `(int)` cast in `bad` |
| `0 < |x| <= 1e-6` (normal tiny) | rejected by guard; overflows the cast in `bad` |
| exactly `1e-6f` | boundary: `(double)1e-6f < 1e-6`, so *rejected* |
| `nextafterf(1e-6f, 1)` | boundary: first value *accepted* by the guard |
| `|x|` where `100/|x|` straddles `INT_MAX` (`x ~= 4.656613e-8`... `x ~= 100/2^31`) | boundary of the `cvttsd2si` in-range check |
| ordinary magnitudes `1e-6 < |x| < 1e6` | guard passes, quotient in range, plain truncation |
| quotient with a fractional part (`3.0f`, `7.0f`, `-3.0f`) | truncation **toward zero**, both signs |
| exact quotients (`2.0f`, `4.0f`, `100.0f`) | no truncation |
| large `|x|` (`1e30f`, `FLT_MAX`) | quotient underflows to `~0` -> prints `0` |
| `±inf` | `100/inf = 0` — in range, prints `0`, *not* an error |
| NaN (quiet + signalling, both sign bits) | guard: unordered -> reject; `bad`: `cvttsd2si` UB |
| sign: every magnitude class also tested negative | `andps` strips the sign for the guard but not for the quotient |

**Axis N — argument independence** (`driver` only): the cross product of the
`goodData` class and the `badData` class, since they flow into two independent
guards and the output must interleave in a fixed order.

## Configuration table

One row per combination the C treats differently. Each row is driven with **many
randomized inputs** (seeded xorshift64\*, seed `0x243F6A8885A308D3`) in addition
to the named boundary values, and asserted byte-identical between the C `.so` and
the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1  | `printLine` | non-NULL short ASCII strings, randomized length 1..64 from a printable alphabet | [x] |
| 2  | `printLine` | empty string `""` | [x] |
| 3  | `printLine` | long string, 4 KiB and 64 KiB (crosses the stdio buffer boundary) | [x] |
| 4  | `printLine` | strings containing `printf` format directives (`%d %s %n %%`) — must be emitted verbatim | [x] |
| 5  | `printLine` | strings with embedded `\n`, `\t`, and high bytes 0x80..0xFF (non-UTF-8) | [x] |
| 6  | `printIntLine` | named extremes: `0`, `1`, `-1`, `INT_MAX`, `INT_MIN` | [x] |
| 7  | `printIntLine` | 512 randomized `i32` values over the full range | [x] |
| 8  | `bad` | ordinary magnitudes, randomized in `(1e-3, 1e3)`, both signs — exact + truncating quotients | [x] |
| 9  | `bad` | exact-quotient divisors `2, 4, 5, 10, 20, 25, 50, 100` and their negations | [x] |
| 10 | `bad` | truncating quotients `3, 6, 7, 9, 11, ...` and negations (toward-zero, both signs) | [x] |
| 11 | `bad` | tiny magnitudes `(0, 1e-6]` incl. subnormals -> quotient overflows the `(int)` cast | [x] |
| 12 | `bad` | divisors straddling the `INT_MAX` cast boundary: `100/2^31` +- a few ULPs, both signs | [x] |
| 13 | `bad` | large magnitudes `1e10 .. FLT_MAX` -> quotient truncates to `0`, both signs | [x] |
| 14 | `bad` | `+0.0`, `-0.0`, `+inf`, `-inf`, quiet NaN, negative NaN | [x] |
| 15 | `bad` | 1024 randomized bit patterns reinterpreted as `f32` (full domain incl. NaNs/subnormals) | [x] |
| 16 | `good` | accepted branch: randomized `|x| > 1e-6` -> `goodG2B` prints `50` then `goodB2G` prints the quotient | [x] |
| 17 | `good` | rejected branch: `0`, `-0`, subnormals, `(0,1e-6]`, NaN -> `50` then the message | [x] |
| 18 | `good` | guard boundary pair: `1e-6f` (reject) and `nextafterf(1e-6f, 1)` (accept), both signs | [x] |
| 19 | `good`, `bad` | region just above the guard (`|x|` in `(1e-6, 1e-5)`) where the quotient is ~1e8 — large but still in `int` range; also the first 64 floats above the guard, one ULP at a time. **Finding:** `100/2^31 ~= 4.66e-8` is *below* the guard threshold, so no guard-accepted value can overflow the cast; the test asserts that relationship holds rather than assuming it | [x] |
| 20 | `good` | `±inf` -> guard passes (`inf > 1e-6`), quotient `0` | [x] |
| 21 | `good` | 1024 randomized bit patterns reinterpreted as `f32` | [x] |
| 22 | `driver` | both arguments ordinary/valid, randomized in `(1e-3, 1e3)`, both signs | [x] |
| 23 | `driver` | cross product of the 12 named `goodData` classes x 12 named `badData` classes (144 combos) | [x] |
| 24 | `driver` | 2048 randomized `(f32, f32)` bit-pattern pairs over the full domain | [x] |
| 25 | all five | interleaving: a randomized *sequence* of calls to all five exports against the same open stdout, to catch buffering/ordering divergence | [x] |

## Binary / driver executable

`c_src/CMakeLists.txt` contains no `add_executable`, and `translation/Cargo.toml`
declares no `[[bin]]` (and there is no `src/main.rs`). Neither side builds a
program, so the "compare C and Rust stdout on the same inputs" gate has no binary
to run. It is instead satisfied at the library level: every test in
`tests/valid_paths.rs` and `tests/error_paths.rs` compares captured `stdout`
bytes, which is the only output this library produces.

## Verification evidence

Run `./run_all.sh` from the crate root. It rebuilds the C `.so`, then for each
profile (`debug`, `release`) x each feature set (`<default>`,
`--no-default-features`, `--all-features`) it runs `cargo build` (needed because
`cargo test` does **not** build `cdylib` artifacts) followed by
`cargo test -- --test-threads=1`, and finally diffs `nm -D` output.

Last run: **6 configurations, all pass** — 25 (valid) + 21 (error) + 5 (symbol /
harness self-check) = 51 tests each, and the `nm -D` diff is empty for both
profiles.

Bulk independent cross-check (outside the Rust test suite, driving both `.so`
files with `ctypes`): 200 000 randomized full-domain `f32` bit patterns through
`bad`, 200 000 through `good`, and 200 000 `(f32, f32)` pairs through `driver` —
about 24 MB of captured stdout, **byte-identical** in all three cases.

Note the row-25 style interleaving matters: `capture()` serialises access to fd 1
behind a mutex and calls `fflush(NULL)` before restoring the descriptor, because
both libraries write through the *same* glibc `stdout` FILE and it becomes fully
buffered when redirected to a file.

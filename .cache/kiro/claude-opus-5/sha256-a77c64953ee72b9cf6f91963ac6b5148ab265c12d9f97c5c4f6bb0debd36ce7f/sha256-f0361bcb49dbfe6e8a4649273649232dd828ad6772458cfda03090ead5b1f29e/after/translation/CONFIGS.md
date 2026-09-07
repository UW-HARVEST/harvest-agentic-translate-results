# CONFIGS.md — Phase A configuration-surface table

Derived mechanically from the C source and build system, not from guesses.

## Axis enumeration (evidence)

**Runtime options / modes / flags.** The complete public API, from
`c_src/include/driver.h`:

```c
void driver(int x);
```

There is exactly ONE public entry point and it *is* the lowest-level entry
point — there is no convenience wrapper layered over a lower-level API, no
context/handle object, no setter, no global state, and no option struct. Grep
for `if` / `switch` / `?:` / `#ifdef` in `src/driver.c` returns 0 functional
matches, so there is no flag the C branches on. Nothing to cross-product on this
axis.

**Compile-time configuration.** `c_src/CMakeLists.txt` defines no `option()`,
no `target_compile_definitions`, and no conditional sources — a single
`add_library(driver SHARED src/driver.c)`. `translation/Cargo.toml` has **no
`[features]` section**, so the only Cargo feature combination that exists is the
default (empty) one; `--no-default-features` and the default build are the same
build. (Verified in Phase D by enumerating features from `Cargo.toml`.)

**Binary executable.** `CMakeLists.txt` builds only the `SHARED` library and
`Cargo.toml` declares only `crate-type = ["cdylib"]` (no `[[bin]]`, no
`src/main.rs`). The project produces **no driver binary**, so the
"compare C and Rust binary stdout" item is not applicable; the equivalent
end-to-end check — comparing everything the library writes to `stdout` for the
same input — is what every row below asserts.

**Input shapes.** The single parameter is a scalar `int`. The shapes the code
actually distinguishes are therefore value-domain regions, driven by (a) the two
arithmetic steps `2*x` and `y += 300` and whether each wraps, and (b) the
`printf("%d\n", …)` conversion, which formats differently for negative / zero /
positive values and for each digit width. Those are the axes enumerated below.

## Configuration surface

One row per combination the C actually treats differently
(`2*x` wraps? × `+300` wraps? × sign/width of the printed result), plus the
FFI-shape rows. Every row is exercised through **both** `.so` exports with many
randomized inputs from a fixed seed (`tests/valid_paths.rs`), except where the
region contains a single value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | no options exist; `x = 0` (the single "empty"/identity value) → prints `300` | [x] |
| 2 | `driver` | `x` small positive, `1..=9` (each value, 3-digit output, no wrap) | [x] |
| 3 | `driver` | `x` random positive, `1..=1_000` (3–4 digit output, no wrap) | [x] |
| 4 | `driver` | `x` random positive, `1_000..=1_000_000` (5–7 digit output, no wrap) | [x] |
| 5 | `driver` | `x` random positive, `1_000_000..=1_073_741_673` (8–10 digit output, no wrap) | [x] |
| 6 | `driver` | `x` random small negative, `-149..=-1` → positive result `2..=298` (both `2*x` and sum stay in range; sign cancels) | [x] |
| 7 | `driver` | `x = -150` exactly → result `0` (zero formatting, no sign) | [x] |
| 8 | `driver` | `x` random negative with `2*x+300 < 0`, `-1_000..=-151` (negative output, minus sign, 1–4 digits) | [x] |
| 9 | `driver` | `x` random negative, `-1_000_000..=-1_000` (negative output, 4–7 digits) | [x] |
| 10 | `driver` | `x` random negative, `-1_073_741_824..=-1_000_000` (negative output, 7–10 digits, `2*x` at/near `INT_MIN` without wrapping) | [x] |
| 11 | `driver` | `x` random in `1_073_741_674..=INT_MAX`: `2*x` wraps **and/or** `y += 300` wraps → negative result from positive input | [x] |
| 12 | `driver` | `x` random in `INT_MIN..=-1_073_741_825`: `2*x` wraps negatively (result becomes large positive, then `+300` wraps again) | [x] |
| 13 | `driver` | `x` = full-range uniform random over `INT_MIN..=INT_MAX` (unbiased sweep across all of the above regions, 2000 values) | [x] |
| 14 | `driver` | `x` = every power of two and its negation, `±2^0 … ±2^31` (bit-pattern shape sweep across the whole width) | [x] |
| 15 | `driver` | `x` = `0x00`/`0xFF`-patterned words (`0x55555555`, `0xAAAAAAAA`, `0x7FFFFFFF`, `0x80000000`, `0xFFFFFFFF`, `0x0000FFFF`, `0xFFFF0000`) reinterpreted as `int` — FFI bit-pattern shapes | [x] |
| 16 | `driver` | repeated / sequential invocation shape: many calls in a row into the *same* loaded `.so` (state carry-over between calls; C keeps no state, Rust must not either) | [x] |
| 17 | `driver` | interleaved-call shape: C and Rust called alternately into the same shared `stdout` stream, asserting per-call output and no cross-call buffering difference | [x] |

## Verification result (Phase B)

All 17 rows are checked off. Each row maps to one test in
`tests/valid_paths.rs` (`cfg01_…` … `cfg17_…`), plus
`cfg_end_to_end_stdout_stream` for the whole-stream comparison. Randomized rows
use SplitMix64 with a fixed seed (`0x5EED_C0FF_EE12_3456`, xored with the row
number), 400–2000 inputs per row, and every row is asserted twice: once
per-call and once as a single batch inside one capture session (so a difference
in call ordering or stdout buffering would also fail).

Total randomized/enumerated inputs exercised across both `.so`s: ~6,900 per run.

`test result: ok. 18 passed; 0 failed` — under the default feature set, under
`--no-default-features`, and in both the `dev` (overflow checks ON) and
`release` profiles.

Run everything with `./verify.sh` from the crate root: it builds the C `.so`,
enumerates the feature combinations from `Cargo.toml`, and for each one rebuilds
the Rust `cdylib` (required — `cargo test` alone does not) and runs all suites,
finishing with the `nm -D` symbol diff.

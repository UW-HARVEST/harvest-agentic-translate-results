# CONFIGS.md — Configuration surface of `c_src/`

## How the axes were derived (mechanically, from the source)

```sh
# build-time configuration
grep -n -E 'option|add_definitions|target_compile_definitions|CMAKE_BUILD_TYPE' c_src/CMakeLists.txt   # -> nothing
grep -rn -E '#if|#ifdef|#ifndef|#else|#elif' c_src/                                                    # -> only the DRIVER_H_ include guard
grep -n '\[features\]' Cargo.toml                                                                      # -> nothing

# runtime configuration
grep -n -E 'if *\(|else|switch|case' c_src/src/driver.c
grep -n -E '^[a-z].*\(' c_src/include/driver.h
nm -D --defined-only c_src/build/libdriver.so
```

### Axis 1 — build-time configuration: **none**

`CMakeLists.txt` has no `option()`, no `add_definitions`, no
`target_compile_definitions`; the only preprocessor conditional in the whole C
tree is the `DRIVER_H_` include guard. `Cargo.toml` has **no `[features]`
table**. Hence the *complete* set of build configurations is a single one:

| # | configuration | cargo invocation |
|---|---------------|------------------|
| 1 | the only one | `cargo test`, `cargo test --no-default-features`, `cargo test --all-features` (all three resolve to the same, empty, feature set) |

### Axis 2 — runtime options/modes/flags: **none**

There is no global state, no init/config struct, no setter, no flags argument
anywhere. Every exported function is a pure `void f(scalar…)` that writes to
`stdout`. So the configuration surface is entirely made of **entry point ×
input shape**.

### Axis 3 — entry points (the FULL set, lowest level first)

From `nm -D` (5 exported) plus the two `static` helpers reachable through them:

| level | entry point | signature | note |
|-------|-------------|-----------|------|
| lowest | `printLine` | `void(const char*)` | leaf; the only pointer-taking function |
| lowest | `printIntLine` | `void(int)` | leaf |
| mid | `bad` | `void(float)` | calls `printIntLine` |
| mid | `good` | `void(float)` | calls `goodG2B` + `goodB2G` (both `static`) |
| top | `driver` | `void(float,float)` | the only symbol in `include/driver.h`; composes `printLine`+`good`+`bad` |

Tests must drive `printLine` / `printIntLine` / `bad` / `good` **directly**, not
only through the `driver` convenience wrapper.

### Axis 4 — input shapes the C code special-cases

* `const char*` (`printLine`, `driver.c:32`): NULL vs non-NULL; and — because
  the C compiles the body to `puts(line)` while the Rust calls
  `printf("%s\n", line)` — length (empty / 1 / short / longer than stdio's
  `BUFSIZ`), byte content (embedded `%` directives, embedded `\n`, `\t`,
  non-UTF-8 / high bytes `0x80..0xFF`, all 255 non-NUL byte values), and a NUL
  in the middle of a larger buffer (truncation point).
* `int` (`printIntLine`, `driver.c:40` `"%d\n"`): `0`, `±1`, negatives (sign
  emission), `INT_MIN` / `INT_MAX`, arbitrary bit patterns.
* `float` (`bad`, `good`, `driver`) — the code branches on it three ways:
  1. the `fabs(data) > 0.000001` guard in `goodB2G` (`driver.c:61`): **accepted**
     vs **rejected** vs **unordered** (NaN);
  2. IEEE division `100.0 / data` (`divsd`): normal / subnormal / zero /
     infinity / NaN operand classes, both signs;
  3. the `(int)` narrowing (`cvttsd2si`): quotient in `[-2^31, 2^31-1]` vs
     out-of-range/NaN → `INT_MIN`.
  Distinguished shapes: `+0.0`, `-0.0`, smallest subnormal, `MIN_POSITIVE`,
  values with `|x| < 100/2^31` (quotient overflows), the exact
  `cvttsd2si` boundary and one step either side, the `1e-6` guard boundary and
  one step either side, `±1`, `±2` (the `goodG2B` constant), values whose
  quotient is exactly an integer vs. needing truncation-toward-zero, `±FLT_MAX`,
  `±INF`, quiet NaN (both signs), signalling NaN, arbitrary 32-bit patterns.
* `driver` takes **two independent floats**, so its shape space is the *cross
  product* of the `good`-relevant classes and the `bad`-relevant classes.

## Configuration-surface table

One row per combination the C actually treats differently. Every row is
exercised with **many randomized inputs from that class** (fixed seed
`0x5EED_1234_5678_9ABC`, xorshift64\*, see `tests/common/mod.rs`), not a single
hand-picked value, and asserted byte-for-byte between the C `.so` and the Rust
`.so`.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| C1 | `printLine` | non-NULL, empty string (`""`) | [x] |
| C2 | `printLine` | non-NULL, every single-byte string `0x01..0xFF` (255 cases, incl. non-UTF-8) | [x] |
| C3 | `printLine` | random short ASCII printable strings, length 1..64 | [x] |
| C4 | `printLine` | random byte strings over the full `0x01..0xFF` alphabet, length 1..256 (non-UTF-8) | [x] |
| C5 | `printLine` | strings containing `printf` directives (`%s`, `%d`, `%n`, `%%`, `%1000000d`) | [x] |
| C6 | `printLine` | strings containing embedded `\n`, `\t`, `\r`, `\0`-terminated early inside a larger buffer | [x] |
| C7 | `printLine` | long strings: lengths 1023/1024/1025, 4095/4096/4097, 8192 (straddle stdio `BUFSIZ`) | [x] |
| C8 | `printLine` | called repeatedly in one capture window (buffering/interleaving across many calls) | [x] |
| C9 | `printIntLine` | `0`, `1`, `-1`, `9`, `10`, `-9`, `-10`, `99`, `100` (digit-count/sign boundaries) | [x] |
| C10 | `printIntLine` | `INT_MIN`, `INT_MIN+1`, `INT_MAX`, `INT_MAX-1` | [x] |
| C11 | `printIntLine` | 4096 uniformly random `i32` bit patterns | [x] |
| C12 | `printIntLine` | powers of ten and powers of two ±1 (all digit counts 1..10) | [x] |
| C13 | `bad` | `data` normal, quotient well inside `int` range, exact (`2.0`, `4.0`, `100.0`, `0.5`) | [x] |
| C14 | `bad` | `data` normal, quotient needs truncation toward zero, positive (`3.0`, `7.0`, `0.3`) | [x] |
| C15 | `bad` | `data` normal **negative**, quotient needs truncation toward zero (round-toward-zero direction differs from floor) | [x] |
| C16 | `bad` | `data` = ±`2.0f` (matches the `goodG2B` constant) and ±`1.0f` | [x] |
| C17 | `bad` | `data` = ±zero, ±INF, ±qNaN, sNaN (division special cases) | [x] |
| C18 | `bad` | `data` subnormal (smallest subnormal, random subnormals) → quotient overflows `int` | [x] |
| C19 | `bad` | `data` = ±`FLT_MAX`, ±`FLT_MIN` (`MIN_POSITIVE`), quotient underflows toward 0 / overflows | [x] |
| C20 | `bad` | `data` straddling the `cvttsd2si` range boundary: `100/2^31` and `100/(2^31-1)` ± 1 ULP, both signs | [x] |
| C21 | `bad` | 8192 random `f32` **bit patterns** (all classes incl. NaN payloads, subnormals) | [x] |
| C22 | `bad` | 8192 random finite `f32` log-uniform in `1e-20..1e20`, both signs | [x] |
| C23 | `good` | `data` **accepted** by the guard (`abs > 1e-6`), quotient in `int` range — random log-uniform `1e-6..1e6` | [x] |
| C24 | `good` | `data` **rejected** by the guard (`abs <= 1e-6`, incl. ±0, subnormals, random tiny) | [x] |
| C25 | `good` | `data` NaN (guard unordered → reject branch), both signs + sNaN | [x] |
| C26 | `good` | `data` at the guard boundary: `1e-6f`, `nextafter(1e-6f, ±INF)`, and their negations | [x] |
| C27 | `good` | `data` accepted but quotient out of `int` range — impossible for `abs>1e-6`? verified: `abs(data)>1e-6 ⇒ abs(100/data) < 1e8 < 2^31`, so this row asserts the *absence* of `INT_MIN` on the accept path over random inputs | [x] |
| C28 | `good` | `data` = ±INF (accepted by guard, quotient ±0 → prints `0`) | [x] |
| C29 | `good` | 8192 random `f32` bit patterns (both guard branches mixed) | [x] |
| C30 | `driver` | `goodData` accepted × `badData` normal (nominal end-to-end transcript) | [x] |
| C31 | `driver` | `goodData` accepted × `badData` = ±0 / NaN / INF / subnormal (reject-free good path + degenerate bad path) | [x] |
| C32 | `driver` | `goodData` rejected (±0, tiny, NaN) × `badData` normal | [x] |
| C33 | `driver` | `goodData` rejected × `badData` degenerate (both anomalies in one call) | [x] |
| C34 | `driver` | full cross product of the 24-value edge corpus × itself (576 combinations) | [x] |
| C35 | `driver` | 4096 random `(f32,f32)` bit-pattern pairs | [x] |
| C36 | mixed / composed | interleaved sequence of `printLine`, `printIntLine`, `bad`, `good`, `driver` calls in one capture window, order chosen by the seeded RNG (checks the composed pipeline + stdio buffering, invisible to per-function tests) | [x] |
| C37 | build config | every row above re-run under all 3 cargo feature invocations of Axis 1 × the `dev` and `release` profiles (6 builds; `release` differs because `[profile.release] panic = "abort"`) | [x] |
| C38 | `bad` | strided sweep of the **whole** 32-bit `f32` space (odd stride, 2^22 = 4 194 304 values by default, `DRIVER_SWEEP_LOG2` to widen): every exponent value × a spread of mantissas | [x] |
| C39 | `good` | the same strided sweep, mixing both guard branches in their natural proportions | [x] |
| C40 | `bad`, `good` | **exhaustive**: all 2^32 `f32` bit patterns, in chunks (env gated with `DRIVER_SWEEP_FULL=1`, ~40 min) — proves there is no `float` input at all on which C and Rust disagree | [x] |

## Status

All rows are implemented in `tests/differential_valid.rs` (see the
`CONFIGS.md row Cxx` comments) and pass byte-for-byte under every feature
combination; `tests/feature_matrix.sh` re-runs the whole suite for each of the
six builds of row C37.

Row C40 is skipped by default because of its runtime; it was run once to
completion over the full `0x00000000..=0xffffffff` range:

```sh
DRIVER_SWEEP_FULL=1 cargo test --offline --test differential_valid c40_full
```

### Runner note

The test targets use `harness = false`.  The differential comparison works by
redirecting file descriptor 1 around each call, which is process-global state;
the default multi-threaded libtest harness writes its own progress reports to
fd 1 from another thread and so corrupts the capture.  `common::run` executes
the cases strictly sequentially instead.  `cargo test <substring>` still filters
cases as usual.

### Negative control (the tests can actually fail)

The suite was validated by mutation: three deliberate bugs were injected into
`src/lib.rs` and each was caught.

| injected bug | rows that caught it |
|---|---|
| `value as c_int` (Rust's saturating cast) instead of the x86 `cvttsd2si` indefinite value | C14, C15, C20, C21, C22, C34, C36, E6, E8, E9, E10, E12, E23, E24, E25, G1, G3, G4 |
| `100.0f32 / data` (single precision) instead of C's `100.0 / (double)data` | C14, C15, C20, C21, C22, C23, C26, C27, C29, C30, C31, C32, C34, C35, C36, E12, E20, E21, E25, G3, G4 |
| `printLine` skipping empty strings + `printIntLine` printing `abs()` | 21 valid rows + 7 error rows |

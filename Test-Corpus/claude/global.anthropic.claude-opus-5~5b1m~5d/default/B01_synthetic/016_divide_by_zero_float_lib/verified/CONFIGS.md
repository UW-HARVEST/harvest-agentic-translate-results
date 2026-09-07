# CONFIGS.md — Phase A: configuration surface table (valid inputs)

Mechanically derived from `c_src/include/driver.h` (the full public API) and the
branches actually present in `c_src/src/driver.c`.

## Axes the C code branches on

**Runtime options / modes / flags:** none. The library has no global state, no
init/teardown, no setters, no `#ifdef`, no environment lookups. Grep confirms
there is no `static` mutable state and no preprocessor conditional other than
the header include guard:

```
$ grep -nE '#if|#ifdef|#ifndef|getenv|static [^v]' c_src/src/driver.c c_src/include/driver.h
c_src/include/driver.h:24:#ifndef DRIVER_H_
```

The only "configuration" is therefore the **input shape** at each entry point.

**Full set of public entry points** (all 5 exported symbols, i.e. including the
lowest-level primitives, not just the `driver` one-shot wrapper):

| level | entry point | signature |
|-------|-------------|-----------|
| lowest | `printLine`    | `void printLine(const char *)` |
| lowest | `printIntLine` | `void printIntLine(int)` |
| mid    | `bad`          | `void bad(float)` |
| mid    | `good`         | `void good(float)` — internally calls `static goodG2B()` + `static goodB2G(data)` |
| top    | `driver`       | `void driver(float, float)` — calls `good` then `bad` |

**Input-shape axes the code distinguishes:**

* `printLine`: NULL vs non-NULL (line 32); string length 0 / 1 / many /
  buffer-crossing; byte content ASCII / format-specifier / high-bit / embedded
  control chars.
* `printIntLine`: sign and magnitude as rendered by `%d` (0, +, −, `INT_MAX`,
  `INT_MIN`).
* float argument classes for `bad` / `good` / `driver`, because
  `100.0/data` then `(int)` truncation is value-dependent:
  normal ≥1, normal <1, quotient magnitude <1 (truncates to 0), negative
  (truncation toward zero), `|quotient|` near `INT_MAX`, out-of-range,
  ±0, ±inf, NaN, subnormal.
* `goodB2G` threshold branch: `fabs(data) > 0.000001` (line 61) — above,
  below, and exactly at the boundary.
* `driver`: cross product of the `good` branch × the `bad` value class, plus
  output *ordering* across the four `printLine` calls.

## Rows

Each row is exercised with many randomized inputs (fixed seed, xorshift PRNG in
`tests/common/mod.rs`) unless the row names one exact value, and both the C and
the Rust `.so` are called through `libloading` with `stdout` captured and
compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1  | `printLine` | non-NULL, length 1..64 random printable ASCII | [x] |
| 2  | `printLine` | non-NULL, length 0 (empty string) | [x] |
| 3  | `printLine` | non-NULL, length 1 (single byte, every value `0x01`–`0xFF`) | [x] |
| 4  | `printLine` | non-NULL, random bytes `0x01`–`0xFF` incl. invalid UTF-8 and control chars, length 1..256 | [x] |
| 5  | `printLine` | non-NULL, contains `printf` format specifiers (`%d %s %n %%`) as data | [x] |
| 6  | `printLine` | non-NULL, large string 4 KiB / 64 KiB / 1 MiB (crosses `stdio` buffer) | [x] |
| 7  | `printLine` | non-NULL, embedded newlines and `\r`, `\t` | [x] |
| 8  | `printLine` | NULL (guard branch — also `ERRORS.md` row 1) | [x] |
| 9  | `printIntLine` | random full-range `i32` (uniform over all 2^32 bit patterns) | [x] |
| 10 | `printIntLine` | boundary set: `0`, `1`, `-1`, `INT_MAX`, `INT_MIN`, `±10^k` | [x] |
| 11 | `bad` | normal float, quotient in `int` range, positive (random `0.001..1e6`) | [x] |
| 12 | `bad` | normal float, quotient in `int` range, negative — truncation toward zero | [x] |
| 13 | `bad` | `data` with `100.0/data` magnitude `<1` → truncates to `0` (`|data| > 100`) | [x] |
| 14 | `bad` | `data` such that quotient is within one ULP of `INT_MAX+1` (`≈4.6566128e-8`), both signs | [x] |
| 15 | `bad` | `data` out of range → quotient overflows `int` (subnormals, `1e-30f`..`1e-45f`) | [x] |
| 16 | `bad` | `data` special: `+0.0`, `-0.0`, `+inf`, `-inf`, quiet NaN, signalling NaN, `-NaN` | [x] |
| 17 | `bad` | fully random `f32` bit patterns (uniform over all 2^32 patterns, incl. subnormals/NaNs) | [x] |
| 18 | `good` | `fabs(data) > 0.000001` → division branch; random normal floats, both signs | [x] |
| 19 | `good` | `fabs(data) <= 0.000001` → message branch; `±0.0`, subnormals, `±1e-7`, NaN | [x] |
| 20 | `good` | exactly at the threshold: `1e-6f`, `1.0000001e-6f`, `9.99999e-7f`, `-1e-6f`, `nextafter` neighbours of `1e-6` | [x] |
| 21 | `good` | verifies the constant `goodG2B()` line (`data = 2.0F` → `50`) is emitted first, before the `goodB2G` line, for every above shape | [x] |
| 22 | `driver` | `good` division branch × `bad` in-range quotient (random pairs) | [x] |
| 23 | `driver` | `good` division branch × `bad` invalid (`0.0`, `inf`, `NaN`, subnormal) | [x] |
| 24 | `driver` | `good` message branch × `bad` in-range quotient | [x] |
| 25 | `driver` | `good` message branch × `bad` invalid — both defect paths at once | [x] |
| 26 | `driver` | fully random `(f32, f32)` bit-pattern pairs — exercises the whole composed pipeline and the 4 fixed `printLine` labels | [x] |
| 27 | *sequencing* | many mixed calls (`printLine`, `printIntLine`, `bad`, `good`, `driver`) in one captured `stdout` session — checks interleaving/buffering of the composed pipeline, not per-wrapper isolation | [x] |
| 28 | *feature combos* | default build, and `--no-default-features` (identical: no `[features]` exist) | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)` —
there is **no `add_executable`**, and `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` with no `[[bin]]`. There is no driver binary, so
the "compare C and Rust binary stdout" gate is not applicable; the equivalent
end-to-end coverage is row 26/27 (the `driver` top-level entry point driven
through the `.so`).

# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

The complete public API is one function (`c_src/include/driver.h`):

```c
void driver(float x);
```

Axes the C code actually branches on / distinguishes:

* **Runtime options / modes / flags:** none. There is no init function, no
  context struct, no global state, no setter, no environment variable read, and
  no `#ifdef` other than the `DRIVER_H_` include guard. `grep -c 'if\|switch'`
  over `src/driver.c` yields exactly one `if`-free `for` loop.
* **Cargo features:** `Cargo.toml` declares **no** `[features]` table, so the
  only feature combination is the default (empty) one. Verified by
  `cargo test --no-default-features` (see summary).
* **Public entry points:** exactly one, `driver`. It is also the *lowest-level*
  exported entry point — the only lower-level routine, `print_hex`, is `static`
  and deliberately not exported by either `.so` (see `SYMBOLS.md`).
* **Input shape:** a single `float`, so the "shape" axis collapses onto the
  4-byte object representation. The code paths the implementation
  distinguishes are therefore *value-classes of the 32-bit pattern* and the
  *per-byte* `%02x` formatting behaviour:
  * byte value `0x00` (must print `00`, not the empty string — `%02x` padding)
  * byte value `0x01..0x0f` (must print a leading `0`)
  * byte value `0x10..0x7f` (two digits, high bit clear)
  * byte value `0x80..0xff` (high bit set — zero-extension on variadic
    promotion, lowercase digits)
  * loop trip count is fixed at `sizeof(float)` == 4, so byte position 0..3
    (little-endian ordering must be preserved: LSB printed first)
  * float value class: zero, subnormal, normal, infinity, NaN
  * sign bit set / clear
* **Byte order / width:** the C casts `&x` to `unsigned char*` and walks it in
  ascending address order, i.e. **host (little-endian x86-64) order, LSB
  first**. The Rust must reproduce this, not the big-endian/"natural reading"
  order.
* **Statefulness:** stdout is a shared, process-global, libc-buffered stream.
  Both `.so`s use the *same* libc `printf`, so a further axis is whether output
  interleaves correctly across many calls and across the C/Rust boundary.

Cross-product of those axes, pruned to combinations the code actually
distinguishes:

## Configuration table

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `driver` (exported, lowest level) | canonical happy path: small positive normals (`1.0`, `2.5`, `3.14159`) | [x] |
| 2  | `driver` | positive zero `+0.0f` — all four bytes are `0x00`, exercises `%02x` zero padding on every byte | [x] |
| 3  | `driver` | negative zero `-0.0f` — only the sign byte differs; byte position 3 == `0x80` | [x] |
| 4  | `driver` | sign bit set, negative normals (`-1.0`, `-2.5`, `-1e30`) | [x] |
| 5  | `driver` | bit patterns whose bytes are all in `0x01..0x0f` (leading-zero nibble padding, e.g. `0x01020304`) | [x] |
| 6  | `driver` | bit patterns whose bytes are all in `0x10..0x7f` (no padding, high bit clear, e.g. `0x11223344`) | [x] |
| 7  | `driver` | bit patterns whose bytes are all in `0x80..0xff` (high bit set → zero- vs sign-extension, e.g. `0x80808080`, `0xdeadbeef`) | [x] |
| 8  | `driver` | mixed-byte patterns spanning all four byte classes in one value (e.g. `0x000f80ff`), verifies per-position independence | [x] |
| 9  | `driver` | little-endian byte-order probe: `0x00000001` vs `0x01000000` must print differently (`01000000` vs `00000001`) | [x] |
| 10 | `driver` | subnormals: smallest `0x00000001`, largest `0x007fffff`, negative subnormal `0x80000001` | [x] |
| 11 | `driver` | infinities: `+inf` `0x7f800000`, `-inf` `0xff800000` | [x] |
| 12 | `driver` | NaNs: quiet `0x7fc00000`, negative quiet `0xffc00000`, signalling `0x7fa00000`, min-payload `0x7f800001`, max-payload `0x7fffffff` | [x] |
| 13 | `driver` | magnitude extremes: `FLT_MIN` `0x00800000`, `FLT_MAX` `0x7f7fffff`, `-FLT_MAX` `0xff7fffff`, all-ones `0xffffffff` | [x] |
| 14 | `driver` | integral values that a decimal-formatting bug would survive but a hex-dump bug would not (`1<<k` for k in 0..31 reinterpreted as float) | [x] |
| 15 | `driver` | randomized property test, **uniform over all 2^32 bit patterns** (`u32::from_bits`), fixed-seed xorshift, 20 000 samples — includes NaNs/subnormals by construction | [x] |
| 16 | `driver` | randomized property test over *finite* floats produced from a random exponent+mantissa (fixed seed, 20 000 samples) | [x] |
| 17 | `driver` | exhaustive-ish sweep: all 2^16 patterns with the low 16 bits varying and high bits fixed, plus all 2^16 with high 16 varying (covers every byte value in every position) | [x] |
| 18 | `driver` | statefulness / stream interleaving: 1000 alternating C-then-Rust calls in one captured stdout region | [x] |
| 19 | `driver` | repeated identical call idempotence (same input 100×, output must be 100 identical lines) | [x] |
| 20 | `driver` | ABI shape: return value is `void`/no-return-slot, `f32` passed in `xmm0`, no callee-save/stack corruption (verified by calling with a full xmm/gp register footprint and re-reading a canary after the call) | [x] |
| 21 | `driver` | binary/driver executable comparison | N/A — `c_src/CMakeLists.txt` declares only `add_library(driver SHARED ...)`; no `add_executable`, so the project builds no binary. Nothing to compare. |
| 22 | `driver` | feature-combination axis | N/A/[x] — `Cargo.toml` has no `[features]`; the default (empty) combo is the only one, and it is what all rows above run under. `--no-default-features` re-run confirms identical results. |

## Status

All 22 rows are implemented in `tests/phase_b_configs.rs` (`cfg01_..cfg22_`),
each driving both `.so`s via `libloading` and comparing stdout byte-for-byte;
rows 5-7, 10, 12, 15-18 and 20 use fixed-seed randomized inputs
(2 000-20 000 samples each) and row 17 sweeps 2x65 536 bit patterns plus every
byte value in every byte position.

Result: `22 passed; 0 failed` under both cargo profiles (`dev`, `release`) and
both feature selections (default, `--no-default-features`) —
`./run_all_configs.sh` => "ALL CONFIGURATIONS PASSED".

Note on the harness: `cargo test` does **not** rebuild a `crate-type =
["cdylib"]` target, so the tests were initially comparing against a stale
`.so` (proven by a mutation that went undetected). `tests/common/mod.rs` now
builds the cdylib itself into a separate `--target-dir` before loading it, and
asserts both `.so`s are newer than their sources. After that fix, six
independent mutations of the Rust source were all detected.

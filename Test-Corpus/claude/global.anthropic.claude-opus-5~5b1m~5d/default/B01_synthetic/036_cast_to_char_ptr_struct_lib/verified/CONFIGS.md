# CONFIGS.md — Phase B configuration-surface table

## Axes derived mechanically from the C source

### Public entry points (`c_src/include/driver.h`)

```
void driver(int x);
```

Exactly ONE public entry point. There is no convenience/one-shot wrapper vs.
low-level split — `driver` *is* the lowest level exported entry point.
The only lower-level routine, `print_hex(unsigned char *, int)`, is `static`
(file-local, not in `nm -D`) and so is not callable by any consumer; it is
reached only through `driver`, always with `p = &house` and
`len = sizeof(house_t)`.

### Runtime options / modes / flags

```
$ grep -nE '#if|#ifdef|#define|switch|extern|static [^v]|global|set_|_config|flag|mode|option' c_src/src/driver.c
c_src/src/driver.c:34:static void print_hex(unsigned char *p, int len) {
```

**None.** There are no `#ifdef`s, no compile-time feature macros, no global
variables, no setters, no option/mode/flag parameters, and no initialisation
function. The library is fully stateless: `house` is a local, and the only
observable effect is `printf` to `stdout`.

Correspondingly, `translation/Cargo.toml` declares **no `[features]` table**,
so the only feature combination is the default (empty) one:

```
$ grep -A5 '\[features\]' translation/Cargo.toml   # -> no match
```

### Input shapes the code distinguishes

The single input is an `int` copied verbatim into `house.floors`, whose 4
bytes are then hex-dumped. The code has no value-dependent branch, so the
"shapes" that matter are the ones that change the *bytes* of the output:

* sign: zero / positive / negative
* magnitude: single-hex-digit (`< 16`, exercises the `%02x` zero-pad),
  one-byte, two-byte, three-byte, full four-byte
* byte-order-sensitive values (`0x01020304`, `0x000000ff`, `0xff000000`) — the
  dump reveals the platform's little-endian layout
* embedded zero bytes (`0x00FF00FF`, `1 << 8`, `1 << 16`, `1 << 24`)
* boundaries: `INT_MIN`, `INT_MIN + 1`, `-1`, `0`, `1`, `INT_MAX - 1`, `INT_MAX`
* powers of two and their neighbours (every single-bit position `1 << k`,
  `k = 0..31`, plus `(1<<k) - 1` and `-(1<<k)`)
* call count: one call vs. many calls in sequence (statelessness)

### Fixed (non-varying) shape facts that the output depends on

* `sizeof(house_t) == 16` on LP64: `int floors` @0, `int bedrooms` @4,
  `double bathrooms` @8 — **no padding** anywhere, so `{0}` leaves no
  indeterminate padding bytes and the dump is fully deterministic.
* `house.bedrooms` is always `3` → bytes 4..8 always `03000000`.
* `house.bathrooms` is always `2.0` → IEEE-754 `0x4000000000000000`, bytes
  8..16 always `0000000000000040`.
* Output is always exactly `2*16` hex chars + `'\n'` = **33 bytes**.

## Configuration table

Every row is driven through the `.so` exports of BOTH libraries and the
captured `stdout` is compared byte-for-byte. Rows marked *(random)* use a
seeded (`0xD3C0DE15EA5E`) xorshift64* generator, 512+ values per row, so the
row is only checked off once it passes across all randomized inputs.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `driver` | no options (none exist); `floors == 0` — the falsy / all-zero shape | `cfg_c1_zero` | [x] |
| C2 | `driver` | `floors == 1` — smallest positive; exercises `%02x` zero padding | `cfg_c2_one` | [x] |
| C3 | `driver` | `floors` in `1..16` — every single-hex-digit value, `%02x` pad path | `cfg_c3_single_hex_digit` | [x] |
| C4 | `driver` | `floors` in `0..256` — every one-byte value (low byte varies, high 3 bytes zero) | `cfg_c4_all_single_byte_values` | [x] |
| C5 | `driver` | `floors == 1 << k` for every `k` in `0..31` — every single-bit position, incl. the sign bit | `cfg_c5_every_single_bit` | [x] |
| C6 | `driver` | `floors == (1 << k) - 1` and `-(1 << k)` for `k` in `0..31` — bit-run boundaries on both signs | `cfg_c6_bit_run_boundaries` | [x] |
| C7 | `driver` | byte-order probes: `0x01020304`, `0x000000ff`, `0x0000ff00`, `0x00ff0000`, `0xff000000` (as `int`) — pins little-endian byte layout | `cfg_c7_byte_order_probes` | [x] |
| C8 | `driver` | embedded NUL bytes: `0x00FF00FF`, `0xFF00FF00`, `256`, `65536`, `1<<24` | `cfg_c8_embedded_zero_bytes` | [x] |
| C9 | `driver` | negative values, small magnitude: `-1 ..= -256` — two's-complement high-byte fill | `cfg_c9_small_negatives` | [x] |
| C10 | `driver` | extremes: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX` | `cfg_c10_extremes` | [x] |
| C11 | `driver` | *(random)* full-range uniform random `i32` — 4096 values, seeded | `cfg_c11_random_full_range` | [x] |
| C12 | `driver` | *(random)* random values biased to the extremes (near `INT_MIN`/`INT_MAX`/0) — 1024 values, seeded | `cfg_c12_random_near_boundaries` | [x] |
| C13 | `driver` | *(random)* random values with random bytes forced to `0x00` / `0xff` — 1024 values, seeded | `cfg_c13_random_sparse_bytes` | [x] |
| C14 | `driver` | many calls in sequence with *different* inputs (statelessness / no carry-over across calls); interleaved C-then-Rust and Rust-then-C orders | `cfg_c14_many_calls_stateless` | [x] |
| C15 | `driver` | many calls in sequence with the *same* input repeated (idempotence, no accumulation) | `cfg_c15_repeated_same_input` | [x] |
| C16 | `driver` (via the private `print_hex`) | structural invariants of every dump: length is always 33 bytes, bytes 4..8 always `03000000` (`bedrooms == 3`), bytes 8..16 always `0000000000000040` (`bathrooms == 2.0`), all chars lowercase hex | `cfg_c16_struct_layout_invariants` | [x] |
| C17 | `driver` | a *single* whole-process stream: all inputs dumped by C in one go vs. all by Rust in one go, compared as one blob (catches per-call buffering/flush differences) | `cfg_c17_bulk_stream_equivalence` | [x] |
| C18 | binary/driver executable | **N/A** — `c_src/CMakeLists.txt` declares only `add_library(driver SHARED ...)`; there is no `add_executable`, and `translation/Cargo.toml` has `crate-type = ["cdylib"]` with no `[[bin]]`. No binary stdout comparison applies. | documented | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` section and no optional
dependencies, so the complete set of feature combinations is:

| combo | command |
|-------|---------|
| default (= empty) | `cargo test --release` |
| explicit no-default | `cargo test --release --no-default-features` |
| all features (= empty) | `cargo test --release --all-features` |

All three are run by `run_all_features.sh` and must pass.

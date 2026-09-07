# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axes the C code actually distinguishes

**Runtime options / modes / flags:** the library has **none** — no setters, no
global state, no context object, no `#ifdef` other than the `DRIVER_H_` include
guard, no environment lookups. `CMakeLists.txt` defines no compile options and
builds exactly one target. `Cargo.toml` declares **no `[features]` table**, so
the only feature combination is the default (empty) one.

**Public entry points (full set, lowest level included):**

- `A. print_foo(const foo_t *)` — the LOW-LEVEL entry point. Exported with
  external linkage from the `.so`, though it is not in the public header. Takes
  a raw pointer to the packed struct, so it can be driven with **arbitrary
  8-byte storage** — including bit patterns `driver` can never produce
  (padding bits 6..7 set, garbage inter-field padding).
- `B. driver(unsigned, unsigned, bool, int)` — the convenience one-shot wrapper
  declared in `driver.h`; it packs a `foo_t` and calls `print_foo`.

**Input shapes the code special-cases (from the bit-field widths / types):**

| axis | distinct values the code treats differently |
|------|---------------------------------------------|
| `x` (`unsigned int : 2`) | `x & 0x3` → 4 classes: 0, 1, 2, 3. In range (0..3) vs. truncated (≥4). Printed with `%u`. |
| `y` (`unsigned int : 3`) | `y & 0x7` → 8 classes: 0..7. In range (0..7) vs. truncated (≥8). Printed with `%u`. |
| `b` (`bool : 1`) | `b & 0x1` → 2 classes: 0, 1. Canonical byte (0/1) vs. non-canonical (2..255). Printed with `%d`. |
| `z` (plain `int`, **not** a bit-field) | sign (negative / zero / positive), magnitude (single digit / multi digit), and the extremes `INT_MIN` / `INT_MAX`. Printed with `%d` → the only column whose *width* varies, so it drives the output byte length. |
| struct storage byte (only reachable via `print_foo`) | bits 0..5 significant; bits 6..7 padding, must be ignored. |
| inter-field padding bytes 1..3 (only via `print_foo`) | must be ignored. |
| element type / byte order / counts / formats / empty-one-many | **N/A** — the API has no arrays, buffers, lengths, counts or serialisation. |

**Output surface:** one `printf("%u %u %d %d\n", ...)` — a single line on
`stdout`. Differential comparison is byte-for-byte on captured stdout.

## Configuration table (pruned cross-product)

Each row is driven with **many randomized inputs (fixed seed, deterministic
xorshift64\*)** unless the row is inherently a single point.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | all-zero input: `x=0, y=0, b=0, z=0` (the minimal shape) | [x] |
| 2 | `driver` | **exhaustive** cross-product of every in-range bit-field combination `x∈0..3 × y∈0..7 × b∈0..1` with `z=0` (64 cases — the complete in-range option space) | [x] |
| 3 | `driver` | same exhaustive 64 bit-field combinations × randomized `z` (many seeds per combination) | [x] |
| 4 | `driver` | in-range `x`,`y`,`b`; `z` randomized over the **full `i32` range** (negative + positive + zero, varying digit widths) | [x] |
| 5 | `driver` | in-range `x`,`y`,`b`; `z` drawn from the boundary set `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` | [x] |
| 6 | `driver` | `x` truncating (`x ≥ 4`, randomized over full `u32`) × in-range `y`,`b` × randomized `z` | [x] |
| 7 | `driver` | `y` truncating (`y ≥ 8`, randomized over full `u32`) × in-range `x`,`b` × randomized `z` | [x] |
| 8 | `driver` | **both** `x` and `y` truncating simultaneously, randomized over full `u32` × randomized `z` | [x] |
| 9 | `driver` | `b` non-canonical byte (2..255, randomized) × randomized `x`,`y`,`z` | [x] |
| 10 | `driver` | fully unconstrained randomized fuzz: `x`,`y` full `u32`, `b` full `u8`, `z` full `i32` (all axes interacting at once) | [x] |
| 11 | `driver` | `x=UINT_MAX, y=UINT_MAX, b=0xFF, z=INT_MIN` and `z=INT_MAX` (all axes simultaneously at their extremes) | [x] |
| 12 | `print_foo` | storage byte = 0, `z = 0` (minimal low-level shape) | [x] |
| 13 | `print_foo` | **exhaustive** storage byte `0x00..=0x3F` (all 64 significant bit-field patterns, padding bits clear) × `z = 0` | [x] |
| 14 | `print_foo` | **exhaustive** storage byte `0x00..=0xFF` (all 256, i.e. every significant pattern × every padding-bit combination) × randomized `z` | [x] |
| 15 | `print_foo` | randomized storage byte × randomized garbage in inter-field padding bytes 1..3 × randomized `z` (padding must not leak into output) | [x] |
| 16 | `print_foo` | randomized storage byte × `z` from the boundary set `{INT_MIN, -1, 0, 1, INT_MAX}` | [x] |
| 17 | `print_foo` | fully randomized 8-byte struct image (all 8 bytes random — the most permissive low-level shape) | [x] |
| 18 | `driver` → `print_foo` composed pipeline | drive `driver` and, for the same logical input, drive `print_foo` with the storage image `driver` is specified to build; assert C-`driver`, Rust-`driver`, C-`print_foo`, Rust-`print_foo` all agree (catches bugs in the composition that per-function tests miss) | [x] |
| 19 | both, interleaved | many alternating `driver` / `print_foo` calls in one process without flushing between them, comparing the whole accumulated stdout stream (catches buffering / statefulness divergence) | [x] |
| 20 | binary driver executable | **N/A** — `CMakeLists.txt` builds only `add_library(driver SHARED ...)`; `Cargo.toml` declares only `crate-type = ["cdylib"]` and has no `[[bin]]`. There is no executable to compare stdout for. Verified by test `no_binary_target_exists`. | [x] |

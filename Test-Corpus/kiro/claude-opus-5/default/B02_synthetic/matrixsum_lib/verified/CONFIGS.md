# CONFIGS.md — configuration-surface table (valid inputs)

Derived mechanically from the branches `c_src/src/lib.c` actually takes. There
is no build-time configuration (`Cargo.toml` has no `[features]`, the C has no
`#ifdef`), so every axis below is a **runtime** axis.

## Axes the C code branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| A1 — permission flag bits | `FLAG_READ 0b0001`, `FLAG_WRITE 0b0010`, `FLAG_EXECUTE 0b0100`, `FLAG_DELETE 0b1000`, plus any bits outside that nibble (ignored) | `#define`s; `process_flags` `&`/`!!` chain; `matrixsum` `permissions |= ...` |
| A2 — zero vs non-zero-ness of each of the 4 `matrixsum` params | independently zero / non-zero (2^4 = 16 combinations), because only `!!param` reaches `permissions` while the raw value reaches `sum` | `matrixsum` lines `int validN = !!checkN;` |
| A3 — `DynamicArray` initial capacity | `0` (degenerate: `realloc(p,0)` path), `1` (grow immediately), `2` (what `matrixsum` uses), `3`, `4`, large, wrapping-large | `init_array` argument; `add_element`'s `size >= capacity` test |
| A4 — element count pushed | `0` (empty), `1`, exactly `capacity` (no growth), `capacity + 1` (one growth), many (repeated doubling) | `add_element` / `expand_array` |
| A5 — element values | zero, positive, negative, `INT_MIN`, `INT_MAX`, random (summation wraps) | `sum += arr->data[i]` |
| A6 — `matrix` global contents | factory values (checksum 916, below the `0xFFF` mask), values whose checksum exceeds `0xFFF`, values whose checksum is negative, all-zero | `calculate_matrix_checksum`; `matrix_sum & 0xFFF` in `matrixsum`; `matrix` is exported writable (`D`) so a real consumer can change it |
| A7 — entry point level | low level (`init_array`, `expand_array`, `add_element`, `free_array`, `process_flags`, `calculate_matrix_checksum`) vs the one-shot wrapper (`matrixsum`) | `include/lib.h` exposes only `matrixsum`; `nm -D` exposes all 7 functions + `matrix` |

## Rows — one per meaningful combination

Every row is exercised through **both** `.so`s via `libloading` with many
randomized inputs (fixed-seed xorshift PRNG, seed `0x5EED_1234_ABCD_F00D`), not
a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `process_flags` | exhaustive `flags` = 0..=15 (all 16 A1 flag subsets) | [x] |
| C2 | `process_flags` | `flags` with high/extra bits set outside the nibble: `0x10`, `0x20`, `0x7F`, `0xFFFF`, `0x7FFF_FFFF`, and 20 000 random `i32` | [x] |
| C3 | `process_flags` | negative `flags` (sign bit set): `-1`, `INT_MIN`, `INT_MIN+1`, random negatives | [x] |
| C4 | `calculate_matrix_checksum` | untouched factory `matrix` (expect 916 in both) | [x] |
| C5 | `calculate_matrix_checksum` + `matrix` (data symbol) | write random 12-int patterns into each lib's own `matrix` via `dlsym`, then compare checksums (small values) | [x] |
| C6 | `calculate_matrix_checksum` + `matrix` | `matrix` filled so the checksum overflows `i32` (all `INT_MAX`, all `INT_MIN`, mixed extremes) — wrapping summation | [x] |
| C7 | `init_array` + `free_array` | capacity 1, no elements added (A3 x A4=0); compare `size`, `capacity`, `data != NULL` | [x] |
| C8 | `init_array` + `free_array` | capacity 0 (degenerate) — compare `size`/`capacity`/non-NULL-ness | [x] |
| C9 | `init_array` + `add_element` x n + `free_array` | capacity `c` in 1..=8, `n == c` (fills exactly, **no** growth), random values incl. extremes; compare every element, `size`, `capacity`, all return codes | [x] |
| C10 | `init_array` + `add_element` x n + `free_array` | capacity `c` in 1..=8, `n == c + 1` (exactly one growth: `capacity` → `2c`) | [x] |
| C11 | `init_array` + `add_element` x n + `free_array` | capacity 1, `n = 1..=64` (repeated doubling: 1→2→4→…→64), random values | [x] |
| C12 | `init_array` + `add_element` x n + `free_array` | capacity 2 (the shape `matrixsum` uses) with `n = 0,1,2,3,4,5` — brackets the exact internal path of `matrixsum` | [x] |
| C13 | `expand_array` (called directly, low level) | freshly-`init_array`'d arrays of capacity 1..=8 expanded 1, 2 and 3 times in a row with no elements; compare return code + `capacity` each time | [x] |
| C14 | `expand_array` (direct) after partial fill | fill `k < capacity` elements, expand, then keep adding; verifies `size` is preserved and old contents survive `realloc` in both | [x] |
| C15 | full low-level pipeline | `init_array(c)` → interleaved `add_element` / `expand_array` in a randomized script (300 randomized scripts) → sum the buffer → `free_array`; compares the whole trace of return values, `size`, `capacity` and buffer bytes at every step | [x] |
| C16 | `matrixsum` | exhaustive A2: all 16 zero/non-zero patterns of the 4 params, with randomized non-zero magnitudes (200 randomized draws per pattern) | [x] |
| C17 | `matrixsum` | fully random `i32` params (50 000 draws) — value-dependent paths, wrapping `sum * 0x10` | [x] |
| C18 | `matrixsum` | boundary param values: `0`, `1`, `-1`, `INT_MAX`, `INT_MIN`, `0xFF`, `0x10`, `0xFFF` in the full 4-way cross product (8^4 = 4096 combinations) | [x] |
| C19 | `matrixsum` + mutated `matrix` | A6 x A2 interaction: mutate `matrix` (small / >0xFFF / negative / zero / extremes), then run `matrixsum` over randomized params — exercises `matrix_sum & 0xFFF` inside the composed pipeline | [x] |
| C20 | all 7 functions, mixed order | randomized whole-library session: mutate `matrix`, call `process_flags`, build and tear down arrays, call `matrixsum`, repeat — catches cross-call state leakage (e.g. a Rust `static mut` diverging from the C global) | [x] |

## How the rows are executed

`translation/tests/phase_b_valid.rs` contains exactly one `#[test]` per row
(`c1_…` … `c20_…`). Both libraries are opened with `libloading` — the C `.so`
from `c_src/build/` and the Rust `cdylib` from `target/<profile>/` — and every
call goes through `dlsym`'d exports, including the writable `matrix` data
symbol, so the `#[no_mangle]` wrappers are part of the system under test.
No Rust function is ever called directly.

Randomization uses a fixed-seed xorshift64\* PRNG
(`SEED = 0x5EED_1234_ABCD_F00D`, per-row salt), biased to hit `0`, `1`, `-1`,
`i32::MIN`, `i32::MAX` and small magnitudes as well as uniform values.

Each row asserts, for both libraries: every return value, the full
`DynamicArray` field snapshot (`size`, `capacity`, whether `data` is NULL) after
*every* mutating call, and the whole backing buffer element by element.

## Feature combinations

`Cargo.toml` declares no `[features]`, so the surface is a single build
configuration. `translation/combos.sh` derives the feature list from
`Cargo.toml` mechanically (so it stays correct if features are added later) and
runs `cargo check` + `cargo build` + the full suite for each combination; with
zero declared features it verifies the default build and
`--no-default-features`. Both pass, and the suite additionally passes under the
`dev` profile, where Rust's arithmetic overflow checks are enabled — confirming
the `wrapping_*` operations used to model C's signed/unsigned wraparound never
trip a debug-only panic.

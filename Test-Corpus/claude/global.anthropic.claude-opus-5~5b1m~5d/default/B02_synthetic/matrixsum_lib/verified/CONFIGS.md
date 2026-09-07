# CONFIGS.md — Configuration-surface table (Phase A, gates Phase B)

Derived mechanically from the branches in `c_src/src/lib.c`. The public header
`c_src/include/lib.h` declares only `matrixsum`, but the `.so` exports **8**
symbols (see `SYMBOLS.md`), so all 7 functions *and* the mutable `matrix` data
object are treated as public entry points and driven directly — not just the
one-shot `matrixsum` wrapper.

## Axes the C code actually branches on

* **A1 `process_flags` bitmask** — 4 independent `if`-free `!!(flags & FLAG_x)`
  tests on bits 0,1,2,3 ⇒ 16 distinct results; bits ≥4 and the sign bit are
  ignored (a 5th axis value: "unmodelled bits set").
* **A2 `matrixsum` validity quadruple** — `!!param1..!!param4` select 4 `|=`
  branches ⇒ 16 distinct `permissions` values, each feeding `process_flags`.
* **A3 `matrixsum` magnitude** — `sum * 0x10` and the two `+` are `int`
  arithmetic: small values vs. values that overflow/wrap.
* **A4 array capacity shape** — `init_array` capacity `0` / `1` / `2` (the value
  `matrixsum` uses) / large; `size >= capacity` is the growth branch in
  `add_element`, so capacity determines how many `expand_array` doublings occur.
* **A5 element count vs. capacity** — 0 elements, fewer than capacity, exactly
  capacity (boundary: next add triggers growth), many (repeated doublings).
* **A6 `matrix` global state** — `calculate_matrix_checksum` reads the exported
  mutable `matrix`; default contents vs. externally overwritten contents, and
  `matrixsum` masks the checksum with `& 0xFFF` so a checksum > 0xFFF is a
  distinct path.
* **A7 element values** — the `int` values stored/summed: zeros, negatives,
  `INT_MIN`/`INT_MAX` (wrapping sum).

There are no runtime option/mode/flag setters, no `switch`, and no `#ifdef` in
the C source; `Cargo.toml` declares **no `[features]`**, so the default feature
set is the only feature combination (Phase D notes this).

## Rows (each = one meaningful combination, checked off when it passes randomized inputs)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `process_flags` | all 16 exhaustive combinations of bits 0-3, mask = exactly the flag bits (A1) | [x] |
| 2  | `process_flags` | flag bits 0-3 combined with unmodelled high bits set (`\|0x10`, `\|0xF0`, `\|0x7FFF_FF00`) — 16×N (A1) | [x] |
| 3  | `process_flags` | negative inputs incl. `INT_MIN`, `-1`, and 4096 uniformly random `i32` (A1, seeded) | [x] |
| 4  | `calculate_matrix_checksum` | default `matrix` contents, called repeatedly (A6) | [x] |
| 5  | `calculate_matrix_checksum` + `matrix` | `matrix` overwritten through the exported data symbol with random `i32[3][4]` (incl. negatives / `INT_MAX` wrap), then checksum read; restored afterwards (A6, A7) | [x] |
| 6  | `init_array` | capacity `0` — `malloc(0)` shape; inspect returned `size`/`capacity`/NULL-ness (A4) | [x] |
| 7  | `init_array` | capacity `1` and `2` (the `matrixsum` value) (A4) | [x] |
| 8  | `init_array` | capacity random in `1..=1024` (A4, seeded) | [x] |
| 9  | `init_array` | capacity large-but-satisfiable (`1<<16`, `1<<20`) (A4) | [x] |
| 10 | `init_array` + `free_array` | allocate/free round-trip at each capacity in row 6-9 (A4) | [x] |
| 11 | `expand_array` | fresh `init_array(k)`, one `expand_array` ⇒ capacity `2k`; k = 1,2,3,7,64 (A4) | [x] |
| 12 | `expand_array` | fresh `init_array(k)`, repeated `expand_array` ×1..6 ⇒ capacity `k·2^n`, verifying contents preserved across each realloc (A4, A5) | [x] |
| 13 | `expand_array` | array already holding `n` elements, then expand, then read all `n` back (contents survive realloc) (A5, A7) | [x] |
| 14 | `add_element` | capacity `k`, add exactly `k` elements — no growth branch taken (A5) | [x] |
| 15 | `add_element` | capacity `k`, add `k` then one more — boundary `size == capacity` triggers exactly one doubling (A5) | [x] |
| 16 | `add_element` | capacity `1`, add 64 elements — 6 consecutive doublings; verify `size`, `capacity`, and every stored value (A4, A5) | [x] |
| 17 | `add_element` | capacity `2` (matrixsum shape), add 4 elements = exactly one doubling (A4, A5) | [x] |
| 18 | `add_element` | values = random `i32` incl. `0`, `INT_MIN`, `INT_MAX`, `-1` (A7, seeded) | [x] |
| 19 | `add_element` | full low-level pipeline: `init_array(cap)` → random mix of `add_element`/`expand_array` calls → read `size`/`capacity`/`data[..]` → `free_array`; 512 random programs (A4, A5, A7, seeded) | [x] |
| 20 | `matrixsum` | all 16 zero/non-zero patterns of the 4 params (A2) — non-zero components randomized | [x] |
| 21 | `matrixsum` | all params small positive (no overflow), 1024 random in `-1000..=1000` (A2, A3) | [x] |
| 22 | `matrixsum` | overflow shapes: `INT_MAX`, `INT_MIN`, `INT_MAX/2`, mixed signs — `sum` and `sum*0x10` wrap (A3) | [x] |
| 23 | `matrixsum` | 4096 uniformly random `i32` quadruples — full-range, exercises A2 × A3 × A7 jointly (seeded) | [x] |
| 24 | `matrixsum` + `matrix` | `matrix` overwritten so `calculate_matrix_checksum()` exceeds `0xFFF` and so it is negative, exercising the `& 0xFFF` mask branch, then `matrixsum` compared (A6) | [x] |
| 25 | interleaved | `matrix` mutation + `matrixsum` + direct low-level array pipeline in one sequence, asserting no cross-call state leaks between the two libraries (A4-A7) | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds `add_library(... SHARED src/lib.c)` only — there is
no `add_executable` and no `main()` anywhere in `c_src`. The Rust crate is
`crate-type = ["cdylib"]` with no `[[bin]]`. **No driver binary exists, so the
"compare stdout of the C and Rust binaries" gate is not applicable.**

## Harness self-check (does the suite actually detect divergence?)

Because "all tests pass" is only meaningful if the tests can fail, four
deliberate mutations were injected into `src/lib.rs`, the `cdylib` rebuilt, and
the suite re-run (all mutations reverted afterwards; `src/lib.rs` is byte-identical
to its pre-mutation state):

| mutation in the Rust source | detected by |
|---|---|
| `matrix_sum & 0xFFF` → `& 0xFFFF` | rows 24, 25 (2 tests failed) |
| `capacity * 2` → `capacity * 2 + 1` in `expand_array` | rows 11, 12, 13, 15, 16, 19 |
| `process_flags` stops testing `FLAG_DELETE` | rows 1, 2, 3, 20, 21, 22, 23 |
| `wrapping_mul` → `saturating_mul` in `init_array` | Phase C row 3 + the size-edge sweep |

4/4 killed, so the differential assertions are live rather than vacuous.

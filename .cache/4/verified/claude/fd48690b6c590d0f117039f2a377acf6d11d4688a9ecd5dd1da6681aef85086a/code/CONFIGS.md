# CONFIGS.md — Configuration-surface table (Phase A → gated Phase B)

Mirror of `ERRORS.md` for **valid** inputs. Axes were derived mechanically from
the branches the C actually takes in `c_src/src/lib.c`, not from what looks
important.

## Axes the C code branches on

**A. Runtime "options" / modes.** This library has no init/option struct; its
mode bits are the four `!!param` truth values in `matrixsum` and the four flag
bits in `process_flags`:
- `FLAG_READ 0b0001`, `FLAG_WRITE 0b0010`, `FLAG_EXECUTE 0b0100`,
  `FLAG_DELETE 0b1000` — set by `if (validN) permissions |= ...` (lines 148-151).
- `matrixsum` maps zero/non-zero of each of its 4 params onto one flag bit, so
  there are `2^4 = 16` distinct `permissions` states → 16 distinct `flag_count`
  contributions (`flag_count * 0xFF`). Non-zero *magnitude* is irrelevant to the
  flag but *is* relevant to `sum`, so the axis is (zero-pattern) × (magnitudes).
- `process_flags` is independently reachable and accepts any `int`; it branches
  on each of the 4 low bits and must ignore all other bits.

**B. Mutable global state.** `int matrix[3][4]` is an exported writable `D`
symbol read live by `calculate_matrix_checksum`, which feeds
`matrixsum` via `(matrix_sum & 0xFFF)`. Default contents vs. externally mutated
contents is a real configuration axis crossing both functions.

**C. Input shapes for the container API.** `init_array` / `expand_array` /
`add_element` / `free_array` are all public (`nm -D`) low-level entry points, not
just internals of `matrixsum`. Shapes the code distinguishes:
- capacity: `0`, `1`, `2` (what `matrixsum` uses), small, large, overflow-adjacent;
- fill level relative to capacity: `size < capacity` (fast path) vs.
  `size >= capacity` (line 78 → triggers `expand_array`, doubling);
- count of appends: `0`, `1`, exactly-capacity, capacity+1 (one expansion),
  many (repeated doublings: 1→2→4→8→…);
- element values: zeros, positives, negatives, `INT_MIN`/`INT_MAX`
  (summation wraps).

**D. Call hierarchy.** Lowest-level first: `process_flags` and
`calculate_matrix_checksum` (leaf, no allocation) → `init_array` →
`expand_array` → `add_element` → `free_array` → `matrixsum` (the composed
pipeline). Rows below exercise the low-level entry points **directly**, then the
composed pipeline, then the pipeline re-implemented externally from the
low-level parts to confirm the composition matches.

## Configuration rows

Every row is driven with **many randomized inputs** (fixed seed
`0x5EED_C0FFEE_1234` in a splitmix64, reproducible), through both `.so`s, and
compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `process_flags` | all 16 exhaustive low-nibble flag combinations (`0b0000`..`0b1111`) | [x] |
| 2 | `process_flags` | flag bits set **plus** reserved high bits (`v & 0xF` varied, upper 28 bits randomized) — unknown bits must be ignored | [x] |
| 3 | `process_flags` | fully random `int` over the whole 32-bit range, incl. negatives, `INT_MIN`, `INT_MAX`, `-1`, `0` | [x] |
| 4 | `calculate_matrix_checksum` | default (unmutated) `matrix` contents, called repeatedly (must be pure/stable) | [x] |
| 5 | `init_array` | `capacity = 1` (min useful): fresh handle, `size==0`, `capacity==1`, then `free_array` | [x] |
| 6 | `init_array` | `capacity = 2` (the value `matrixsum` uses) | [x] |
| 7 | `init_array` | randomized small/medium capacities `1..=4096` | [x] |
| 8 | `expand_array` | `capacity = 1` then expand → `capacity == 2`, existing element preserved | [x] |
| 9 | `expand_array` | randomized capacity, expanded **repeatedly** (2, 3, 4 successive doublings) with contents verified after each | [x] |
| 10 | `add_element` | `size < capacity` fast path only (fill below capacity, no expansion), randomized values | [x] |
| 11 | `add_element` | appends exactly `capacity` elements (last one lands at `size == capacity-1`, still no expansion) | [x] |
| 12 | `add_element` | appends `capacity + 1` → crosses line 78 exactly once (one doubling), randomized values | [x] |
| 13 | `add_element` | many appends from `capacity = 1` → repeated doublings (1→2→4→8→16→32…), up to 200 elements; `size`, `capacity` and the **initialized** buffer region `[0, size)` compared after every append (bytes past `size` are indeterminate `malloc`/`realloc` padding and are deliberately not compared) | [x] |
| 14 | `calculate_matrix_checksum` | `matrix` **mutated** through the exported data symbol: randomized 12 `int`s written into both `.so`s' `matrix`, checksum compared | [x] |
| 15 | `calculate_matrix_checksum` | `matrix` mutated to overflow-inducing extremes (`INT_MAX`/`INT_MIN` mixes) so the sum wraps | [x] |
| 16 | `matrixsum` + `matrix` | `matrix` mutated **and** `matrixsum` called → exercises `(matrix_sum & 0xFFF)` with negative and wrapped checksums | [x] |
| 17 | `matrixsum` | all 16 exhaustive zero/non-zero param patterns (each non-zero slot given a randomized non-zero value) → all 16 `permissions` states | [x] |
| 18 | `matrixsum` | all-zero params (`0,0,0,0`): `flag_count == 0`, `sum == 0` → result is pure matrix term | [x] |
| 19 | `matrixsum` | fully randomized 4-tuples over the whole `int` range (incl. negatives) — `sum * 0x10` wraps | [x] |
| 20 | `matrixsum` | boundary scalars: `INT_MIN`, `INT_MAX`, `-1`, `1`, `0x08000000` in every param position | [x] |
| 21 | `matrixsum` | called repeatedly in a loop (allocation churn: init/expand/free each call) to confirm no state leaks between calls | [x] |
| 22 | composed pipeline | `matrixsum` result vs. an **externally re-implemented** pipeline built from the low-level exports (`init_array(2)` + 4×`add_element` + manual sum + `process_flags` + `calculate_matrix_checksum` + `free_array`), compared C-vs-Rust *and* against the one-shot wrapper | [x] |
| 23 | cross-`.so` struct ABI | `DynamicArray` handle allocated by one `.so` inspected field-by-field (`data`, `size`, `capacity` at offsets 0/8/16) — confirms identical `repr(C)` layout and that `size`/`capacity` are `size_t` | [x] |
| 24 | interleaved multi-handle | several live `DynamicArray`s per `.so` used concurrently in interleaved add/expand order (independence of handles, no shared global state) | [x] |
| 25 | `matrix` (data symbol) + `calculate_matrix_checksum` + `matrixsum` | **pristine, never-written** state of the exported `matrix` initializer: raw 48 bytes compared C-vs-Rust before any test writes to the symbol, plus the checksum and `matrixsum` derived from it. Lives alone in its own test binary (`tests/differential_initial_state.rs`) because the other rows overwrite the shared global for determinism, which would otherwise mask an initializer mismatch | [x] |

## Feature combinations

`Cargo.toml` declares **no `[features]`** → the only valid combination is the
empty set; `--no-default-features` and the default build are byte-identical
configurations. `c_src/CMakeLists.txt` exposes no build options and the C source
contains no `#ifdef` configuration branches. Every row above was therefore run
under both invocations (Phase D) for completeness.

## Suite self-validation (mutation testing)

A green differential suite only means something if it can go red. `mutation_test.sh`
injects 12 deliberate defects into `src/lib.rs`, rebuilds the cdylib and re-runs
every suite, requiring each defect to be caught:

| mutation | caught by |
|----------|-----------|
| `matrixsum` multiplier `0x10` → `0x11` | 7 tests |
| `matrixsum` flag weight `0xFF` → `0xFE` | 7 tests |
| `matrixsum` mask `0xFFF` → `0xFFFF` | 3 tests |
| `add_element` grow test `>=` → `>` | 4 tests |
| `expand_array` doubling `*2` → `*3` | 6 tests |
| `init_array` wrapping → checked multiply | 3 tests |
| `process_flags` drops `FLAG_DELETE` | 10 tests |
| checksum loop `3x4` → `3x3` | 11 tests |
| `matrix` initializer `0xD4` → `0xD5` | 2 tests (row 25) |
| `matrix` initializer `0x01` → `0x02` | 2 tests (row 25) |
| NULL guards return `1` instead of `0` | 4 tests |
| `free_array` drops its NULL guard | abort/signal on `free_array(NULL)` |

Result: **12 caught, 0 missed**, and `src/lib.rs` is byte-identically restored
(md5 verified) afterwards.

Row 25 exists *because* of this exercise: the `matrix`-initializer mutations were
initially **MISSED**, since every matrix-touching test overwrote the symbol with a
harness constant before comparing. That blind spot is now covered.

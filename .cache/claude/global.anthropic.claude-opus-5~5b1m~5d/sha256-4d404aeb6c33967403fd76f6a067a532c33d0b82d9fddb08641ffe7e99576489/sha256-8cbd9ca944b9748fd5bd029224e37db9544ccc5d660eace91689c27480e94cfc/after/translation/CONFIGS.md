# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches the C in `c_src/src/lib.c` actually takes.

## Axes the C code branches on

**A. Public entry points** (all 5, low-level ones included, not just `betagamma`):
`create_block`, `allocate_block`, `free_block`, `compute_hash`, `betagamma`.

**B. Runtime options / flags.** There is no global state, no mode flag, no
`#ifdef` in the library. The only "options" are the function arguments:

| arg | branch it drives |
|-----|------------------|
| `DataBlock.flags` (`uint8_t`) | 4 independent mask tests: `&0x0F`, `&0xF0`, `&0xAA`, `&0x55` ⇒ **16 reachable contribution combinations** |
| `param1` | `block_size = (param1 % 10) + 5` ⇒ 19 residue classes (`-9..9`), of which 4 (`%10 ∈ {-9..-6}`) hit the error path; also an addend in `flag_contribution` |
| `param2` | `init_value` of `mem2`; addend for `&0xF0` blocks |
| `param3` | addend for `&0xAA` blocks |
| `param4` | addend for `&0x55` blocks |
| `count` (`allocate_block`) | loop trip count; `0` vs `1` vs many vs overflowing |
| `init_value` (`allocate_block`) | `init_value + i` computed in `size_t`, truncated to `int` ⇒ wrap behaviour |
| pointer ordering (`compute_hash`) | `<` / `>` / `==` on `mb->data` **and** on `mb` ⇒ 3 × 3 = 9 combinations, contributing `{0,100,200} + {0,10,20}` |

**C. Input shapes special-cased:** `count` = 0 / 1 / small / large / overflow;
`name` length = 0 / 1 / 31 / >31; `int` values = 0 / ±1 / small / `INT_MIN` /
`INT_MAX` / values that make the `int` sums and `result` overflow; the
`(sum1 - sum2) / 10` truncation-toward-zero for negative numerators.

## Rows (cross-product pruned to combinations the C distinguishes)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `create_block` | `name` = "" (len 0), random `id`, random `flags`; compares `id`/`name[0..1]`/`flags` | `cfg_c1_create_block_empty_name` | [x] |
| C2 | `create_block` | `name` len 1; full random `id` incl. `INT_MIN`/`INT_MAX`, `flags` 0..255 | `cfg_c2_create_block_len1` | [x] |
| C3 | `create_block` | `name` len 2..30 (typical), randomized ASCII incl. non-printable bytes | `cfg_c3_create_block_mid_len` | [x] |
| C4 | `create_block` | `name` len exactly 31 (+NUL fills the array exactly) — boundary shape | `cfg_c4_create_block_len31` | [x] |
| C5 | `create_block` | `flags` swept over the **entire** 0..255 domain, fixed name | `cfg_c5_create_block_all_flags` | [x] |
| C6 | `allocate_block` + `free_block` | `count == 0` (`calloc(0,4)` non-NULL, empty loop), random `init_value` | `cfg_c6_allocate_zero_count` | [x] |
| C7 | `allocate_block` + `free_block` | `count == 1`, random `init_value` incl. `INT_MIN`/`INT_MAX`/`-1` ⇒ checks `init_value + i` truncation | `cfg_c7_allocate_one` | [x] |
| C8 | `allocate_block` + `free_block` | `count` = 2..64 (many), `init_value` random ⇒ `size`, every element, wrap across `INT_MAX` | `cfg_c8_allocate_many` | [x] |
| C9 | `allocate_block` + `free_block` | `count` large but allocatable (4096, 65536, 1<<20) — large-shape / page-crossing | `cfg_c9_allocate_large` | [x] |
| C10 | `allocate_block` | `init_value = INT_MAX` with `count > 1` ⇒ `init_value + i` wraps past `INT_MAX` into negative | `cfg_c10_allocate_wrap_at_int_max` | [x] |
| C11 | `compute_hash` | `data1 < data2` **and** `mb1 < mb2` ⇒ 110 | `cfg_c11_compute_hash_9_orderings`, `cfg_c12_c13_c14_compute_hash_randomized_orderings` | [x] |
| C12 | `compute_hash` | `data1 < data2`, `mb1 == mb2` / `mb1 > mb2` ⇒ 100 / 120 | `cfg_c11_compute_hash_9_orderings`, `cfg_c12_c13_c14_compute_hash_randomized_orderings` | [x] |
| C13 | `compute_hash` | `data1 == data2` (aliased data ptr) × `mb1 <`/`==`/`>` `mb2` ⇒ 10 / 0 / 20 | `cfg_c11_compute_hash_9_orderings`, `cfg_c12_c13_c14_compute_hash_randomized_orderings` | [x] |
| C14 | `compute_hash` | `data1 > data2` × `mb1 <`/`==`/`>` `mb2` ⇒ 210 / 200 / 220 | `cfg_c11_compute_hash_9_orderings`, `cfg_c12_c13_c14_compute_hash_randomized_orderings` | [x] |
| C15 | `compute_hash` | operands built by real `allocate_block` calls (natural heap ordering, both argument orders) | `cfg_c15_compute_hash_real_heap` | [x] |
| C16 | `betagamma` | `param1 % 10 == 0` ⇒ `block_size == 5`; params randomized small | `cfg_c16_betagamma_residue_sweep` | [x] |
| C17 | `betagamma` | `param1 % 10 ∈ {1..9}` ⇒ `block_size` 6..14 — one row per residue, all covered | `cfg_c16_betagamma_residue_sweep` | [x] |
| C18 | `betagamma` | `param1 % 10 ∈ {-5..-1}` ⇒ `block_size` 0..4 (small valid; `-5` gives `block_size == 0`, which `calloc(0,4)` accepts) | `cfg_c16_betagamma_residue_sweep` | [x] |
| C19 | `betagamma` | `param1 % 10 ∈ {-9..-6}` ⇒ `block_size < 0` ⇒ error path, `-1` (valid-path row asserting the exact same sentinel) | `cfg_c16_betagamma_residue_sweep` | [x] |
| C20 | `betagamma` | all-zero params (`0,0,0,0`) — degenerate shape | `cfg_c20_betagamma_corner_vectors` | [x] |
| C21 | `betagamma` | `±1` on each param independently (8 vectors) | `cfg_c20_betagamma_corner_vectors` | [x] |
| C22 | `betagamma` | `INT_MAX`/`INT_MIN` in each param position (all 4 positions × 2 values) ⇒ `flag_contribution`/`result` signed wrap | `cfg_c20_betagamma_corner_vectors` | [x] |
| C23 | `betagamma` | params chosen so `sum1 - sum2` is **negative** and not a multiple of 10 ⇒ C truncation-toward-zero of `/10` | `cfg_c23_betagamma_negative_division` | [x] |
| C24 | `betagamma` | params chosen so `sum1 - sum2` is positive, and exactly a multiple of 10 (`param1 == param2`) ⇒ `0` | `cfg_c24_betagamma_equal_params` | [x] |
| C25 | `betagamma` | large-magnitude params (`±2^30`, `±2^24`) ⇒ `sum1`/`sum2` overflow inside the accumulation loops | `cfg_c25_betagamma_overflow_sums` | [x] |
| C26 | `betagamma` | fully randomized 4-tuples over the whole `i32` range, 4000 iterations, seeded | `cfg_c26_betagamma_random_full_range` | [x] |
| C27 | `betagamma` | randomized 4-tuples restricted to small values (−50..50) so every `flags` mask combination and every residue is hit densely | `cfg_c27_betagamma_random_small` | [x] |
| C28 | composed pipeline | `allocate_block` → `compute_hash` → element sums → `free_block` driven from the test (the low-level path `betagamma` composes), compared end-to-end | `cfg_c28_manual_pipeline` | [x] |
| C29 | `free_block` | double-use safety shape: allocate `n` blocks in a row, hash pairwise, free in reverse order; asserts C and Rust agree on all hashes | `cfg_c29_multi_block_pairwise` | [x] |
| C30 | ABI | `DataBlock`/`MemoryBlock` size & field offsets as observed through the FFI (sret return of `create_block`, `MemoryBlock` read back after `allocate_block`) | `cfg_c30_abi_layout` | [x] |
| C31 | `allocate_block` + `compute_hash` | **allocation ORDER** with n = 2/3/5/8 simultaneously live blocks × `count` ∈ {0,1,5,14,40}: the full n×n `compute_hash` matrix must match, which pins the whole address ordering rather than a single pair | `cfg_c28b_hash_matrix` | [x] |
| C32 | `allocate_block` | the C6–C10 shapes re-verified **out of process** (fresh, pristine heap), pinning `size` / `data` null-ness / every element again | `cfg_c9b_allocate_out_of_process` | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds **only** `add_library(... SHARED src/lib.c)` — there
is no `add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. There is therefore **no driver
binary** whose stdout could be compared; that completion-gate item is N/A.

# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

Mirror of `ERRORS.md`, for inputs the C **accepts**. Derived from the branches
`c_src/src/lib.c` actually takes, not from what looks important.

## Axes the C code branches on

There are **no** runtime option/mode/flag setters and no `#ifdef`s in this
library: it is four stateless leaf functions plus one composite. The axes are
therefore entirely *input shape* axes, one per branch in the source:

| axis | source of the branch | distinct classes |
|------|----------------------|------------------|
| A. entry point | 5 external-linkage definitions (`SYMBOLS.md`) | `create_block`, `allocate_block`, `free_block`, `compute_hash`, `betagamma` |
| B. `flags` bit pattern | the 4 masks at `lib.c:110,113,116,119` (`0x0F`, `0xF0`, `0xAA`, `0x55`) → 16 reachable mask-hit combinations | all 256 `uint8_t` values |
| C. `name` length | `strcpy` at `lib.c:43` into `char[32]` | 0, 1, mid, 30, 31 (exact fit), >31 (→ `ERRORS.md` row 10) |
| D. `count` | `calloc` + the `i < count` loop at `lib.c:60` | 0 (empty), 1 (one), 2..n (many), `>SIZE_MAX/4` (→ `ERRORS.md` row 2) |
| E. `init_value` | `init_value + i` computed in `size_t`, truncated to `int` (`lib.c:61`) | 0, positive, negative, `INT_MAX`, `INT_MIN` (wrap) |
| F. `data`-pointer order | `mb1->data < / > / ==` `mb2->data` (`lib.c:77-81`) | `<`, `>`, `==` |
| G. struct-pointer order | `mb1 < / > / ==` `mb2` (`lib.c:83-87`) | `<`, `>`, `==` |
| H. `param1 mod 10` | `block_size = (param1 % 10) + 5` (`lib.c:126`) | `0`→5 … `9`→14, `-1`→4 … `-5`→0, `-6..-9`→error |
| I. `param2/3/4` magnitude | `flag_contribution`, `result`, `sum1/sum2` signed wrap (`lib.c:110-123,139-147`) | 0, small ±, large ±, `INT_MAX`, `INT_MIN` |
| J. `(sum1 - sum2)` sign | `/ 10` truncation direction (`lib.c:147`) | positive, zero, negative |
| K. build profile | `Cargo.toml` (`dev`, `release` + `panic="abort"`) | 2 |
| L. cargo features | **none declared** → exactly 1 configuration (`default` == `--no-default-features`) | 1 |

Cross-product pruned to the combinations the code actually distinguishes:

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `create_block` | `name` length 0 (`""`), `id = 0`, `flags = 0x00` | [x] |
| 2  | `create_block` | `name` length 1, `id` random, `flags` random | [x] |
| 3  | `create_block` | `name` length 2..29 random, random `id`, random `flags` | [x] |
| 4  | `create_block` | `name` length 30 (one byte spare) | [x] |
| 5  | `create_block` | `name` length 31 — exact fit, NUL lands on the last byte | [x] |
| 6  | `create_block` | full 256-value sweep of `flags` with fixed `name`/`id` | [x] |
| 7  | `create_block` | `id ∈ {INT_MIN, -1, 0, 1, INT_MAX}` boundary sweep | [x] |
| 8  | `create_block` | non-ASCII / high-bit bytes in `name` (`char` is signed on x86-64) | [x] |
| 9  | `allocate_block` + `free_block` | `count = 0` (empty), `init_value = 0` — `calloc(0,4)` unique non-NULL | [x] |
| 10 | `allocate_block` + `free_block` | `count = 1` (one), `init_value` random | [x] |
| 11 | `allocate_block` + `free_block` | `count` random 2..4096 (many), `init_value = 0` | [x] |
| 12 | `allocate_block` + `free_block` | `count` random 2..4096, `init_value` large positive (`INT_MAX-k`) → wrap through `INT_MAX` inside the fill loop | [x] |
| 13 | `allocate_block` + `free_block` | `count` random 2..4096, `init_value` negative (`INT_MIN+k`) → `int→size_t` sign-extension then truncation | [x] |
| 14 | `allocate_block` + `free_block` | `count` random, `init_value = INT_MIN` exactly | [x] |
| 15 | `allocate_block` + `free_block` | `count` boundary sweep `{0,1,2,3,7,8,9,15,16,17,255,256,257,1023,1024}` × `init_value ∈ {0,1,-1,INT_MAX,INT_MIN}` | [x] |
| 16 | `free_block` | `mb` from `allocate_block(0, ..)` (non-NULL `data`, `size==0`) | [x] |
| 17 | `free_block` | `mb` from `allocate_block(n, ..)`, `n>0` — both frees run | [x] |
| 18 | `free_block` | heap `mb` hand-built with `data = NULL` → inner guard skips `free` | [x] |
| 19 | `compute_hash` | F=`<`, G=`<` → 110 | [x] |
| 20 | `compute_hash` | F=`<`, G=`>` → 120 | [x] |
| 21 | `compute_hash` | G=`==` (aliased struct) with a non-zero `data` → isolates the pointer term as exactly 0. NOTE: F=`<` together with G=`==` is **unconstructible** — one struct cannot hold two different `data` values — so the C's `100`-only outcome is unreachable and is verified as its reachable neighbour instead | [x] |
| 22 | `compute_hash` | F=`>`, G=`<` → 210 | [x] |
| 23 | `compute_hash` | F=`>`, G=`>` → 220 | [x] |
| 24 | `compute_hash` | F=`>` with adjacent structs → 210. NOTE: F=`>` together with G=`==` is **unconstructible** for the same reason as row 21; the `200` term is instead isolated by rows 22/23 differing only in the pointer term | [x] |
| 25 | `compute_hash` | F=`==`, G=`<` → 10 | [x] |
| 26 | `compute_hash` | F=`==`, G=`>` → 20 | [x] |
| 27 | `compute_hash` | F=`==`, G=`==` (aliased) → 0 | [x] |
| 28 | `compute_hash` | high-bit-set `data` values (unsigned pointer comparison), randomized `u64` pairs | [x] |
| 29 | `compute_hash` | `data` values from **real** `allocate_block` results, plus `size` fields varied (must be ignored) | [x] |
| 30 | `betagamma` | `param1 % 10 == 0` → `block_size = 5`; params small positive | [x] |
| 31 | `betagamma` | `param1 % 10 ∈ {1..9}` → `block_size = 6..14`; full sweep of the 9 residues | [x] |
| 32 | `betagamma` | `param1 % 10 ∈ {-1..-4}` → `block_size = 4..1`; full sweep | [x] |
| 33 | `betagamma` | `param1 % 10 == -5` → `block_size = 0`, `calloc(0)` path, sums both 0 | [x] |
| 34 | `betagamma` | all-zero params `(0,0,0,0)` | [x] |
| 35 | `betagamma` | `param2/3/4` at `INT_MAX` / `INT_MIN` with a valid `param1` residue → `flag_contribution`/`result` wrap | [x] |
| 36 | `betagamma` | `(sum1 - sum2)` strictly negative (J) → truncate-toward-zero of a negative quotient | [x] |
| 37 | `betagamma` | `(sum1 - sum2)` strictly positive and not a multiple of 10 | [x] |
| 38 | `betagamma` | `(sum1 - sum2) == 0` (`param1 == param2`) | [x] |
| 39 | `betagamma` | randomized `param1..param4` over the full `i32` range, 1200 cases (fork-isolated) | [x] |
| 40 | `betagamma` | randomized params restricted to the 15 valid `param1` residue classes × extreme `param2..4`, 900 cases | [x] |
| 41 | composite pipeline | `allocate_block` × 2 → `compute_hash` on the two results → sum both arrays → `free_block` both, driven from the test (the `betagamma` body re-implemented via the low-level exports) and compared against C's own low-level exports | [x] |
| 42 | all 5 | every row above re-run under build profile `dev` (K) | [x] |
| 43 | all 5 | every row above re-run under `--no-default-features` (L) | [x] |

Rows 30–43 that reach `compute_hash` through `betagamma` observe raw
allocator addresses and are therefore run under `fork()` isolation so both
libraries see byte-identical heap state (see `tests/differential.rs`).

## Row → test mapping

Rows 1–41 map one-to-one onto `tests/differential.rs::rowNN_*`
(`cargo test --test differential`; 45 tests). Rows 42–43 are the profile /
feature axes, driven by `./run_all.sh`, which reruns every suite per
configuration.

Randomization: each row that says "random" uses the fixed-seed splitmix64 PRNG
in `tests/common/mod.rs` (`Rng::new(<row-specific seed>)`), so failures are
reproducible. `Rng::interesting_i32` biases toward `INT_MIN`/`INT_MAX`/0/±1 and
near-boundary values as well as uniform 32-bit noise, so a row is exercised at
its edges and in bulk. Roughly 12 000 randomized differential calls run per
configuration.

## Why `betagamma` rows are run under `fork()`

`compute_hash` adds 100/200 based on `mb1->data < mb2->data` and 10/20 based on
`mb1 < mb2` — i.e. on raw allocator **addresses**. `betagamma` calls it on two
freshly `malloc`'d blocks, so its return value is a function of process heap
state, not only of its four arguments. Calling C and then Rust in one process
makes them disagree *even when C is compared against itself*: glibc's tcache
returns freed chunks LIFO, so the second caller sees `mem1`/`mem2` in the
opposite address order (observed: C=574 then C=464 for identical arguments).

`fork_both` therefore forks two children from the identical parent state, so both
libraries see byte-identical heap layouts. `fork_harness_is_deterministic`
guards the harness itself by comparing C against C and Rust against Rust.
`betagamma_arithmetic_core_matches_modulo_hash` additionally cross-checks 3 000
in-process calls, allowing only a difference explainable by the address-dependent
hash term — so the flag/sum/division arithmetic is verified independently of the
fork harness.

## Mutation check (does this suite actually have teeth?)

`./mutation_check.py` injects 12 plausible mistranslations into `src/lib.rs` one
at a time, rebuilds, and runs the suite; `src/lib.rs` is restored and verified
byte-identical afterwards. Result: **11 of 12 caught**.

| mutation | caught by |
|----------|-----------|
| `compute_hash` signed instead of unsigned pointer comparison | `row28_hash_high_bit_data_values` |
| `compute_hash` 100/200 constants swapped | 19 tests |
| `compute_hash` struct-pointer term dropped | 10 tests |
| `allocate_block` saturating instead of truncating fill | 11 tests |
| `allocate_block` off-by-one fill value | 6 tests |
| `betagamma` floor instead of truncating division | 8 tests |
| `betagamma` returns 0 instead of the `-1` sentinel | 2 tests |
| `create_block` writes `flags` before the `strcpy` | `row10_create_block_no_length_check_overflow_within_struct` |
| `betagamma` `0b00001111` mask widened | 11 tests |
| `allocate_block` calloc NULL check dropped | test binary SIGSEGVs |
| `free_block` NULL guard dropped | test binary SIGSEGVs |

The one **surviving** mutation is `block_size` zero-extended instead of
sign-extended (`as u32 as usize` instead of `as isize as usize`). This is a
limitation of the environment, not a gap that a better test could close: the only
reachable negative values of `(param1 % 10) + 5` are −1..−4, which sign-extend to
`SIZE_MAX-3..SIZE_MAX` (glibc's `nmemb*size` overflow check → `NULL`) and
zero-extend to `0xFFFF_FFFC..0xFFFF_FFFF` (a 16 GiB request). Both were measured
to return `NULL` on this host, so the two conversions are observationally
identical here. `row03_row04_int_to_size_t_conversion_boundary` pins both count
families for both libraries so the divergence would be caught on a host where
16 GiB can be committed. The shipped Rust is correct by construction
(`as isize as usize` reproduces C's sign extension).

# CONFIGS.md — Phase B configuration surface table

## Mechanical derivation

The C library has **no runtime options**: no flags, no modes, no setters, no
environment reads, no `#ifdef` feature gates (`grep -nE '#define|#if'` finds only
`ARRAY_SIZE` and `ITERATIONS`; `grep -ncE '\bif\b|\bswitch\b|\?'` returns 0 for
both files). So the configuration axes are exactly:

* **Axis E — entry point.** The complete public surface, lowest level first:
  1. `array` (exported 1 MiB `int[262144]` object — the *state* every operation
     reads and writes; drivable directly by an external caller via `dlsym`)
  2. `perform_expensive_operations()` — the lowest-level *function*; applies the
     arithmetic kernel `f` 100× to every element (`f^100`, element-wise)
  3. `long_exec(seed)` — the one-shot convenience wrapper: `srand` → fill with
     `rand()` → 2000 × `perform_expensive_operations` → XOR-reduce → `printf`
  4. **composition**: `perform_expensive_operations()` called k times in a row,
     which is what `long_exec` does internally and is *not* observable from a
     single-call test
* **Axis S — input shape of `array`** (the values the kernel branches on
  numerically even though there is no `if`): all-zero (`.bss` initial state),
  all-`INT_MIN`, all-`INT_MAX`, all-`-1`, small negatives, multiples of 7,
  `x % 7 == 0`, kernel fixed points / cycle members, uniformly random over the
  full `u32` bit space, `rand()`-generated (`0 .. 2^31-1`, i.e. bit 31 always
  clear — a genuinely different sub-domain from arbitrary `int`), single-slot
  perturbations, first/last slot boundaries.
* **Axis N — iteration count** (`k` calls ⇒ `n = 100k` kernel applications).
  `long_exec` pins `n = 200000`, but `perform_expensive_operations` lets a caller
  choose any `n ∈ 100·ℕ`. The Rust translation switches strategy at
  `LEARN_MIN_N = 8192` (naive below, cycle-accelerated at or above), and
  `long_exec` replaces the 2000-call loop with a single `f^200000`, so `n` must be
  swept **across** that threshold: `n = 100, 200, 700, 5000, 8100, 8200, 20000`
  and the real `n = 200000`.
* **Axis D — seed domain** for `long_exec`: `0` (glibc alias of 1), `1`, small,
  `255`, `65535`, `65536`, `8191/8192/8193`, `INT_MAX`, `2^31`, `2^31+1`,
  `UINT_MAX`, `UINT_MAX-1`, and a spread of arbitrary values.
* **Axis F — cargo features.** `Cargo.toml` declares one non-default feature,
  `debug-stats` (stderr diagnostics only). Combos: `{}` (default),
  `{debug-stats}`, plus `--all-features` and `--no-default-features`.
* **Axis O — call ordering / residual state**, since `array` is a mutable global
  shared by both functions and never reset except by `long_exec`'s fill loop.

Rows below are the cross-product of E × S × N (and E × D for `long_exec`), pruned
to the combinations the C actually treats differently. Every row is driven
through **both** `.so`s via `libloading` and compared byte-for-byte, with many
randomized inputs per row from a fixed seed (`tests/common/mod.rs`, SplitMix64
seeded per row) rather than one hand-picked value.

## Table

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | `array` write + read-back | write 262144 randomized `i32` (full `u32` bit space) through the `dlsym`'d object in each `.so`, read back, compare — proves both exported objects are the same size and fully addressable, incl. slots 0 and 262143 | [x] |
| 2  | `perform_expensive_operations` | `n = 100` (1 call), array = all zeros (untouched `.bss` shape) | [x] |
| 3  | `perform_expensive_operations` | `n = 100`, array = all `INT_MIN` (signed-overflow / `x<<1 == 0` / exact `/2` corner) | [x] |
| 4  | `perform_expensive_operations` | `n = 100`, array = all `INT_MAX` (overflow on first statement) | [x] |
| 5  | `perform_expensive_operations` | `n = 100`, array = all `-1` (arithmetic `>>` sign propagation) | [x] |
| 6  | `perform_expensive_operations` | `n = 100`, array = `-1..-262144` descending (all-negative domain: truncating `/`, sign-of-dividend `%`) | [x] |
| 7  | `perform_expensive_operations` | `n = 100`, array = `0,1,2,…` ascending (all-positive small domain) | [x] |
| 8  | `perform_expensive_operations` | `n = 100`, array = multiples of 7 and values with `x % 7 == 0` | [x] |
| 9  | `perform_expensive_operations` | `n = 100`, array = every value in `[INT_MIN, INT_MIN+K)` ∪ `(INT_MAX-K, INT_MAX]` ∪ `[-K, K]` (exhaustive boundary bands) | [x] |
| 10 | `perform_expensive_operations` | `n = 100`, array = 262144 uniformly randomized `i32` over the full `u32` bit space, 8 independent randomized batches | [x] |
| 11 | `perform_expensive_operations` | `n = 100`, array = `rand()`-shaped values (`0 .. 2^31-1`, bit 31 clear) — the sub-domain `long_exec` actually feeds it | [x] |
| 12 | `perform_expensive_operations` | `n = 100`, array = kernel fixed points / cycle members (values `v` with `f^L(v) == v`), discovered at test time from the C `.so` itself | [x] |
| 13 | `perform_expensive_operations` composed | `n = 200` (k = 2 calls), randomized full-range array — first composition step | [x] |
| 14 | `perform_expensive_operations` composed | `n = 700` (k = 7), randomized array — checks no per-call drift | [x] |
| 15 | `perform_expensive_operations` composed | `n = 5000` (k = 50), randomized array — mid-range composition depth | [x] |
| 16 | `perform_expensive_operations` composed | `n = 8100` (k = 81), randomized array — last `n` *below* the Rust `LEARN_MIN_N = 8192` strategy switch | [x] |
| 17 | `perform_expensive_operations` composed | `n = 8200` (k = 82), randomized array — first `n` *at or above* `LEARN_MIN_N`; also produces the deep/cycle-member values row 12 feeds back in | [x] |
| 18 | `perform_expensive_operations` composed | `n = 20000` (k = 200), randomized array — deep composition | [x] |
| 19 | `perform_expensive_operations` composed | `n = 200`, array = all-zero / all-`INT_MIN` / all-`INT_MAX` / all-`-1` (degenerate shapes *under composition*, where every element follows the identical orbit) | [x] |
| 20 | `long_exec` | `seed = 0` — full pipeline, `n = 200000`; compare stdout bytes **and** the full 1 MiB `array` after the call | [x] |
| 21 | `long_exec` | `seed = 1` — full pipeline; must equal the `seed = 0` output (glibc `srand` aliasing) | [x] |
| 22 | `long_exec` | `seed = 7` — full pipeline, stdout + full `array` compare | [x] |
| 23 | `long_exec` | `seed = 255` (8-bit boundary) | [x] |
| 24 | `long_exec` | `seed = 65535` and `seed = 65536` (16-bit boundary, either side) | [x] |
| 25 | `long_exec` | `seed = 8191 / 8192 / 8193` (values straddling the Rust strategy constant, as *seeds* rather than counts) | [x] |
| 26 | `long_exec` | `seed = 2147483647` (`INT_MAX`) | [x] |
| 27 | `long_exec` | `seed = 2147483648` (`2^31`, bit 31 set — negative if reinterpreted as `int`) | [x] |
| 28 | `long_exec` | `seed = 2147483649` (`2^31+1`) | [x] |
| 29 | `long_exec` | `seed = 4294967294` and `seed = 4294967295` (`UINT_MAX-1`, `UINT_MAX`) | [x] |
| 30 | `long_exec` | arbitrary spread of seeds — the recorded sweep covers **57 seeds** in total: `0 1 2 3 5 7 9 11 13 17 23 29 37 42 99 100 254 255 256 777 1000 4096 8191 8192 8193 12345 31337 32768 65535 65536 66666 424242 999983 1048576 7777777 16777216 88888888 305419896 123456789 555555555 987654321 1073741824 1234567890 1431655765 2000000000 2147483647 2147483648 2147483649 2718281828 2863311530 3000000000 3141592653 3221225472 4000000000 4026531840 4294967294 4294967295` | [x] |
| 31 | `long_exec` ordering | `long_exec(a)` then `long_exec(b)` in the *same* process/library instance — residual `array` state must not affect run `b` | [x] |
| 32 | `long_exec` idempotence | `long_exec(s)` twice in the same instance — identical stdout both times (`srand` fully resets PRNG state) | [x] |
| 33 | `long_exec` + `array` | `long_exec(s)`, then `perform_expensive_operations()` on the *residual* array (mixing the two entry points, feeding `f^200000` output back into `f^100`) | [x] |
| 34 | `perform_expensive_operations` before any `long_exec` | fresh library instance, kernel run on the pristine `.bss` (all-zero) array — no lazy init hidden in `long_exec` | [x] |
| 35 | driver binary | the project builds **no** binary/executable target (`Cargo.toml` has only `[lib] crate-type = ["cdylib"]`; `CMakeLists.txt` only `add_library`), so there is no stdout-of-binary comparison to make; the equivalent check is row 20/22's `printf` stdout capture through the FFI boundary | [x] |
| 36 | features | rows 1–35 re-run under all four feature combinations: `--no-default-features`, default, `--features debug-stats`, `--no-default-features --features debug-stats` (`tools/run_all.sh` enumerates them from `Cargo.toml` rather than hard-coding) | [x] |

## How the rows are executed

| test file | rows |
|-----------|------|
| `tests/symbols.rs` | Phase A / D symbol parity (`SYMBOLS.md`) |
| `tests/kernel_diff.rs` | 1–19, 34 — the low-level entry points, live C `.so` vs Rust `.so` |
| `tests/errors.rs` | `ERRORS.md` 1–20 |
| `tests/long_exec_diff.rs` | 20–33, 35 — full pipeline; the `#[ignore]`d tests are the live 8-min-per-seed C-vs-Rust runs |
| `src/fast.rs` unit tests | the `f^n` accelerator vs plain iteration, incl. `n = 200000` and 2000×100 composability |
| `tools/gen_c_refs.sh` | records C `.so` ground truth (stdout + 1 MiB `array` dumps) |
| `tools/run_all.sh` | drives everything across every feature combination |

Note on `long_exec` cost: one C `long_exec` call performs
2000 × 262144 × 100 = 5.24 × 10^10 kernel applications and takes ≈ 8 minutes, so
rows 20–33 compare the Rust `.so` against stdout/`array` **recorded from the C
`.so` under test** (`tests/cref/`), and additionally run live in-process
C-vs-Rust for a subset of seeds via the `#[ignore]`d tests. The recordings are
not hand-written constants — `tools/gen_c_refs.sh` regenerates them by
`dlopen`ing `c_src/build/liblong.so` and calling its exported `long_exec`.

The C library's arithmetic was also checked to be optimisation-independent
(`gcc -O0/-O1/-O2/-O3/-Os` all produce the identical XOR), so the wrapping
signed-overflow / arithmetic-shift / truncating-division semantics the Rust
reproduces are what the C actually compiles to at any optimisation level.

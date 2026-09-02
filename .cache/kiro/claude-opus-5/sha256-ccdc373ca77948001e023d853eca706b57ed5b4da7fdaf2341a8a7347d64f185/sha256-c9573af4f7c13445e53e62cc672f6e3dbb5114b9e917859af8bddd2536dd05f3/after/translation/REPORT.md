# Verification report — C `long` vs. Rust `long`

Ground truth: `c_src/src/long.c` + `c_src/include/long.h`, built by
`c_src/CMakeLists.txt` exactly as given (no `CMAKE_BUILD_TYPE`, therefore
**`-O0`**), gcc 11.5.0, x86-64, glibc. `c_src/` was not modified.

Everything below drives **both** shared objects through `dlopen`/`dlsym` only —
`array`, `perform_expensive_operations`, `long_exec`. No Rust function is called
directly anywhere in the test suite, so the `#[no_mangle]` wrappers and the
exported `.bss` object are themselves under test.

## What the library is

68 lines, one translation unit. `f(x)` is the inner loop body

```c
x = x * 3 + 7;  x = x ^ (x >> 3);  x = x - (x << 1);  x = x / 2 + x % 7;
```

`perform_expensive_operations()` applies `f^100` to each of 262144 `int`s in the
exported global `array`. `long_exec(seed)` does `srand(seed)`, fills the array
with `rand()`, calls `perform_expensive_operations()` 2000 times, and prints the
XOR of the result with `printf("%d\n", …)`. One `long_exec` is
2000 × 100 × 262144 ≈ 5.2·10^10 applications of `f`, ~470 s of CPU at `-O0`.

The Rust translation reproduces `perform_expensive_operations` literally, but
`long_exec` does **not** run the nested loop: `src/fast.rs` computes the same
`f^200000` by exact function-iteration algebra (Brent cycle finding + a memo of
"how this value relates to its cycle"), which is why the Rust `long_exec` takes
0.4 s instead of 470 s. That accelerator is the whole risk surface of this
translation, and most of the effort below is aimed at it.

## Phase A — surface map

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | `nm -D` on both objects; 3 C symbols, all exported by Rust; `array` 0x100000 bytes on both sides; 0 missing, 0 undefined non-libc |
| `ERRORS.md` | 20 rows. The greps establish the C has **no** error codes, sentinels, asserts, null checks or range checks, so the rejection surface is the implicit numeric edge conditions (signed overflow, negative shifts, truncating division, sign-of-dividend remainder, seed extremes). Rows 14/15/19/20 are recorded as unreachable/not-applicable with the reason. |
| `CONFIGS.md` | 40 rows. The C has no runtime options at all, so the axes are: entry point, `k` = number of `perform_expensive_operations` calls, array data shape, and `seed`. |

No C source was left untranslated — `c_src` has exactly one `.c` file, and every
non-`static` definition in it has a Rust counterpart.

## Phase B — valid-path results

| what | scale | result |
|------|-------|--------|
| `perform_expensive_operations` (`f^100`), **exhaustive** | all 2^32 `int` values, as 16384 disjoint 262144-wide windows, whole-array checksum compared | **0 mismatches** (`tools/sweep.sh`, 24 shards, all `DONE`) |
| `perform_expensive_operations` composed, `k = 0 … 83` | 24 rows × 262144 values: zeros, all-ones, all `INT_MIN`/`INT_MAX`, 37-value sentinel tile, three contiguous windows (around 0, at `INT_MIN`, at `INT_MAX`), 6 random fills, sparse, values already on cycles of `f`, `k=0` no-op, state carry-over, randomized `seed × k` matrix | all byte-identical |
| `long_exec` end-to-end | **42 seeds**, full C runs (~470 s CPU each, run in parallel): exact `printf` bytes **and** the full 1 MiB final array compared with `cmp` | all identical |
| accelerated vs. naive through the FFI | `srand(s)` + `rand()` fill + 2000 naive `pxo` calls vs. `long_exec(s)`, for 9 seeds; and the same naive route on the **C** library for seed 42 | all identical |
| stream parity | C writes nothing to stderr; default-feature Rust matches on both fds | identical |

The exhaustive sweep is the load-bearing result: it makes the Rust `f^100`
provably equal to the C `f^100` on the entire input domain, so every composition
`f^(100k)` agrees too. The accelerated-vs-naive checks then pin `src/fast.rs` to
that same composition, and the 42 direct C runs confirm the whole pipeline
end to end.

Negative controls were run to confirm the harness can actually fail: the
checksum comparison distinguishes `pxo:1` from `pxo:2`, and the test suite caught
three real problems during development (a wrong `dlsym` idiom for the data
symbol, a reference-fixture mix-up, and the stderr difference introduced by the
`debug-stats` feature).

Incidental finding from the accelerator's own diagnostics (`--features
debug-stats`): a `long_exec` run resolves into 7–9 distinct cycles of `f`, of
lengths `[25, 47, 84, 109, 126, 631, 3166, 11991, 52330]`, with ~1.7 M memo
entries. The final image contains only ~62 k distinct values.

## Phase C — error-path results

The C library has no error surface to compare, which the table documents
mechanically rather than assuming. Every row that *is* reachable has a
differential test in `tests/errors.rs` (14 tests, all passing):

* signed-overflow bands around `INT_MIN`, `INT_MIN/2`, `INT_MIN/3`, `INT_MAX/3`,
  `INT_MAX/2`, `INT_MAX` (±3 each);
* negative arithmetic right shift and negative left shift (all of `-64..0` plus
  `INT_MIN`, `INT_MIN+1`, `INT_MIN+7`, `-2^30`, `-2^20`);
* division truncation toward zero and remainder sign, over `-70..=70`
  (every residue class mod 2 and mod 7, both signs);
* `INT_MIN / 2` does not trap, and `INT_MIN / -1` is unreachable because both
  divisors are literals;
* seed boundary sweep across the FFI: `0, 1, 2, INT_MAX, INT_MAX+1, 32767/32768,
  65535/65536, 2147483646, UINT_MAX-1, UINT_MAX`, plus `-1 as u32`,
  `INT_MIN as u32`, and 64-bit values truncated to 32 bits, requiring equal bit
  patterns to give equal output;
* `k = 0` leaves the array bit-unchanged in both libraries;
* pristine `.bss`: both `array` objects are all-zero at load, in a dedicated
  test binary that owns a fresh process (`tests/bss_initial.rs`).

Two documented findings rather than assumptions:

* `srand(0)` aliases `srand(1)` in glibc, so seeds 0 and 1 produce identical
  output — confirmed on the C side from its own reference dumps.
* A negative printed value is **unreachable**: after `f^200000` every element
  lies in `[-1073734582, -536871525]`, so all 262144 elements have bit 31 set
  and the even count cancels it. `%d` vs. `%u` is therefore not observable
  through this API. `ERRORS.md` row 18 records this instead of claiming a test
  for a case that cannot occur.

## Phase D — symbol parity and feature combinations

`[features]` declares exactly one optional feature and no defaults, so the
complete power set is `{}` and `{debug-stats}`. `tools/check_features.sh`
enumerates it from `Cargo.toml` (not hard-coded), and for each combination
rebuilds the `cdylib` into its own target directory, re-checks `nm -D` parity and
the `array` object size, and runs the whole suite against *that* build.

```
features declared : debug-stats
combinations      : 2
### <no features>   : 0 missing symbols, array size 0x100000, all tests pass
### debug-stats     : 0 missing symbols, array size 0x100000, all tests pass
ALL FEATURE COMBINATIONS PASSED
```

The one intentional behavioural difference from the C is that `debug-stats`
writes diagnostics to **stderr**. It is opt-in, off by default, and does not
touch stdout or the array; `stderr_parity` asserts empty stderr in the default
configuration and diagnostics-only in the feature build.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing and 0 undefined non-libc symbols in Rust.
- [x] Phase B: every `CONFIGS.md` row passes across randomized inputs — plus an
      exhaustive sweep of all 2^32 inputs for the low-level entry point.
- [x] Phase C: every reachable `ERRORS.md` row has a passing differential test;
      the unreachable rows are documented with the reason they cannot occur.
- [x] Both hold under every feature combination.

**No divergence was found between the C and Rust libraries.** No change to
`src/lib.rs` or `src/fast.rs` was needed; the only edits to `translation/` were
the `libloading`/`libc` dev-dependencies, the three Phase A artifacts, and the
test suite.

## Layout

```
translation/
  SYMBOLS.md  ERRORS.md  CONFIGS.md  REPORT.md
  tests/harness/mod.rs      dlopen both .so, fd capture, splitmix32, FNV-1a, compare
  tests/bss_initial.rs      pristine .bss, own process
  tests/differential.rs     Phase B rows 1-20 (24 tests)
  tests/errors.rs           Phase C (14 tests)
  tests/long_exec_diff.rs   Phase B rows 21-35 + stream parity (7 + 2 ignored)
  tests/reference/          cached C output: 18 full 1 MiB dumps + 42 stdout captures
                            + 42 FNV-1a fingerprints + 3 composite rows
tools/                      C harnesses (outside c_src, which is untouched)
  driver.c                  dlopen, long_exec(seed), dump array
  runner.c                  op-sequence runner: fill / pxo / exec / dump / hash / xor
  fnv.c                     FNV-1a of a file
  sweep.sh                  exhaustive 2^32 sweep, shardable
  gen_reference.sh          regenerate tests/reference from the C .so
  check_features.sh         Phase D: enumerate features, rebuild, nm parity, test
  verify_unit_test_vectors.sh  check src/lib.rs's hard-coded vectors against the C .so
```

## Reproducing

```bash
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../tools && gcc -O2 -o driver driver.c -ldl && gcc -O2 -o runner runner.c -ldl && \
  gcc -O2 -o fnv fnv.c
./gen_reference.sh          # ~8 min wall, 21 C long_exec runs in parallel
./check_features.sh         # both feature combinations, ~10 min
./verify_unit_test_vectors.sh
for i in $(seq 0 23); do ./sweep.sh $i 24 sweepout/shard.$i.log & done; wait   # exhaustive, ~20 min
cd ../translation && cargo build --release && \
  cargo test --release -- --ignored --test-threads=1   # the two slow cross-checks
```

`cargo build --release` must precede `cargo test`: `cargo test` builds the test
binaries but not the `cdylib`, so the harness refuses to run against a
`liblong.so` older than `src/*.rs`.

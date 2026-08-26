# Verification report — C ↔ Rust differential testing

The C code in `c_src/` is the ground truth. The Rust code in `src/` must produce
byte-identical results. Everything below is verified by comparing the two
**shared objects** through `libloading`/`dlopen`; no Rust function is ever
called directly, so the `#[unsafe(no_mangle)] extern "C"` export wrappers are
themselves under test.

## Library under test

| | |
|---|---|
| C source | `c_src/include/lib.h` (20 lines), `c_src/src/lib.c` (58 lines) |
| C `.so` | `c_src/build/libtranslated_rust.so` (CMake, GCC 11.5.0, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`) |
| Rust source | `src/lib.rs` (150 lines) |
| Rust `.so` | `target/release/libflac_validate_lib.so` **and** `target/debug/libflac_validate_lib.so` (both are loaded and compared against C in every test) |
| Public API | `int flac_validate(tflac *t)`, `tflac_u32 tflac_size_memory(tflac_u32)` |

## Phase A — surface maps

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | `nm -D` inventory of both `.so`s + `struct tflac` ABI layout table |
| `ERRORS.md` | 11 `return -1` rejection rows (one per `return -1` in `lib.c`) + 11 generic-boundary rows |
| `CONFIGS.md` | 10 branch axes derived from the `if`/`while` statements, expanded into 40 valid-configuration rows |

**Completeness:** `c_src/CMakeLists.txt` compiles exactly one translation unit
(`src/lib.c`), and `c_src/` contains no other C source. No module was skipped by
the translation, so no Phase-A "translate the missing C source" work was needed.

## Phase B — valid-path differential tests (`tests/diff_valid.rs`, 40 tests)

Every one of the 40 `CONFIGS.md` rows has a test that drives both `.so`s in that
configuration with many randomized inputs (fixed-seed `xorshift64*`) and
compares the `int` return **and all 28 struct bytes, tail padding included**.

Three tests additionally enumerate the decision space *exhaustively* rather than
sampling it:

* `cfg_exhaustive_blocksize_x_partition_orders` — 8 910 720 configurations
  (every `blocksize` 16..65535 × every legal `(min_po, max_po)` pair)
* `cfg_exhaustive_mode_x_channels_x_bitdepth_x_rice` — 2 031 616 configurations
  (all 256 `channel_mode` × 8 `channels` × 32 `bitdepth` × 31 `max_rice_value`)
* `cfg_exhaustive_size_memory_dense_windows` — every `u32` in `0..2^21`, ±4096
  around every power of two, and the top 4096 `u32` values

## Phase C — error-path differential tests (`tests/diff_errors.rs`, 24 tests)

Every `ERRORS.md` row (11 rejections + precedence + G1..G10) has a test that
constructs the exact invalid input and asserts C and Rust return the **same
sentinel** and leave the struct in the **same byte state**. Notable cases:

* Rows 1–8 assert the struct comes back byte-identical (C rejects before any
  write).
* Rows 9–11 pin down the **partial mutations** that precede the rejection
  (`channel_mode` zeroed, `max_rice_value` auto-filled) — a return-code-only
  test would miss these.
* **G1 (NULL)**: `flac_validate` has no null check and dereferences `t`
  unconditionally, so the call faults. Verified in a child process: both the C
  `.so` and the release Rust `.so` die with **SIGSEGV** (signal 11), no exit
  code. (The *debug* Rust `.so` is excluded from this one test only, because
  `debug_assertions` turns a null-reference deref into a Rust diagnostic panic;
  that is a debug-only diagnostic, not a behavioural difference in the shipped
  artifact.)
* **G2 (out-of-range enum)**: `channel_mode` is a `tflac_u8` field and the C
  compares only `!= TFLAC_CHANNEL_INDEPENDENT`, so all 256 values are legal
  inputs and values `4..=255` (past `TFLAC_CHANNEL_MODE_COUNT`) take the
  "not independent" branch and are **preserved verbatim** when
  `channels == 2 && bitdepth != 32`. All 256 values are tested in all four
  branch outcomes.

Two tests enumerate the error surface exhaustively:
`err_exhaustive_u8_fields` (196 608 configurations across three 256×256 planes)
and `err_exhaustive_u32_field_boundaries` (every threshold crossing of all four
`u32` fields, plus the extremes).

## Phase D — symbol parity and feature combinations

`tests/symbol_parity.rs` encodes Phase D as executable tests:

| test | asserts |
|------|---------|
| `every_c_exported_symbol_is_exported_by_rust` | `nm -D --defined-only` diff (C → Rust) is **empty** for both Rust profiles |
| `rust_so_has_no_unresolved_non_libc_symbols` | `nm -D --undefined-only` on the Rust `.so`s contains only libc / language-runtime symbols |
| `harness_really_loads_two_distinct_shared_objects` | a C `.so` **and** ≥ 2 distinct Rust `.so`s really were `dlopen`ed, so Phase B/C cannot be vacuous |
| `rust_struct_layout_matches_the_c_compiler` | `sizeof`/`_Alignof`/`offsetof` from a probe compiled against the real header match what `src/lib.rs` and the harness assume |

### Feature combinations

`Cargo.toml` declares **no `[features]`**, `src/` contains **no
`#[cfg(feature = ...)]`**, `c_src/CMakeLists.txt` declares no `option()` and
`src/lib.c` contains no `#ifdef`. The complete configuration space is therefore
one feature set, verified under both invocations:

```
cargo check/build/test --no-default-features   ✅
cargo check/build/test                          ✅
```

`./run_all_configs.sh` enumerates the feature power set out of `Cargo.toml`
programmatically (so it stays correct if features are added later) and runs
`cargo check`, both `cargo build`s and the full `cargo test` suite for each.

## Harness negative control (anti-vacuity)

`./mutation_check.sh` injects **28 deliberate bugs** into `src/lib.rs` one at a
time and requires the suite to fail for each: every comparison bound off by one,
every rejection removed, every auto-fill constant changed, the partition-order
loop's shift amount / cap / comparison altered, the `tflac_size_memory`
constants, mask, multiplier and wrapping altered, and each
`#[unsafe(no_mangle)]` export deleted.

```
ALL 28 MUTANTS DETECTED
```

## Result

| gate | status |
|------|--------|
| `SYMBOLS.md` — 0 missing symbols, 0 unresolved non-libc symbols in Rust | ✅ |
| Phase B — all 40 `CONFIGS.md` rows pass across randomized + exhaustive inputs | ✅ |
| Phase C — all 22 `ERRORS.md` rows have a passing error-path differential test | ✅ |
| Every feature combination (1) under both `--no-default-features` and defaults | ✅ |
| Harness proven non-vacuous (28/28 mutants caught) | ✅ |

**68 tests pass** (40 valid-path + 24 error-path + 4 symbol/ABI), against both
the debug and the release Rust `.so`, in ~14 s.

**No divergence between the C and Rust implementations was found.** `src/lib.rs`
required no correctness changes.

## Reproducing

```bash
# build the C shared object
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . && cd ../..

# full suite, all feature combinations
./run_all_configs.sh

# harness negative control
./mutation_check.sh

# quick run (scales the randomized row counts)
HARVEST_ITERS=200 cargo test
```

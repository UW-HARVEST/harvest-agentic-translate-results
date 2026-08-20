# VERIFICATION.md — completion gate

Differential verification of the Rust translation of `c_src/src/main.c` against
the C original. The C code is the ground truth throughout; every divergence was
fixed on the Rust side.

## How to reproduce

```sh
# 1. Build the C reference (executable, as CMakeLists.txt declares it)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . && cd ../..

# 2. Every build configuration: cargo check + build + the full differential suite
./run_all_configs.sh

# 3. Prove the suite is actually sensitive (mutation testing)
./mutation_check.sh
```

The test suite builds the two shared objects it needs by itself
(`gcc -shared -fPIC` for the C side, `cargo build --lib` for the Rust
`cdylib`), so `cargo test` alone is sufficient too.

## Phase A — surface map

| artifact | content |
|----------|---------|
| `SYMBOLS.md`  | `nm -D` of both shared objects; 4 C-defined symbols, 4 matched, **0 missing** |
| `ERRORS.md`   | 11 rejection rows mechanically derived from the C source + 10 generic FFI-boundary rows, **0 unchecked** |
| `CONFIGS.md`  | 36 valid-input configuration rows (runtime × input shape × build config), **0 unchecked** |

## Phase B / C — differential tests

All tests reach **both** implementations only through their shared objects,
loaded with `libloading`/`dlopen`. The Rust functions are never called directly,
so the `#[no_mangle] extern "C"` export wrappers are themselves under test.

| test target | tests | what it covers |
|-------------|-------|----------------|
| `tests/ffi_diff.rs`      | 17 | `printLine`, `bad`, `good` — in-process `dlopen` of both objects with file-descriptor-1 capture. Uses a sequential harness (`harness = false`) because libtest's parallel progress output would otherwise land inside a capture. |
| `tests/main_diff.rs`     | 26 | the exported `main`, i.e. `scanf("%d")` + the `if (x)` branch, one fresh process per invocation; plus the stand-alone executables end to end |
| `tests/sweep.rs`         |  6 | exhaustive enumeration of short byte strings (~5.3 k inputs) and high-volume randomized sweeps |
| `tests/symbol_parity.rs` |  4 | `nm -D` diff, `dlopen(RTLD_NOW)` eager-binding load, `dlsym` of each export |
| `tests/c_opt_levels.rs`  |  4 | the C side rebuilt at `-O0/-O1/-O2/-O3/-Os`, all compared against the one Rust object |
| **total**                | **57** | |

Randomization is property-style with the fixed seed `0x5EED_1234_ABCD_EF01`
(SplitMix64, `tests/common/mod.rs`), so failures are reproducible.

## Phase D — symbol parity and configurations

* `nm -D` symbol diff (C-defined → Rust-exported): **empty**.
* `dlopen(libdriver.so, RTLD_NOW)` succeeds, i.e. **0** unresolved non-libc
  symbols.
* Feature combinations: `Cargo.toml` declares `[features] default = []` and
  `c_src` has no build options and no preprocessor conditionals, so there is a
  single valid combination. It is verified three ways (`<default>`,
  `--no-default-features`, `--all-features`) and additionally under the
  `release` profile (`panic = "abort"`). `run_all_configs.sh` derives the list
  from `Cargo.toml` rather than hard-coding it.

## Mutation sensitivity

`./mutation_check.sh` injects 27 defects into the Rust translation:

* **22 caught** — the suite fails, as it must.
* **5 documented-equivalent** — provably unobservable through this program's
  behavior, so the suite is *expected* to keep passing:
  * `int x = 0;` runs *before* `scanf`, and `scanf`'s return value is
    discarded, so "conversion failed, `x` untouched" and "conversion produced 0"
    are the same observable program. That makes the lone-sign rejection, the
    matching-failure return, and the recording of a *leading* zero
    behaviorally inert.
  * the negative `strtol` cutoff differs from the positive one only at magnitude
    exactly 2^63, which yields `LONG_MIN` either way (via the exact value or via
    `ERANGE` saturation).
  * the stdin refill chunk size is an internal buffering detail.
* **0 missed.**

## Notable C behaviors reproduced (not "fixed")

1. `helperBad()` returns the address of a function-local array (CWE-562,
   undefined behavior). GCC diagnoses this with `-Wreturn-local-addr` and emits
   an unconditional `return NULL` — verified in the disassembly at every
   optimization level. `printLine()` skips null pointers, so **`bad()` prints
   nothing at all**. The translation models this exactly instead of printing
   `"helperBad string"`.
2. `scanf("%d")` accumulates into a `long` and *saturates* on overflow, then the
   result is narrowed with `*ARG (int *) = (int) num.l`. So
   `"4294967296"` → `0` → `bad()` (no output), while
   `"99999999999999999999"` → `LONG_MAX` → `(int) -1` → `good()`, and
   `"-99999999999999999999"` → `LONG_MIN` → `(int) 0` → `bad()`.
3. `%d` pins the conversion base at 10, so a leading `0x` is *not* honored:
   `"0x10"` parses as the number `0` and the `'x'` is pushed back.
4. `printLine` is byte-transparent (`printf("%s\n", …)`, which gcc lowers to
   `puts`), so the Rust wrapper passes raw bytes through without UTF-8
   validation.
5. `main` has a single `return 0;`, so the exit status is `0` for every input,
   including all rejections.

# SYMBOLS.md — Symbol parity between the C `.so` and the Rust `.so`

## How this table was produced

```sh
# C shared library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust shared library (crate-type = ["cdylib"])
cargo build --offline
# -> target/debug/libdriver.so

nm -D --defined-only c_src/build/libdriver.so   | awk '{print $3}' | sort > c.txt
nm -D --defined-only target/debug/libdriver.so  | awk '{print $3}' | sort > r.txt
comm -23 c.txt r.txt   # symbols in C but NOT in Rust  -> MUST be empty
```

## Exported (dynamic, defined) symbols

`nm -D --defined-only` on both libraries. `T` = global text symbol.

| # | symbol | C `.so` | Rust `.so` | C declaration | Rust definition |
|---|--------|---------|------------|---------------|-----------------|
| 1 | `printLine`    | `T` | `T` | `void printLine(const char *line)`        | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn printLine` |
| 2 | `printIntLine` | `T` | `T` | `void printIntLine(int intNumber)`        | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn printIntLine` |
| 3 | `bad`          | `T` | `T` | `void bad(float data)`                    | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn bad` |
| 4 | `good`         | `T` | `T` | `void good(float data)`                   | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn good` |
| 5 | `driver`       | `T` | `T` | `void driver(float goodData, float badData)` (only symbol in `include/driver.h`) | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn driver` |

### Symbol diff

```
--- C only (missing from Rust) ---
   <empty>
--- Rust only (extra) ---
   <empty>
```

**Result: 0 missing symbols.** The Rust `.so` exports exactly the same 5 names
as the C `.so`, with the same C ABI signatures.

## Non-exported (internal) C functions

These are `static` in `c_src/src/driver.c`, therefore *not* dynamic symbols and
*not* required to be exported by Rust. They are still translated (as private
Rust `fn`s) and are exercised indirectly through `good` / `driver`.

| C symbol | linkage | Rust counterpart | reachable via |
|----------|---------|------------------|---------------|
| `goodG2B` | `static` (local `t`) | `fn goodG2B()` (private) | `good`, `driver` |
| `goodB2G` | `static` (local `t`) | `fn goodB2G(data: c_float)` (private) | `good`, `driver` |

## Undefined (imported) symbols

The C `.so` imports only `printf` and `puts` from libc (GCC rewrites
`printf("%s\n", s)` into `puts(s)` inside `printLine`; the two emit byte-identical
output for any NUL-terminated string). Its remaining undefined symbols are the
standard weak ELF/CRT ones (`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports `printf` plus the libc/`libgcc` unwinder symbols that the
Rust standard library needs (`malloc`, `memcpy`, `_Unwind_*`, …). There are **no
undefined non-libc / non-unwinder symbols**, i.e. nothing is left dangling:

```sh
nm -D --undefined-only target/debug/libdriver.so \
  | grep -v -E 'GLIBC|GCC_|_ITM_|__gmon_start__|__cxa_|gettid|statx'
# -> no output
```

## Build-time configurations

`Cargo.toml` declares **no `[features]` table**, and `c_src/CMakeLists.txt`
declares no options / `#ifdef`-driven variants (no `option()`, no
`target_compile_definitions`, no `add_definitions`). See `CONFIGS.md` for the
full enumeration; the complete set of valid Cargo feature combinations is:

| # | cargo invocation | feature set |
|---|------------------|-------------|
| 1 | `cargo build`                          | `{}` (no `default` feature exists) |
| 2 | `cargo build --no-default-features`    | `{}` (identical to #1) |
| 3 | `cargo build --all-features`           | `{}` (identical to #1) |

All three are the same compilation, so no `#[cfg(feature = "…")]` gating is
needed anywhere in `src/`. Symbol parity was verified to be empty-diff for each.

## Completeness of the translation

The whole C tree is three files and it is *all* translated — nothing was skipped,
so no symbol needed a new implementation (and nothing is stubbed or
`unimplemented!()`):

| C file | lines | translated into |
|--------|-------|-----------------|
| `c_src/include/driver.h` | 29 | declaration of `driver` — covered by the `extern "C"` export |
| `c_src/src/driver.c` | 87 | `src/lib.rs` — all 7 functions (`printLine`, `printIntLine`, `bad`, `goodG2B`, `goodB2G`, `good`, `driver`) |
| `c_src/CMakeLists.txt` | 37 | `Cargo.toml` (`crate-type = ["cdylib"]`) |

```sh
find c_src -name '*.c' -o -name '*.h' | grep -v /build/   # only the two files above
```

## Verification summary

| gate | result |
|------|--------|
| `nm -D` symbol diff (C \ Rust) | **empty** — 5/5 exported symbols matched, in all 6 builds |
| undefined non-libc / non-unwinder symbols in the Rust `.so` | **none** |
| Phase B — `CONFIGS.md` rows C1–C40 | **all pass** (`tests/differential_valid.rs`, 39 cases) |
| Phase C — `ERRORS.md` rows E1–E25 + G1–G4 | **all pass** (`tests/differential_errors.rs`, 29 cases) |
| harness self-checks | **pass** (`tests/harness_selfcheck.rs`, 5 cases) |
| every feature combination × dev/release profile | **all 6 pass** (`tests/feature_matrix.sh`) |
| exhaustive sweep of all 2^32 `f32` inputs through `bad` and `good` | **byte-identical everywhere** (16 slices of 2^28, ~40 min) |

The tests only ever reach the implementations through `dlopen`/`dlsym`
(`libloading`) on the two `.so` files, so the `#[no_mangle] extern "C"` export
wrappers are themselves part of what is verified; no Rust function of the crate
is called directly.

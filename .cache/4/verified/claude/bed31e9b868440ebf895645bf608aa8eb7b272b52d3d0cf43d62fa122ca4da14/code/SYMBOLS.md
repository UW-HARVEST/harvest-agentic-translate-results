# SYMBOLS.md — exported symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust
cargo build                    # -> target/debug/libhex2bin_lib.so
```

## Whole-translation-unit inventory of the C library

`c_src/CMakeLists.txt` compiles exactly one translation unit: `src/lib.c`
(there are no other `.c` files, no `#ifdef`-selected sources, no generated
sources). `c_src/include/lib.h` declares exactly one function.

| C source file | functions defined | translated in Rust? |
|---|---|---|
| `c_src/src/lib.c` | `hex2bin` | yes — `src/hex2bin.rs` (`#[unsafe(no_mangle)] pub unsafe extern "C" fn hex2bin`) |

No C source file is left untranslated, so there is no missing-module case to
repair.

## `nm -D --defined-only` comparison

| # | symbol | type in C `.so` | present in Rust `.so` | notes |
|---|--------|-----------------|-----------------------|-------|
| 1 | `hex2bin` | `T` (global text) | **yes**, `T` | exported via `#[unsafe(no_mangle)] extern "C"` |

```
$ nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort   > c_syms
$ nm -D --defined-only target/debug/libhex2bin_lib.so    | awk '{print $NF}' | sort   > rust_syms
$ comm -23 c_syms rust_syms      # symbols in C but missing from Rust
<empty>
```

**Missing symbols: 0.**

## Weak / compiler-emitted symbols (not part of the API)

The C `.so` additionally lists these *undefined/weak* entries, which are
toolchain artifacts, not library API, and are therefore not required of the
Rust `.so`:

| symbol | kind | comment |
|---|---|---|
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable` | `w` (weak undef) | GCC transactional-memory stubs |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | glibc destructor registration |
| `__gmon_start__` | `w` | profiling hook |
| `strchr@GLIBC_2.2.5` | `U` | libc import used by `hex2bin` |

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only target/debug/libhex2bin_lib.so` lists only libc
(`memcpy`, `malloc`, `strlen`, `write`, …), pthread, `dl_iterate_phdr`, and
`_Unwind_*` entries pulled in by the Rust standard library / panic runtime.

**Non-libc / non-runtime undefined symbols: 0.**

## Feature combinations

`Cargo.toml` has **no `[features]` table**, so the crate has exactly one
build configuration. `c_src/CMakeLists.txt` defines no options, no
`target_compile_definitions`, and no conditional sources, so the C side has
exactly one configuration too.

| # | feature combination | `cargo check` | symbol parity |
|---|---------------------|---------------|---------------|
| 1 | `--no-default-features` (empty set == default) | pass | pass (0 missing) |

Combinations are enumerated mechanically by `scripts/feature_combos.sh` (parses
`[features]` from `Cargo.toml` and prints the powerset; the empty set is always
included) and each one is checked/built/tested by `scripts/run_diff_tests.sh`.

Symbol parity is additionally asserted as a test
(`tests/harness.rs::symbol_parity`), so it is re-verified on every run.

# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

## Build commands

C shared object:

```sh
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> translated_rust/c_src/build/libtranslated_rust.so
```

Rust shared object (`crate-type = ["cdylib"]`, `[lib] name = "rgb_to_hsv_lib"`):

```sh
cd translated_rust && cargo build --no-default-features
# -> translated_rust/target/debug/librgb_to_hsv_lib.so
```

## Translation-unit inventory (completeness check)

The whole C library is a single translation unit; nothing was skipped.

| C file | lines | translated to | status |
|--------|-------|---------------|--------|
| `c_src/include/lib.h` | 1 (single prototype) | `src/lib.rs` (signature of `rgb_to_hsv`) | complete |
| `c_src/src/lib.c` | 38 (one function, `rgb_to_hsv`) | `src/lib.rs` | complete |

`CMakeLists.txt` lists exactly one source file (`src/lib.c`), so there is no
untranslated module and no macro-generated symbol family.

## `nm -D` on the C `.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
00000000000010f9 T rgb_to_hsv
```

Defined, non-weak, non-libc public symbols exported by the C `.so`: **1**

## Parity table

| # | C symbol | type in C `.so` | exported by Rust `.so` | type in Rust `.so` | status |
|---|----------|-----------------|------------------------|--------------------|--------|
| 1 | `rgb_to_hsv` | `T` (global text) | yes | `T` (global text) | ✅ match |

Weak/link-editor artifacts present in the C `.so` (`_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`) are toolchain
stubs, not library API; they are also present as weak entries in the Rust `.so`.

### Symbol diff

```sh
diff <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '$2=="T"{print $3}' | sort) \
     <(nm -D --defined-only target/debug/librgb_to_hsv_lib.so   | awk '$2=="T"{print $3}' | sort)
```

Result: **empty** — every symbol the C `.so` exports is exported by the Rust
`.so` under the exact same name.

### Undefined symbols in the Rust `.so`

`nm -D --undefined-only target/debug/librgb_to_hsv_lib.so` lists only libc /
libgcc-unwind imports pulled in by the Rust standard library
(`malloc`, `memcpy`, `pthread_key_create`, `_Unwind_*`, …).

**0 missing symbols, 0 undefined non-libc symbols.**

## Feature combinations

`Cargo.toml` has **no `[features]` section**, so the complete set of valid
build-time configurations is:

| # | cargo invocation | notes |
|---|------------------|-------|
| 1 | `cargo check/build/test --no-default-features` | the only configuration |
| 2 | `cargo check/build/test --all-features` | identical to #1 (no features exist) |
| 3 | `cargo check/build/test` (default) | identical to #1 |

`c_src/CMakeLists.txt` defines no `option()`, no `target_compile_definitions`,
and the C source contains no `#if`/`#ifdef`, so there is exactly one C
configuration as well.

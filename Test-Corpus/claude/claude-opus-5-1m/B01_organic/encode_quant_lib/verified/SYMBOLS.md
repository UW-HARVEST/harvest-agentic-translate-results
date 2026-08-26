# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on the two shared libraries.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust
cargo build --no-default-features
# -> target/debug/libencode_quant_lib.so
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C file | lines | translated to |
|--------|-------|---------------|
| `c_src/src/lib.c` | 62 | `src/lib.rs` (`encode_quant`) |
| `c_src/include/lib.h` | 1 (single prototype) | n/a (declaration only) |

No C source file is left untranslated, so there is no "whole module missing"
completeness failure to repair.

## `nm -D` on the C `.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
00000000000010f9 T encode_quant
```

The four `w` entries are **undefined weak** toolchain/libc glue emitted into
every gcc shared object (`crtstuff`/`__cxa_finalize`); they are not part of the
library's API and are not symbols the Rust `.so` must define.

## Symbol parity table

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `encode_quant` | `T` (defined, global) | `T` (defined, global) | `#[no_mangle] pub extern "C" fn` in `src/lib.rs` |

Non-API entries present in the C `.so` (undefined weak, deliberately not
mirrored):

| symbol | kind | why not required in Rust |
|--------|------|--------------------------|
| `_ITM_deregisterTMCloneTable` | `w` undefined weak | gcc `crtstuff` transactional-memory hook |
| `_ITM_registerTMCloneTable` | `w` undefined weak | gcc `crtstuff` transactional-memory hook |
| `__cxa_finalize@GLIBC_2.2.5` | `w` undefined weak | glibc destructor registration |
| `__gmon_start__` | `w` undefined weak | gprof profiling hook |

## Diff result

```
$ comm -23 <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort) \
           <(nm -D --defined-only target/debug/libencode_quant_lib.so | awk '{print $NF}' | sort)
(empty)
```

* Symbols exported by C but missing from Rust: **0**
* Undefined non-libc symbols in the Rust `.so`: **0**
  (the Rust `.so`'s remaining `nm -D` entries are its own mangled `_ZN…`/`_R…`
  internals plus glibc/`libgcc_s` imports)

**Phase A symbol gate: PASS.**

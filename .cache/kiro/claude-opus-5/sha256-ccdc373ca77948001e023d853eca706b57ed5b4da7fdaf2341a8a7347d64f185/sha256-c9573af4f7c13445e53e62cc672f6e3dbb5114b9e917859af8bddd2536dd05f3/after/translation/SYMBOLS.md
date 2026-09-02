# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .   # -> liblong.so (-O0, no CMAKE_BUILD_TYPE)
cd translation && cargo build --release                              # -> target/release/liblong.so
```

## C `.so` dynamic symbols (defined)

```
$ nm -SD --defined-only c_src/build/liblong.so | grep -v ' [wU] '
0000000000004060 0000000000100000 B array
00000000000011f4 00000000000000ae T long_exec
0000000000001139 00000000000000bb T perform_expensive_operations
```

`c_src/include/long.h` declares only `long_exec`; `perform_expensive_operations`
and `array` are non-`static` file-scope definitions in `c_src/src/long.c` and are
therefore part of the exported ABI as well. All three are in scope for this
verification.

## Rust `.so` dynamic symbols (defined)

```
$ nm -SD --defined-only translation/target/release/liblong.so | grep -v ' [wU] '
0000000000050000 0000000000100000 B array
0000000000012440 00000000000010ec T long_exec
0000000000013530 000000000000007e T perform_expensive_operations
```

## Parity table

| # | C symbol | type | size (bytes) | exported by Rust `.so` | notes |
|---|----------|------|--------------|------------------------|-------|
| 1 | `array` | `B` (.bss object) | `0x100000` = 1048576 | yes, `B`, `0x100000` | `int array[256*1024]`; `#[no_mangle] pub static mut array: Array` in Rust, `#[repr(C, align(32))]`. Sizes match exactly. |
| 2 | `long_exec` | `T` (func) | 0xae | yes, `T` | `extern "C" fn long_exec(seed: c_uint)` |
| 3 | `perform_expensive_operations` | `T` (func) | 0xbb | yes, `T` | `extern "C" fn perform_expensive_operations()` |

Function `size` differs (different codegen) — irrelevant; only name/kind/data-size
parity matters for ABI compatibility.

## Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/liblong.so       | awk '{print $NF}' | sort) \
           <(nm -D --defined-only translation/target/release/liblong.so | awk '{print $NF}' | sort)
<empty>
```

**0 symbols missing from the Rust `.so`.**

No C source file was left untranslated: `c_src` contains exactly one translation
unit (`src/long.c`, 68 lines incl. the 23-line licence header) and one header
(`include/long.h`), and every non-static definition in it has a Rust counterpart.

## Undefined symbols in the Rust `.so`

```
$ nm -D --undefined-only translation/target/release/liblong.so | awk '{print $NF}' | sort -u
```

All 53 entries are libc / libgcc-unwind imports (`printf`, `srand`, `rand`,
`malloc`, `memcpy`, `_Unwind_*`, `__cxa_finalize`, …). **0 undefined non-libc
symbols.** The C `.so` imports `printf`, `srand`, `rand` (plus `__cxa_finalize`);
the Rust `.so` imports the same three plus the allocator/unwinder support the
Rust standard library needs.

## Verified checklist

- [x] `nm -D` shows 0 missing symbols in Rust (`comm -23` is empty).
- [x] `nm -D` shows 0 undefined non-libc symbols in Rust.
- [x] Data symbol `array` has byte-identical size in both objects.
- [x] Holds for both feature configurations (`--no-default-features`, and
      `--features debug-stats`) — see `check_features.sh` output in the report.

# SYMBOLS.md — Phase A / Phase D symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/liblong.so

# Rust
cd translation && cargo build --release
# -> translation/target/release/liblong.so
```

## Complete C translation unit inventory

`c_src` contains exactly one header and one translation unit, so there is no
possibility of a whole module having been skipped by the translation step:

| C file | translated in |
|--------|---------------|
| `c_src/include/long.h` | `translation/src/lib.rs` (declaration of `long_exec`) |
| `c_src/src/long.c`     | `translation/src/lib.rs` (all 3 definitions) |

`c_src/CMakeLists.txt` lists `src/long.c` as the only source of the `long`
target. There are no other `.c` files in the tree:

```
$ find c_src -name '*.c' -o -name '*.h'
c_src/include/long.h
c_src/src/long.c
```

## Exported symbols of the C `.so`

```
$ nm -D --defined-only c_src/build/liblong.so
0000000000004060 B array
00000000000011f4 T long_exec
0000000000001139 T perform_expensive_operations
```

## Exported symbols of the Rust `.so`

```
$ nm -D --defined-only translation/target/release/liblong.so | grep -v ' [UwW] '
000000000004631c B array
000000000000c8c0 T long_exec
000000000000c9a0 T perform_expensive_operations
```

## Parity table

| # | symbol | nm type (C) | nm type (Rust) | C declaration | Rust definition | status |
|---|--------|-------------|----------------|---------------|-----------------|--------|
| 1 | `array` | `B` (`.bss` object, 1048576 bytes) | `B` (`.bss` object, 1048576 bytes) | `int array[256*1024];` (`long.c:33`) | `#[unsafe(no_mangle)] pub static mut array: [c_int; ARRAY_SIZE]` | **present** |
| 2 | `long_exec` | `T` (text) | `T` (text) | `void long_exec(unsigned int seed);` (`long.h:27`, `long.c:49`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn long_exec(seed: c_uint)` | **present** |
| 3 | `perform_expensive_operations` | `T` (text) | `T` (text) | `void perform_expensive_operations();` (`long.c:36`, not in header but non-`static`, therefore exported) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn perform_expensive_operations()` | **present** |

### Missing symbols

**None.** The C→Rust symbol diff is empty in the `C \ Rust` direction, which is
the direction that matters for drop-in ABI replacement.

No stubs, no `unimplemented!()`, no `todo!()` — every exported symbol is a real
translation of the corresponding C definition:

```
$ grep -c 'unimplemented!\|todo!\|panic!' translation/src/lib.rs
0
```

### Extra symbols exported by the Rust `.so`

The Rust `cdylib` additionally exports the standard Rust/`compiler-builtins`
runtime helpers (`rust_eh_personality`, `__rust_*` allocator shims, etc.) and
the usual ELF bookkeeping symbols (`_init`, `_fini`, `_edata`, `_end`,
`__bss_start`). Extra symbols are harmless for ABI compatibility — a consumer
linked against the C library never references them. The parity requirement is
one-directional: every C symbol must exist in Rust.

### Size / ABI checks performed by the test suite

`tests/symbols.rs` re-derives both symbol lists with `nm -D` at test time and
asserts the `C \ Rust` difference is empty, so this document cannot silently
drift from reality. It additionally asserts, via `readelf -sW`, that the `array`
object has **the same st_size in both objects** (1048576 = 256*1024*4), because
a consumer is allowed to `dlsym("array")` and index all 262144 elements.

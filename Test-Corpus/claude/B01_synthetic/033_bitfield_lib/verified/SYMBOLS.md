# SYMBOLS.md — Symbol parity between the C `.so` and the Rust `.so`

Derived mechanically, not from assumptions:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u

# Rust
cargo build --release
nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u
```

## Full public symbol surface of the C `.so`

The C library is built from a single translation unit (`c_src/src/driver.c`,
per `c_src/CMakeLists.txt`). It defines exactly two non-`static` functions, so
`nm -D` yields exactly two dynamic symbols. Neither is macro-generated.

| # | C symbol | C signature | declared in `include/driver.h`? | present in Rust `.so`? |
|---|----------|-------------|--------------------------------|------------------------|
| 1 | `driver`    | `void driver(unsigned int x, unsigned int y, bool b, int z)` | yes | **yes** |
| 2 | `print_foo` | `void print_foo(const foo_t *foo)`                           | no (exported anyway — non-`static`, so it is part of the library ABI) | **yes** |

`print_foo` is *not* in the public header but *is* an exported dynamic symbol,
which makes it a real entry point an external caller can resolve and call. It
is therefore treated as the lowest-level public entry point and tested directly
(see `CONFIGS.md`), not just indirectly through `driver`.

## Symbol diff

```
$ comm -23 c_syms.txt rs_syms.txt     # in C .so but NOT in Rust .so
(empty)
```

**0 symbols missing.** No implementation was absent, so no C source needed to
be translated and no `#[no_mangle]` wrapper needed to be added. Both symbols
are exported from `src/lib.rs` via `#[unsafe(no_mangle)] pub unsafe extern "C"`.

The Rust `.so` additionally exports the usual Rust/`cdylib` runtime symbols
(`rust_eh_personality`, `_ZN*` std internals, etc.). Extra symbols are not a
correctness problem; the requirement is that every C symbol is present, which
holds.

## Undefined (imported) non-libc symbols

```sh
nm -D --undefined-only target/release/libdriver.so
```

The Rust `.so` imports only libc/glibc symbols (`printf`, `memcpy`,
`pthread_*`, `__cxa_*`, …) — the same class the C `.so` imports (`printf`).
**0 missing/undefined non-libc symbols.**

## Verified struct layout (affects the `print_foo` ABI)

`foo_t` is private to the C translation unit, but its layout is part of
`print_foo`'s ABI. Measured with a probe compiled by the same `cc` (GCC 11.5.0,
x86-64 SysV):

```
sizeof=8  alignof=4  offsetof(z)=4
x -> byte0 mask 0x03   (bits 0..1)
y -> byte0 mask 0x1c   (bits 2..4)
b -> byte0 mask 0x20   (bit 5)
z -> bytes 4..7
byte0 bits 6..7 -> padding, never read
```

This matches `foo_t` in `src/lib.rs` (`bits: u8`, `_pad: [u8;3]`, `z: c_int`,
`#[repr(C)] #[repr(align(4))]`, with compile-time `size_of == 8` /
`align_of == 4` assertions) exactly.

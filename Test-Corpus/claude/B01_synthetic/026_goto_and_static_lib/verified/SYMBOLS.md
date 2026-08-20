# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared libraries.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust
cargo build            # -> target/debug/libdriver.so
```

## C source inventory (what `add_library` compiles)

`c_src/CMakeLists.txt` builds exactly one translation unit:

| C source file | translated to |
|---------------|---------------|
| `c_src/src/driver.c` | `src/lib.rs` |

Public headers: `c_src/include/driver.h` (declares exactly `void driver(int, int, int)`).
There are no namespace/renaming macros (`#define foo BAR(foo)`) anywhere in the
headers or sources, so source-level names == final linker names.

## `nm -D` on the C `.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
00000000000011b5 T driver
                 U printf@GLIBC_2.2.5
                 U puts@GLIBC_2.2.5
```

`nm -D --defined-only c_src/build/libdriver.so`:

```
00000000000011b5 T driver
```

## `nm -D` on the Rust `.so`

`nm -D --defined-only target/debug/libdriver.so` (filtered to non-local
`T`/`D`/`B`/`W` entries):

```
0000000000012430 T driver
```

## Symbol parity table

| # | C symbol | C type | exported by Rust `.so` | Rust definition | status |
|---|----------|--------|------------------------|-----------------|--------|
| 1 | `driver` | `T` (global text) | yes, `T driver` | `#[unsafe(no_mangle)] pub extern "C" fn driver` in `src/lib.rs` | OK |

### Symbols intentionally NOT exported

These have internal linkage in C (`static`) and therefore are absent from the
C `.so` dynamic symbol table. They are reproduced as private Rust items and must
NOT be exported:

| C item | linkage | Rust counterpart |
|--------|---------|------------------|
| `static int y = 123;` | internal | `static Y: AtomicI32 = AtomicI32::new(123)` |
| `static int multi_stage(int x, int z)` | internal | `fn multi_stage(x: c_int, z: c_int) -> c_int` |

### Undefined (imported) symbols

The C `.so` imports `printf@GLIBC_2.2.5` and `puts@GLIBC_2.2.5`. `puts` appears
only because GCC rewrites `printf("literal\n")` into `puts("literal")`; this is a
codegen detail, not part of the ABI, and produces byte-identical output on the
same `stdout` FILE stream. The Rust translation routes every message through
`printf`, so the emitted byte stream and the stdio buffering behaviour are
identical.

The Rust `.so` additionally imports the usual `libc`/`libgcc` runtime symbols
pulled in by Rust `std` (`malloc`, `memcpy`, `_Unwind_*`, `pthread_key_create`,
…). All of them are libc/compiler-runtime symbols; there are **0 undefined
non-libc symbols**.

## Diff result

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $NF}' | sort) \
     <(nm -D --defined-only target/debug/libdriver.so  | awk '{print $NF}' | sort)
```

**Empty** — every symbol exported by the C `.so` is exported by the Rust `.so`
with the exact same name, and the Rust `.so` exports no extra public API symbol.

- [x] `nm -D` shows 0 missing symbols in Rust.
- [x] `nm -D` shows 0 undefined non-libc symbols in Rust.

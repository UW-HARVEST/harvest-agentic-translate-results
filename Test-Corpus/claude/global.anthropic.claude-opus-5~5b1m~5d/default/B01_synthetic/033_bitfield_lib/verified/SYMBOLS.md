# SYMBOLS.md — Phase A symbol surface

C source translated: `c_src/src/driver.c` (the ONLY `.c` file in the project,
per `c_src/CMakeLists.txt`: `add_library(driver SHARED src/driver.c)`).
Public header: `c_src/include/driver.h`.

## `nm -D --defined-only` on the C `.so`

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001175 T driver
0000000000001119 T print_foo
```

(`printf@GLIBC_2.2.5` is *undefined* / imported, not exported — not part of the
surface. No data symbols, no macro-generated symbols, no weak symbols.)

## `nm -D --defined-only` on the Rust `.so`

```
$ nm -D --defined-only translation/target/release/libdriver.so | grep -v ' [wWuv] '
0000000000011710 T driver
0000000000011740 T print_foo
```

## Parity table

| # | symbol      | type | in C `.so` | in Rust `.so` | status | notes |
|---|-------------|------|-----------|---------------|--------|-------|
| 1 | `driver`    | `T` (func) | yes | yes | OK | `void driver(unsigned int, unsigned int, bool, int)`; Rust `#[no_mangle] extern "C" fn driver(c_uint, c_uint, u8, c_int)` |
| 2 | `print_foo` | `T` (func) | yes | yes | OK | `void print_foo(const foo_t *)`; not declared in `driver.h` but has external linkage in C, so it IS part of the exported ABI surface |

**Missing from Rust: NONE.**
**Extra non-libc undefined symbols in Rust: NONE** (only `printf` + the usual
glibc/`__cxa`/unwind imports, which the C `.so` also imports).

### Non-exported C internals

| C entity | kind | exported? | Rust counterpart |
|----------|------|-----------|------------------|
| `foo_t` (`typedef struct { unsigned int x:2; unsigned int y:3; bool b:1; int z; }`) | type | no (types have no symbols) | `#[repr(C)] pub struct foo_t { bits: u8, z: c_int }` — 8 bytes, align 4, matching the SysV/ELF bit-field layout (x = bits 0..1, y = bits 2..4, b = bit 5 of byte 0; `z` at offset 4) |

### Verification command

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so           | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so \
         | grep -v ' [wWuv] ' | awk '{print $3}' | sort)
```
=> empty diff (see `tests/symbols.rs::symbol_parity_c_vs_rust`).

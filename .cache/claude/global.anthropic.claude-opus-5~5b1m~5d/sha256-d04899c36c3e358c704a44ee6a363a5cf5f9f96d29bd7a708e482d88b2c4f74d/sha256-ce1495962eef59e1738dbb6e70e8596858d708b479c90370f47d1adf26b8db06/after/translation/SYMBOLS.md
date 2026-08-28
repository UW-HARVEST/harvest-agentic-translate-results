# SYMBOLS.md — public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared objects.

```
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (completeness check)

The whole C library is a single translation unit, so there is no possibility of
a "skipped module":

| C file | translated in | status |
|--------|---------------|--------|
| `c_src/src/lib.c` (119 lines) | `translation/src/lib.rs` | translated in full |
| `c_src/include/lib.h` (3 lines, declares `w_utf8_filter` only) | — (header) | n/a |

`c_src/src/lib.c` defines exactly two functions with external linkage
(`w_utf8_drop`, `w_utf8_filter`) plus four function-like macros
(`valid_1` … `valid_4`, no symbols emitted) and one object macro
(`REPLACEMENT_INC`, no symbol emitted). Nothing else in the file has external
linkage, so the exported surface below is complete.

## Defined dynamic symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `w_utf8_drop`   | `T` | `T` | not declared in `lib.h`, but non-`static` in `lib.c`, therefore exported. `const char *(const char *)` |
| 2 | `w_utf8_filter` | `T` | `T` | `char *(const char *, _Bool)`; Rust wrapper takes `c_uchar` — identical ABI (GCC emits `cmpb $0x0` on the incoming byte, so *any* non-zero byte is true) |

**Symbol diff (C defined − Rust defined): EMPTY.**
**Symbol diff (Rust defined − C defined): EMPTY.**

No macro-generated symbols exist (`valid_1` … `valid_4` are preprocessor
macros; `REPLACEMENT_INC` is an object-like macro).

## Undefined (imported) symbols

The C `.so` imports only these non-weak symbols; all are libc:

```
__assert_fail@GLIBC_2.2.5  malloc@GLIBC_2.2.5  memcpy@GLIBC_2.14
realloc@GLIBC_2.2.5        strdup@GLIBC_2.2.5  strlen@GLIBC_2.2.5
```

The Rust `.so` imports the same six, plus the libc/`libgcc` symbols the Rust
runtime itself needs (`_Unwind_*`, `abort`, `free`, `memmove`, `memset`,
`mmap64`, `dl_iterate_phdr`, …). **0 undefined non-libc / non-unwinder
symbols.**

Verify with:

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so    | awk '{print $NF}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
```

(`translation/check_symbols.sh` automates exactly this and must print
`SYMBOL PARITY: OK`.)

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
buildable configuration is the default one (`--no-default-features` is also
valid and produces an identical crate — verified by
`translation/check_all_features.sh`). The symbol parity check above is
performed for every one of those configurations.

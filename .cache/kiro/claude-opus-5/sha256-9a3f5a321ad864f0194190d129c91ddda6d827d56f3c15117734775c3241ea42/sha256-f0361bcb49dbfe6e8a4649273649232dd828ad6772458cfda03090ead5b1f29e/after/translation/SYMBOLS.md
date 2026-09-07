# SYMBOLS.md — exported-symbol parity

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (`c_src/src/lib.c`)

The whole library is one translation unit. Every function definition in it:

| C definition | `static`? | expected to be exported |
|---|---|---|
| `const char* extractFilename(const char* path, char separator)` | no | yes |
| `char* FIO_createFilename_fromOutDir(const char* path, const char* outDirName, const size_t suffixLen)` | no | yes |

`c_src/include/lib.h` declares only `FIO_createFilename_fromOutDir`, but
`extractFilename` has external linkage in `lib.c` and therefore lands in the
dynamic symbol table too. It must be exported by Rust as well.

No macros generate additional symbols; there are no global/static data objects.

## Parity table

| # | symbol | type | in C `.so` | in Rust `.so` | status |
|---|--------|------|-----------|---------------|--------|
| 1 | `FIO_createFilename_fromOutDir` | `T` (text, global) | yes | yes | OK |
| 2 | `extractFilename` | `T` (text, global) | yes | yes | OK |

### Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so   | awk '$2=="T"{print $3}' | sort) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '$2=="T"{print $3}' | sort)
<empty>
```

**0 symbols missing from the Rust `.so`.**

## Undefined (imported) symbols

The Rust `.so` must not require any non-libc symbol.

| symbol imported by Rust `.so` | provider | non-libc? |
|---|---|---|
| `calloc` | libc | no |
| `memcpy` | libc | no |
| `strlen` | libc | no |
| `strrchr` | libc | no |
| `strerror` | libc | no |
| `fputs` | libc | no |
| `exit` | libc | no |
| `__errno_location` | libc (glibc) | no |
| `stderr` | libc (glibc data symbol) | no |
| `_Unwind_Resume` / `rust_eh_personality` | may appear from the Rust runtime; satisfied within the same object or unused under `panic = "abort"` | no |

The C `.so` imports `calloc`, `memcpy`, `strlen`, `strrchr`, `strerror`,
`fprintf`, `exit`, `__errno_location`, `stderr` — the same libc surface.
Rust replaces the single `fprintf(stderr, "...%s", strerror(errno))` with two
`fputs` calls, which emit an identical byte stream, so the import set differs
only in `fprintf` vs `fputs` (both libc).

**0 missing/undefined non-libc symbols.**

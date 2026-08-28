# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared libraries.

```
C   : c_src/build/libdriver.so
Rust: translation/target/release/libdriver.so
```

## Defined (exported) symbols

`nm -D --defined-only` output, filtered to real (non-weak, non-toolchain) symbols:

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `decode_base64` | `T` (0x11e2) | `T` (0x11c70) | `char *decode_base64(const char *src)`; exported from Rust via `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |

**Missing from Rust: NONE.** The symbol diff is EMPTY.

### Static (non-exported) C functions

These are `static` in `c_src/src/lib.c`, therefore absent from `nm -D` on the C
`.so` and correctly NOT exported from the Rust `.so` either. They are still
translated (as private Rust `fn`s) and are covered indirectly through
`decode_base64`, which is the only caller:

| C symbol | linkage | Rust counterpart | exported? |
|----------|---------|------------------|-----------|
| `decode(char c)` | `static unsigned char` | `fn decode(c: c_char) -> u8` | no (correct — matches C) |
| `is_base64(char c)` | `static int` | `fn is_base64(c: c_char) -> c_int` | no (correct — matches C) |

There is no untranslated C module: `c_src/src/lib.c` is the only source file in
`add_library(driver SHARED ...)`, and every function in it (1 public + 2 static)
has a Rust counterpart. No stubs and no `unimplemented!()` anywhere.

## Undefined (imported) symbols

The C `.so` imports exactly `calloc`, `malloc`, `free`, `strlen` from glibc.
The Rust `.so` imports the same four plus the Rust `std`/`libgcc` runtime
(`_Unwind_*`, `memcpy`, `mmap64`, `pthread_key_*`, …). All are libc/libgcc —
**0 missing/undefined non-libc symbols.**

Because `calloc`/`malloc` are *dynamic* imports in both libraries (`U
calloc@GLIBC_2.2.5`), they are interposable with `LD_PRELOAD`. Phase C uses that
to drive the two allocation-failure branches in both implementations
identically.

## Verification command

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
```

Restricted to symbols the C library exports, this diff is empty (see
`tests/symbols.rs`, which asserts it automatically).

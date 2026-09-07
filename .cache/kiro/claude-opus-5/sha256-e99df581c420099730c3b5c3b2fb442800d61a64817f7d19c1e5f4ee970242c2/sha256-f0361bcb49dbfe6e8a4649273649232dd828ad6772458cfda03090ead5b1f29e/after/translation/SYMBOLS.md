# SYMBOLS.md — public ABI surface parity

Source of truth: `nm -D` on the C shared object built from `c_src/`.

C library: `c_src/build/libharvest-work-kpTOZu.so`
Rust library: `translation/target/release/libhdr_compare_lib.so`

## C source inventory (completeness check)

The whole C project is two files:

| C file | lines | functions defined |
|--------|-------|-------------------|
| `c_src/include/lib.h` | 3 | (declaration only) `hdr_compare` |
| `c_src/src/lib.c` | 13 | `static int hdr_valid(const uint8_t*)`, `int hdr_compare(const uint8_t*, const uint8_t*)` |

`c_src/CMakeLists.txt` builds exactly one target (`SHARED` library from
`src/lib.c`). There is **no** second module, no binary/driver target, and no
conditional compilation, so there is no un-translated C source. Both C functions
are present in `translation/src/lib.rs`.

## Defined (exported) symbols

`nm -D` defined-symbol rows, filtered to non-weak, non-libc entries:

| symbol | C `.so` | Rust `.so` | notes |
|--------|---------|-----------|-------|
| `hdr_compare` | `T` (0x1190) | `T` (0x11690) | exported by `#[unsafe(no_mangle)] pub unsafe extern "C" fn hdr_compare` |

`hdr_valid` is `static` in C, therefore has **no** dynamic symbol and must NOT
appear in the Rust `.so` either. It is a private `unsafe fn` in Rust — correct.
Confirmed absent from both `nm -D` outputs.

Weak/toolchain symbols present in both and irrelevant to ABI parity:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`.

## Symbol diff

```
comm -23 <(c defined symbols) <(rust defined symbols)   ->   (empty)
```

Reproduce with `translation/check_symbols.sh`.

**Missing/undefined non-libc symbols in the Rust `.so`: 0.**

The Rust `.so` additionally carries `U` (undefined) imports for libc and the
unwinder (`malloc`, `memcpy`, `_Unwind_*`, `pthread_key_create`, …). These come
from the Rust standard library and are all libc/`libgcc_s` runtime imports, not
missing translated symbols.

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.

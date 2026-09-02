# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D` on both shared objects.

* C `.so`:    `c_src/build/libharvest-work-FXVbjZ.so`
* Rust `.so`: `translation/target/release/libpow43_lib.so`

## Defined (`T`) symbols exported by the C `.so`

```
$ nm -D c_src/build/libharvest-work-FXVbjZ.so
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
00000000000010f9 T pow43
```

| # | C symbol | type | present in Rust `.so`? | notes |
|---|----------|------|------------------------|-------|
| 1 | `pow43`  | `T`  | YES (`T pow43`)        | `#[unsafe(no_mangle)] pub extern "C" fn pow43(x: c_int) -> f32` |
| 2 | `_ITM_deregisterTMCloneTable` | `w` (weak, undefined) | YES (`w`) | toolchain-emitted, not library API |
| 3 | `_ITM_registerTMCloneTable`   | `w` (weak, undefined) | YES (`w`) | toolchain-emitted, not library API |
| 4 | `__cxa_finalize@GLIBC_2.2.5`  | `w` | YES (`w`) | libc |
| 5 | `__gmon_start__`              | `w` | YES (`w`) | libc/profiling |

## Non-API symbols in the C source (deliberately NOT exported)

| C identifier | storage | exported? | reason |
|--------------|---------|-----------|--------|
| `g_pow43` | `static const float[129 + 16]` | no (C: local symbol; Rust: private `static G_POW43`) | `static` in C ⇒ internal linkage, correctly absent from `nm -D` on both sides |

## Header surface (`c_src/include/lib.h`)

The entire header is one line:

```c
float pow43(int x);
```

There are no macros, no renaming/aliasing macros, no additional declarations, no
`#ifdef`-guarded alternate names. So the linker symbol is plainly `pow43` and
there are no macro-generated symbols to reproduce.

## Missing-symbol diff

```
$ comm -23 <(nm -D <C.so>  | awk '$2=="T"{print $3}' | sort) \
           <(nm -D <RS.so> | awk '$2=="T"{print $3}' | sort)
<empty>
```

**Result: 0 missing symbols.** No module of the C source was left untranslated —
`c_src/src/lib.c` is the only translation unit and its single external function
is implemented and exported. Nothing is stubbed.

Undefined (`U`) symbols in the Rust `.so` are all libc / `_Unwind_*` /
`pthread_*` runtime imports pulled in by the Rust `std` prelude
(`__errno_location`, `malloc`, `memcpy`, `dl_iterate_phdr`, …). There are 0
missing/undefined **non-libc** symbols. Verified with the script
`translation/check_symbols.sh`.

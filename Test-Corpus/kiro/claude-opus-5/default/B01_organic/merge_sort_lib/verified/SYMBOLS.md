# SYMBOLS.md — symbol parity between the C `.so` and the Rust `.so`

Derived mechanically, not from assumptions.

## Commands used

```sh
nm -D --defined-only c_src/build/libharvest-work-fLbS0v.so
nm -D --defined-only translation/target/release/libmerge_sort_lib.so
nm -D --undefined-only translation/target/release/libmerge_sort_lib.so
```

## C `.so` — full `nm -D` output

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U memcpy@GLIBC_2.14
00000000000012e0 T merge_sort
```

Everything in `c_src/src/lib.c` other than `merge_sort` is declared `static`
(`spritebatch_internal_sprite_less_than_or_equal`,
`spritebatch_internal_merge_sort_iteration`,
`spritebatch_internal_merge_sort_recurse`) and therefore has internal linkage
and no dynamic symbol. `c_src/include/lib.h` declares no other function and
defines no macros that could generate additional symbols. There is exactly one
C source file in `CMakeLists.txt` (`src/lib.c`), so no module was skipped by the
translation.

## Parity table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `merge_sort` | `T` (defined, global) | `T` (defined, global) | MATCH |

Weak/undefined loader helpers (`_ITM_*`, `__cxa_finalize`, `__gmon_start__`)
and libc imports (`memcpy`) are toolchain artifacts, not library API, and are
excluded from the parity requirement.

## Missing symbols

**None.** The symbol diff is empty:

```sh
$ diff <(nm -D --defined-only c_src/build/*.so   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libmerge_sort_lib.so \
           | awk '{print $NF}' | grep -v '^_' | sort)
# (no output)
```

No `#[no_mangle]` wrapper had to be added and no C module had to be translated:
the Rust crate already implements the whole translation unit
(`merge_sort` plus all three `static` helpers as private `unsafe fn`s).

## Undefined symbols in the Rust `.so`

All are libc / libgcc-unwind imports pulled in by the Rust runtime
(`memcpy`, `malloc`, `free`, `_Unwind_*`, `pthread_key_*`, `dl_iterate_phdr`,
`open64`/`read`/`write`, …). **0 undefined non-libc symbols.**

`memcpy@GLIBC_2.14` is imported *deliberately and identically to the C*: the
translation calls libc `memcpy` directly for `merge_sort`'s bulk copy rather than
`core::ptr::copy_nonoverlapping`, so that degenerate lengths (e.g. the ~2**64
length a negative `size` produces) hit exactly the same code and produce exactly
the same outcome. See CONFIGS.md § "Divergence found and fixed".

Both parity checks are also enforced as tests (`tests/symbol_parity.rs`:
`exported_symbols_match`, `no_unexpected_undefined_symbols`) so they cannot
silently regress.

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.

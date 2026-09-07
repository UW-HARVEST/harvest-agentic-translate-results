# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on the C shared library
`c_src/build/libharvest-work-WnzTyP.so`, compared with the Rust
`translation/target/release/libgaussian_kernel_lib.so`.

## C `.so` exported symbols (non-local, filtered of linker-generated entries)

```
$ nm -D --defined-only c_src/build/libharvest-work-WnzTyP.so | grep -v ' [a-z] '
0000000000001109 T gaussian_kernel
```

The C translation unit is a single file (`c_src/src/lib.c`) declaring a single
public function in `c_src/include/lib.h`:

```c
void gaussian_kernel(float *dest, int size, float radius);
```

There are no macro-generated symbols, no additional modules, no global data
objects, and no `#ifdef`-gated alternate entry points in the C source.

## Symbol parity table

| # | C symbol | type | exported by Rust `.so` | status |
|---|----------|------|------------------------|--------|
| 1 | `gaussian_kernel` | `T` (global text) | yes, `T gaussian_kernel` | OK |

Linker-generated / runtime symbols present in either library and intentionally
excluded from the comparison (they are not part of the library's API):
`_init`, `_fini`, `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__gmon_start__`, `__cxa_finalize`, `rust_eh_personality` and the Rust
allocator/panic shims.

## Undefined (imported) symbols

C library imports from libm/libc: `expf` (plus the standard `__cxa_finalize`,
`__gmon_start__`).
The Rust library declares `extern "C" { fn expf(x: f32) -> f32; }` so it
resolves the **same** platform `libm` symbol at run time rather than using
Rust's own `f32::exp`. This is required for bit-identical results.

```
$ nm -D --undefined-only translation/target/release/libgaussian_kernel_lib.so
```
shows only libc/libm symbols (`expf`, `memcpy`, `__cxa_finalize`, ...).

## Verdict

**0 missing symbols. 0 undefined non-libc symbols.** No C source file was left
untranslated; the whole library is one function and it is present in Rust with
the exact same name via `#[no_mangle] pub unsafe extern "C" fn`.

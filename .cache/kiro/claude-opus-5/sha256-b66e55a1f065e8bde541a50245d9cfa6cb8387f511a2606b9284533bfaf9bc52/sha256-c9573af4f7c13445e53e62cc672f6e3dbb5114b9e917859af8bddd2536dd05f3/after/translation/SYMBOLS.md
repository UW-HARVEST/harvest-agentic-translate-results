# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

- C `.so`: `c_src/build/libharvest-work-QWzLbB.so` (built by `c_src/CMakeLists.txt`,
  single translation unit `src/lib.c`, no `CMAKE_BUILD_TYPE` set → `-O0`).
- Rust `.so`: `translation/target/release/libhsl_to_rgb_lib.so`
  (`crate-type = ["cdylib"]`, `name = "hsl_to_rgb_lib"`).

## Regeneration

```sh
nm -D --defined-only c_src/build/libharvest-work-QWzLbB.so
nm -D --defined-only translation/target/release/libhsl_to_rgb_lib.so
```

## Defined (exported) symbols

| # | C symbol (`nm -D --defined-only`) | type | present in Rust `.so` | Rust definition |
|---|-----------------------------------|------|-----------------------|-----------------|
| 1 | `hsl_to_rgb`                      | `T`  | YES                   | `#[unsafe(no_mangle)] pub unsafe extern "C" fn hsl_to_rgb` in `src/lib.rs` |

`c_src/include/lib.h` declares exactly one prototype, `void hsl_to_rgb(float *dest, const float *src);`.
There are no namespace/renaming macros, no macro-generated symbol families, no
`#ifdef`-gated additional entry points, and no additional `.c` files in
`CMakeLists.txt`. So the C `.so` exports exactly one non-libc symbol and the
translation is complete at file granularity — no C module was skipped.

**Missing-from-Rust count: 0.** No `#[no_mangle]` wrapper had to be added and no
untranslated C module was found.

## Undefined symbols (imports)

C `.so` imports, excluding weak toolchain stubs
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`):

| symbol | source |
|--------|--------|
| `fmodf@GLIBC_2.2.5` | libc/libm |

Rust `.so` imports: only libc (`malloc`, `free`, `memcpy`, `mmap64`, `open64`,
`read`, `write`, `pthread_key_*`, `__errno_location`, `abort`, …) and libgcc
unwinder symbols (`_Unwind_*`). These come from the Rust standard library that
`cdylib` links in; they are all platform runtime symbols.

**Undefined non-libc / non-runtime symbols in the Rust `.so`: 0.**

Note: Rust's `f32 % f32` lowers to LLVM `frem`, which the backend expands
inline for `f32` in this build, so `fmodf` does not appear as an import name in
the Rust `.so`. The differential tests in `tests/differential.rs` verify the
numeric result matches glibc `fmodf` bit-for-bit over the tested domain rather
than relying on symbol identity.

## Symbol diff

```
$ comm -3 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort) \
          <(nm -D --defined-only RUST.so | awk '{print $NF}' | sort)
(empty)
```

Symbol diff is EMPTY. Phase D symbol-parity gate: PASS.

# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-L9tH1h.so`
* Rust `.so`: `translation/target/release/libtfm_lib.so`

## Public headers

`c_src/include/lib.h` is a single line and declares exactly one entry point:

```c
void tfm(float *dest, const float *src, int count);
```

`c_src/CMakeLists.txt` builds one translation unit (`src/lib.c`) into one
`SHARED` library. There is no second module, no binary/driver target, and no
`#ifdef`-gated source, so there is no untranslated C file.

## `nm -D --defined-only` — C `.so`

| # | symbol | type | also exported by Rust `.so`? |
|---|--------|------|------------------------------|
| 1 | `tfm`  | `T`  | YES (`#[no_mangle] pub unsafe extern "C" fn tfm`) |

Non-`tfm` defined symbols in the C `.so`: none. (`nm -D --defined-only` on the
C library prints exactly one line.)

## `nm -D -u` — undefined (imported) symbols

C `.so` imports:

| symbol | note |
|--------|------|
| `sqrtf@GLIBC_2.2.5` | `<math.h>`; modelled in Rust by `f32::sqrt` (`sqrtss`) plus explicit NaN quieting |
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__` | weak, toolchain boilerplate |

Rust `.so` imports: the same weak toolchain symbols plus libc / `libgcc`
unwinder symbols pulled in by the Rust standard library (`malloc`, `memcpy`,
`_Unwind_*`, `__errno_location`, …). All are libc/compiler-runtime symbols, not
symbols of the library under translation.

## Completion check

- [x] Every symbol defined by the C `.so` is defined by the Rust `.so` with the
      exact same name. Symbol diff is **empty**.
- [x] 0 missing symbols; 0 undefined non-libc / non-compiler-runtime symbols in
      the Rust `.so`.
- [x] No stubs, no `unimplemented!()`, no faked exports.

Reproduce with:

```sh
diff <(nm -D --defined-only c_src/build/libharvest-work-L9tH1h.so | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libtfm_lib.so | awk '{print $3}' | sort)
```

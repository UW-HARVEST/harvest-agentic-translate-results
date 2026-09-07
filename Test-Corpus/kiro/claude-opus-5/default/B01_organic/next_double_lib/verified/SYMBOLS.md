# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-92FIXf.so
nm -D --defined-only translation/target/release/libnext_double_lib.so
```

## C translation units

`c_src/CMakeLists.txt` compiles exactly one translation unit:

* `src/lib.c` — the only C source file in the project.

There is **no** additional C module, and **no** binary/driver executable target
(`add_library(... SHARED src/lib.c)` is the only target). Therefore there is no
"whole module never translated" gap to close: `translation/src/lib.rs` covers
100% of the C source.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | kind | notes |
|---|--------|---------|------------|------|-------|
| 1 | `next_double` | `T` (defined, global) | `T` (defined, global) | `double next_double(cn_rnd_t *)` | Exported from Rust via `#[unsafe(no_mangle)] pub unsafe extern "C" fn next_double`. |

## Non-exported C symbols (intentionally not in the ABI)

| symbol | C linkage | Rust counterpart | must be exported? |
|--------|-----------|------------------|-------------------|
| `cn_rnd_next` | `static` (internal, file-local; no `nm -D` entry) | private `fn cn_rnd_next` | No — `static` in C has internal linkage, so exporting it from Rust would *add* a symbol the C `.so` does not have. |

## Types crossing the ABI

| C type | Rust type | layout check |
|--------|-----------|--------------|
| `typedef struct cn_rnd_t { uint64_t state[2]; }` | `#[repr(C)] pub struct cn_rnd_t { pub state: [u64; 2] }` | 16 bytes, align 8, both sides. Verified in `tests/differential.rs::abi_struct_layout`. |
| `double` | `std::ffi::c_double` (`f64`) | IEEE-754 binary64, 8 bytes. |

## Diff result

```
$ diff <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only RUST.so| awk '{print $3}' | sort | grep -v '^_')
(empty)
```

**Missing from Rust `.so`: 0.**
**Undefined non-libc symbols in Rust `.so`: 0** (only the usual toolchain weak
undefineds `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`, `__tls_get_addr`, which are `w`/libc and are
also present in ordinary C shared objects).

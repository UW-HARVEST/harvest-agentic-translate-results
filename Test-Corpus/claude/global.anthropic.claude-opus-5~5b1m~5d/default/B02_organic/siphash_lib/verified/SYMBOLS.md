# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-XEIFF4.so`
* Rust `.so`: `translation/target/{debug,release}/libsiphash_lib.so`

Regenerate with:

```sh
nm -D --defined-only c_src/build/libharvest-work-XEIFF4.so
nm -D --defined-only translation/target/release/libsiphash_lib.so
```

## C `.so` — defined dynamic symbols (`nm -D --defined-only`)

```
0000000000001547 T siphash
000000000000151a T stbds_hash_bytes
```

That is the complete list. `c_src/src/lib.c` contains exactly one more
function, `stbds_siphash_bytes`, which is `static` and therefore **not** part
of the dynamic surface (it is inlined/called internally only). It must NOT be
exported by the Rust `.so` either.

## Symbol parity table

| # | symbol | C type | signature (from `c_src/src/lib.c` / `include/lib.h`) | in C `.so` | in Rust `.so` | status |
|---|--------|--------|------------------------------------------------------|-----------|---------------|--------|
| 1 | `siphash`          | `T` (global text) | `void siphash(int init)`                              | yes | yes | OK |
| 2 | `stbds_hash_bytes` | `T` (global text) | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | yes | yes | OK |

### Deliberately-not-exported (internal linkage in C)

| symbol | reason |
|--------|--------|
| `stbds_siphash_bytes` | `static` in `c_src/src/lib.c`; not in C `nm -D`. Rust keeps it as a private `fn`. |

## Diff result

```
$ comm -23 <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
           <(nm -D --defined-only RUST.so | awk '{print $3}' | sort)
<empty>
```

**0 symbols missing from the Rust `.so`.** No `#[no_mangle]` wrappers had to be
added and no C module was left untranslated — `c_src` consists of a single
translation unit (`src/lib.c`, 126 lines) and all three of its functions are
present in `translation/src/lib.rs`.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports (`printf`, `puts`, `memcpy`, `malloc`, `_Unwind_*`, …), i.e. the same
class of imports the C `.so` has (`printf`, `puts`, `__cxa_finalize`, …).

**0 missing/undefined non-libc symbols.**

Note: the C `.so` imports `puts` in addition to `printf` because GCC rewrites
the constant-format call `printf(" },\n")` into `puts(" },")`. This is a pure
codegen detail — the bytes written to `stdout` are identical, which the
`siphash_*` stdout-differential tests verify directly.

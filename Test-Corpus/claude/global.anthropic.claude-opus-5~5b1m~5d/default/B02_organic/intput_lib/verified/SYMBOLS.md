# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on the C shared library
`c_src/build/libharvest-work-zSimsg.so`, compared with the Rust cdylib
`translation/target/release/libintput_lib.so`.

Regenerate with:

```sh
nm -D --defined-only c_src/build/libharvest-work-zSimsg.so | awk '{print $3}' | sort > /tmp/c.txt
nm -D --defined-only translation/target/release/libintput_lib.so | awk '{print $3}' | sort > /tmp/r.txt
diff /tmp/c.txt /tmp/r.txt      # MUST be empty
```

## Exported symbol table

| # | symbol | C type | in C .so | in Rust .so | notes |
|---|--------|--------|----------|-------------|-------|
| 1 | `stbds_arrgrowf`     | `void *(void*, size_t, size_t, size_t)` | T | T | dynamic-array grow |
| 2 | `stbds_arrfreef`     | `void (void*)` | T | T | frees `stbds_header(a)`; UB on NULL in both |
| 3 | `stbds_rand_seed`    | `void (size_t)` | T | T | sets the file-static `stbds_hash_seed` |
| 4 | `stbds_hash_string`  | `size_t (char*, size_t)` | T | T | rotate/mix string hash |
| 5 | `stbds_hash_bytes`   | `size_t (void*, size_t, size_t)` | T | T | wraps static `stbds_siphash_bytes` |
| 6 | `stbds_hmfree_func`  | `void (void*, size_t)` | T | T | frees map + strdup'd keys + arena |
| 7 | `stbds_hmget_key_ts` | `void *(void*, size_t, void*, size_t, ptrdiff_t*, int)` | T | T | thread-safe lookup, index via `temp` out-param |
| 8 | `stbds_hmget_key`    | `void *(void*, size_t, void*, size_t, int)` | T | T | lookup, index stored in header `temp` |
| 9 | `stbds_hmput_default`| `void *(void*, size_t)` | T | T | materialises slot `[-1]` |
| 10 | `stbds_hmput_key`   | `void *(void*, size_t, void*, size_t, int)` | T | T | insert/replace, grow/rehash |
| 11 | `stbds_shmode_func` | `void *(size_t, int)` | T | T | creates map with `string.mode = (unsigned char) mode` |
| 12 | `stbds_hmdel_key`   | `void *(void*, size_t, void*, size_t, size_t, int)` | T | T | delete, tombstone, shrink/rebuild |
| 13 | `stbds_stralloc`    | `char *(stbds_string_arena*, char*)` | T | T | arena string allocator |
| 14 | `stbds_strreset`    | `void (stbds_string_arena*)` | T | T | frees arena block chain, zeroes arena |
| 15 | `strkey`            | `char *(int)` | T | T | `sprintf(buffer, "test_%d", n)` into a 256-byte static |
| 16 | `intput`            | `void (int)` | T | T | the only symbol in `include/lib.h` |

## Not exported (correctly absent from both)

These are `static` in `c_src/src/lib.c` and therefore never in the dynamic
symbol table; the Rust translation likewise keeps them private:

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`, `stbds_hash_seed`, `buffer`.

`stbds_unit_tests` is *declared* `extern` in the C source but never defined and
never called, so it is not a defined symbol in the C `.so` either.

## Result

```
$ diff c_syms.txt r_syms.txt
(no output)
```

**0 missing symbols, 0 extra symbols.** Undefined (imported) symbols in the
Rust `.so` are libc only (`realloc`, `free`, `__assert_fail`, plus the Rust
runtime's `memcpy`/`memset`/unwind stubs), matching the C `.so`'s libc imports.

## Automated check

`tests/phase_d.rs::d_01_symbol_parity` shells out to `nm` on both `.so`s and
fails if either direction of the set difference is non-empty; it also pins the
count at 16 so a newly added C symbol cannot silently slip through.
`d_02_rust_so_has_no_unexpected_undefined_symbols` asserts the Rust `.so` has no
*undefined* `stbds_*` / `intput` / `strkey` import (i.e. nothing was left as an
unresolved reference to the C library).

```
$ cargo test --offline --test phase_d -- --test-threads=1
test d_01_symbol_parity ... ok
test d_02_rust_so_has_no_unexpected_undefined_symbols ... ok
```

No symbol needed a new `#[no_mangle]` wrapper and no C module was missing: the
whole of `c_src/src/lib.c` (one translation unit) is present in
`translation/src/lib.rs`, including the non-`stbds_` test helpers `strkey` and
`intput`.

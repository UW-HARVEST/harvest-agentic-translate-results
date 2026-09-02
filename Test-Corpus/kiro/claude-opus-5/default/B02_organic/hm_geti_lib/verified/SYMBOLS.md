# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-aena63.so   | awk '{print $3}' | sort -u
nm -D --defined-only translation/target/release/libhm_geti_lib.so | awk '{print $3}' | sort -u
```

The C translation unit is `c_src/src/lib.c` (the `STB_DS_IMPLEMENTATION` body of
`stb_ds.h` plus the small `strkey` / `hm_geti` driver at the bottom).

## Public (dynamic, defined) symbols

| # | symbol | C `.so` | Rust `.so` | Rust definition site |
|---|--------|---------|------------|----------------------|
| 1 | `hm_geti`             | yes | yes | `src/unit_tests.rs` |
| 2 | `strkey`              | yes | yes | `src/unit_tests.rs` |
| 3 | `stbds_arrgrowf`      | yes | yes | `src/arr.rs` |
| 4 | `stbds_arrfreef`      | yes | yes | `src/arr.rs` |
| 5 | `stbds_rand_seed`     | yes | yes | `src/hash.rs` |
| 6 | `stbds_hash_bytes`    | yes | yes | `src/hash.rs` |
| 7 | `stbds_hash_string`   | yes | yes | `src/hash.rs` |
| 8 | `stbds_stralloc`      | yes | yes | `src/strings.rs` |
| 9 | `stbds_strreset`      | yes | yes | `src/strings.rs` |
| 10 | `stbds_hmfree_func`  | yes | yes | `src/hashmap.rs` |
| 11 | `stbds_hmget_key`    | yes | yes | `src/hashmap.rs` |
| 12 | `stbds_hmget_key_ts` | yes | yes | `src/hashmap.rs` |
| 13 | `stbds_hmput_default`| yes | yes | `src/hashmap.rs` |
| 14 | `stbds_hmput_key`    | yes | yes | `src/hashmap.rs` |
| 15 | `stbds_hmdel_key`    | yes | yes | `src/hashmap.rs` |
| 16 | `stbds_shmode_func`  | yes | yes | `src/hashmap.rs` |

**Symbol diff (`comm -23 c_syms rs_syms`): EMPTY.** 16 / 16 present, exact names.

## `static` (internal, not exported) C functions

These are `static` in C so they are absent from `nm -D` in both builds. All are
translated (as private Rust `fn`s) and are exercised indirectly:

| C static symbol | Rust counterpart |
|---|---|
| `stbds_probe_position`   | `hashmap::stbds_probe_position` |
| `stbds_log2`             | `hashmap::stbds_log2` |
| `stbds_make_hash_index`  | `hashmap::stbds_make_hash_index` |
| `stbds_siphash_bytes`    | `hash::stbds_siphash_bytes` |
| `stbds_is_key_equal`     | `hashmap::stbds_is_key_equal` |
| `stbds_hm_find_slot`     | `hashmap::stbds_hm_find_slot` |
| `stbds_strdup`           | `strings::stbds_strdup` |
| `stbds_hash_seed` (data) | `hash::stbds_hash_seed` |
| `buffer` (data)          | `unit_tests::buffer` |

`stbds_unit_tests` is *declared* `extern` in the C source but never defined, so
it is exported by neither `.so`. Correct — do not add it.

## Undefined (imported) symbols

C imports only libc: `malloc realloc free memcmp memcpy memmove memset sprintf
strcmp strlen __assert_fail` (+ CRT/ITM stubs).

Rust imports the same libc set (`memcmp` is lowered to `bcmp` by LLVM, which is
libc) plus the Rust std runtime's own libc/unwind imports. **0 missing or
undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** — there are exactly two
resolvable configurations, `DEFAULT` and `--no-default-features`, and they are
the same crate. `verify.sh` enumerates combinations out of `Cargo.toml`
generically (so adding a feature later cannot silently skip coverage) and runs
`cargo check`, the release build, the `nm -D` diff and all five test suites for
each one.

Result:

```
[DEFAULT]    C exports 16, Rust exports 16, symbol diff EMPTY, no undefined non-libc
[NO_DEFAULT] C exports 16, Rust exports 16, symbol diff EMPTY, no undefined non-libc
```

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` diff empty (16/16), 0 undefined non-libc symbols in
      the Rust `.so`. No stubs — `grep -E 'unimplemented!|todo!'` over `src/`
      returns nothing.
- [x] Phase B: every one of the 38 `CONFIGS.md` rows plus 5 cross-cutting stress
      rows passes across randomized inputs (fixed seeds).
- [x] Phase C: every one of the 55 `ERRORS.md` rows and 9 generic FFI-boundary
      rows has a passing differential test; 0 unchecked boxes.
- [x] Both feature configurations hold: 91 tests green under `DEFAULT` and under
      `--no-default-features`.

Reproduce with:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation && ./verify.sh all
```

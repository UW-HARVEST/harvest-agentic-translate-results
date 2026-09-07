# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Sources compared:

* C   : `c_src/build/libharvest-work-oeLV3y.so` (cmake, `src/lib.c`, links `-lm`)
* Rust: `translation/target/release/libstr_put_lib.so` (`crate-type = ["cdylib"]`)

Commands:

```sh
nm -D --defined-only c_src/build/libharvest-work-oeLV3y.so   | awk '{print $2,$3}' | sort
nm -D --defined-only translation/target/release/libstr_put_lib.so | awk '{print $2,$3}' | sort
```

`Cargo.toml` declares **no `[features]` section**, so there is exactly one
build configuration (`--no-default-features` and the default build are the same
`.so`). Feature-combination sweep is therefore the single default combination.

## Dynamic symbol table (`nm -D`), defined symbols

| # | symbol | C type | Rust type | Rust site | status |
|---|--------|--------|-----------|-----------|--------|
| 1 | `stbds_arrfreef`     | T | T | `src/arr.rs`     | OK |
| 2 | `stbds_arrgrowf`     | T | T | `src/arr.rs`     | OK |
| 3 | `stbds_hash_bytes`   | T | T | `src/hash.rs`    | OK |
| 4 | `stbds_hash_string`  | T | T | `src/hash.rs`    | OK |
| 5 | `stbds_hmdel_key`    | T | T | `src/hash.rs`    | OK |
| 6 | `stbds_hmfree_func`  | T | T | `src/hash.rs`    | OK |
| 7 | `stbds_hmget_key`    | T | T | `src/hash.rs`    | OK |
| 8 | `stbds_hmget_key_ts` | T | T | `src/hash.rs`    | OK |
| 9 | `stbds_hmput_default`| T | T | `src/hash.rs`    | OK |
| 10 | `stbds_hmput_key`   | T | T | `src/hash.rs`    | OK |
| 11 | `stbds_rand_seed`   | T | T | `src/hash.rs`    | OK |
| 12 | `stbds_shmode_func` | T | T | `src/hash.rs`    | OK |
| 13 | `stbds_stralloc`    | T | T | `src/strings.rs` | OK |
| 14 | `stbds_strreset`    | T | T | `src/strings.rs` | OK |
| 15 | `str_put`           | T | T | `src/testapi.rs` | OK |
| 16 | `strkey`            | T | T | `src/testapi.rs` | OK |

**Missing from Rust `.so`: 0.**

```
$ comm -23 <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
           <(nm -D --defined-only RUST.so| awk '{print $3}' | sort)
(empty)
```

## Non-exported C internals (`static`, correctly absent from both `nm -D`)

Translated but intentionally not exported — they are `static` in C, so they must
NOT appear in `nm -D`:

| C symbol | Rust counterpart |
|----------|------------------|
| `stbds_hash_seed` (static var) | `types::STBDS_HASH_SEED` |
| `buffer` (static var)          | `testapi::BUFFER` |
| `stbds_probe_position`         | `hash::stbds_probe_position` |
| `stbds_log2`                   | `hash::stbds_log2` |
| `stbds_make_hash_index`        | `hash::stbds_make_hash_index` |
| `stbds_siphash_bytes`          | `hash::stbds_siphash_bytes` |
| `stbds_is_key_equal`           | `hash::stbds_is_key_equal` |
| `stbds_hm_find_slot`           | `hash::stbds_hm_find_slot` |
| `stbds_strdup`                 | `strings::stbds_strdup` |

Verified: neither `.so` exports any of these.

## Undefined (imported) symbols

The Rust `.so` imports only libc, libgcc-unwind and libpthread symbols — no
undefined symbol belongs to the library itself.

One difference is worth naming explicitly: the C `.so` imports `memcmp`, the
*release* Rust `.so` imports `bcmp` instead (the debug build imports both).
`src/hash.rs` does declare and call `c::memcmp`, but every call site is
`0 == memcmp(...)`, so LLVM rewrites it to glibc's `bcmp`, which answers exactly
that equality question. Behaviour is unchanged; the differential tests in
`tests/phase_c_errors.rs` (`g2_keysize_zero`, `g3_keysize_covers_whole_element`)
and the randomized binary-map pipelines cover the key-comparison path directly.

`verify.sh` step 3 enforces this mechanically: it strips the libc / unwinder /
pthread allowlist from `nm -D --undefined-only` and fails if anything is left.

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.

## Completion gate (Phase D)

```
$ ./verify.sh
C exports:    16
Rust exports: 16 of them
symbol diff: EMPTY (0 missing)
--- undefined non-libc symbols in the Rust .so ---
none
Cargo.toml declares no [features]; the only configuration is the default.
test result: ok. 17 passed   (tests/phase_b_arr.rs)
test result: ok. 15 passed   (tests/phase_b_hash.rs)
test result: ok. 43 passed   (tests/phase_b_map.rs)
test result: ok.  7 passed   (tests/phase_b_strput.rs)
test result: ok. 53 passed   (tests/phase_c_errors.rs)
PHASE D: ALL CHECKS PASSED
```

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing and 0 undefined non-libc symbols in Rust.
- [x] Phase B: every one of the 80 `CONFIGS.md` rows passes across randomized inputs.
- [x] No binary executable is produced by `c_src/CMakeLists.txt` (it declares a
      single `add_library(... SHARED ...)`), so the only program output the
      library can produce is `str_put`'s `printf`; that is compared byte-for-byte
      through a redirected fd 1 in `tests/phase_b_strput.rs`.
- [x] Phase C: every one of the 52 `ERRORS.md` rows plus G1–G7 has a passing
      error-path differential test.
- [x] All of the above hold under every feature combination — `Cargo.toml`
      declares no `[features]`, so `--no-default-features` and the default build
      are the same configuration, and `verify.sh` runs the sweep either way.
      The suite also passes in both the `dev` and `release` profiles.

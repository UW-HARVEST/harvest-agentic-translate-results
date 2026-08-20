# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Source of truth: `nm -D --defined-only` on the C shared library built from
`c_src/` (`c_src/build/libtranslated_rust.so`) vs. the Rust cdylib
(`target/release/libstr_dups_lib.so`).

Reproduce with:

```sh
./check_symbols.sh
```

## Build-time configurations

* `c_src/CMakeLists.txt` — a single target, `add_library(<dir> SHARED src/lib.c)`,
  linked against `m`. **No** `option()`, `add_definitions`, `target_compile_definitions`
  or `#ifdef`-driven variants exist ⇒ exactly **one** C configuration.
* `translated_rust/Cargo.toml` — has **no `[features]` section** ⇒ the only valid
  feature combination is the empty set (`--no-default-features`, which is
  identical to the default build).  See `CONFIGS.md` §0.

Therefore Phase D's "every feature combination" reduces to the single
configuration, but the loop is still automated in `check_features.sh`.

## C exports (16) → all present in Rust

| # | symbol | C addr | Rust addr | defined in Rust |
|---|--------|--------|-----------|-----------------|
| 1 | `stbds_arrgrowf`      | `0x1259` | `0x12de0` | `src/stb_ds.rs` |
| 2 | `stbds_arrfreef`      | `0x13c8` | `0x12dd0` | `src/stb_ds.rs` |
| 3 | `stbds_rand_seed`     | `0x13e7` | `0x13dc0` | `src/stb_ds.rs` |
| 4 | `stbds_hash_string`   | `0x190c` | `0x13110` | `src/stb_ds.rs` |
| 5 | `stbds_hash_bytes`    | `0x1da6` | `0x12e80` | `src/stb_ds.rs` |
| 6 | `stbds_hmfree_func`   | `0x1e67` | `0x133f0` | `src/stb_ds.rs` |
| 7 | `stbds_hmget_key_ts`  | `0x217e` | `0x13580` | `src/stb_ds.rs` |
| 8 | `stbds_hmget_key`     | `0x22cb` | `0x134c0` | `src/stb_ds.rs` |
| 9 | `stbds_hmput_default` | `0x2334` | `0x13630` | `src/stb_ds.rs` |
| 10 | `stbds_hmput_key`    | `0x23de` | `0x136e0` | `src/stb_ds.rs` |
| 11 | `stbds_shmode_func`  | `0x2b8e` | `0x13dd0` | `src/stb_ds.rs` |
| 12 | `stbds_hmdel_key`    | `0x2c1c` | `0x13170` | `src/stb_ds.rs` |
| 13 | `stbds_stralloc`     | `0x3081` | `0x13ee0` | `src/stb_ds.rs` |
| 14 | `stbds_strreset`     | `0x3254` | `0x13ff0` | `src/stb_ds.rs` |
| 15 | `strkey`             | `0x32ac` | `0x14470` | `src/str_dups.rs` |
| 16 | `str_dups`           | `0x32e3` | `0x14040` | `src/str_dups.rs` |

`comm -23 c_syms r_syms` (missing in Rust) → **empty**.
`comm -13 c_syms r_syms` (extra in Rust) → **empty**.

## Static / internal C functions (no dynamic symbol, translated anyway)

These are `static` in `c_src/src/lib.c`, so they are absent from `nm -D` on the
C `.so`.  They are still fully translated because the exported functions call
them; they are *not* exported from the Rust `.so` either (parity preserved).

| C `static` symbol | Rust counterpart |
|-------------------|------------------|
| `stbds_hash_seed` (static var) | `stb_ds::STBDS_HASH_SEED` |
| `stbds_probe_position`  | `stb_ds::stbds_probe_position` |
| `stbds_log2`            | `stb_ds::stbds_log2` |
| `stbds_make_hash_index` | `stb_ds::stbds_make_hash_index` |
| `stbds_siphash_bytes`   | `stb_ds::stbds_siphash_bytes` |
| `stbds_is_key_equal`    | `stb_ds::stbds_is_key_equal` |
| `stbds_hm_find_slot`    | `stb_ds::stbds_hm_find_slot` |
| `stbds_strdup`          | `stb_ds::stbds_strdup` |
| `buffer` (static array) | `str_dups::BUFFER` |

## Declared-but-never-defined in the C TU

`c_src/src/lib.c` line 83 declares `extern void stbds_unit_tests(void);` but
never defines it, and nothing references it ⇒ it appears in **neither** `.so`
(`nm -D` on the C library does not list it).  Correctly absent from Rust.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only target/release/libstr_dups_lib.so` lists only libc /
libgcc-unwind imports (`realloc`, `free`, `memmove`, `memcpy`, `bcmp`,
`strcmp`, `strlen`, `printf`, `sprintf`, `__assert_fail`, `abort`, the
`_Unwind_*` / `__cxa_*` / `pthread_*` runtime helpers used by `std`).

**0 missing / undefined non-libc symbols.**  ✅

## ABI data layout parity

The structures shared through the returned opaque pointers are `#[repr(C)]` in
`src/ffi.rs` with compile-time size assertions that match the C (LP64) layout:

| struct | C `sizeof` | Rust `size_of` |
|--------|-----------|----------------|
| `stbds_array_header` | 32 | 32 |
| `stbds_string_block` | 16 | 16 |
| `stbds_string_arena` | 24 | 24 |
| `stbds_hash_bucket`  | 128 | 128 |
| `stbds_hash_index`   | 104 | 104 |

Verified at runtime by `tests/differential.rs::abi_layout_matches_c` which
derives the C layout from live `.so` behaviour (header offsets observed through
`stbds_arrgrowf` / `stbds_shmode_func`).

## Assertion payload parity

`STBDS_ASSERT` is `assert()`, `NDEBUG` is never defined ⇒ live in both builds.
`strings` on the C `.so` yields exactly the expression texts and
`__PRETTY_FUNCTION__` names hard-coded in the Rust translation:

```
t->used_count_threshold + t->tombstone_count_threshold < t->slot_count   (line 401, stbds_make_hash_index)
(size_t) i+1 <= stbds_arrcap(a)                                         (line 778, stbds_hmput_key)
slot < (ptrdiff_t) table->slot_count                                    (line 828, stbds_hmdel_key)
slot >= 0                                                               (line 846, stbds_hmdel_key)
b->index[i] == final_index                                              (line 849, stbds_hmdel_key)
len <= a->remaining                                                     (line 913, stbds_stralloc)
*strmap[0].key == 'a'                                                   (line 960, str_dups)
strmap[0].key != s.key                                                  (line 961, str_dups)
strmap[0].value == s.value                                              (line 962, str_dups)
```

`STBDS_ASSERT(table->used_count >= 0)` (C line 832) compares a `size_t` against
`0`; it is a tautology, GCC folds it away and emits no string for it — the Rust
translation likewise omits it.  ✔ parity.

One unavoidable difference: the C `__FILE__` baked into the `.so` is the
*absolute path used at cmake time*
(`/local/.../translated_rust/c_src/src/lib.c`), while the Rust translation
hard-codes `src/lib.c`.  This string is a property of *where the C library was
compiled*, not of the library's behaviour (it changes if you move the checkout),
so the differential assertion tests compare `line`, `function` and the
assertion *expression* plus the resulting `SIGABRT`, and ignore the directory
prefix of the path.  See `ERRORS.md` note (F).

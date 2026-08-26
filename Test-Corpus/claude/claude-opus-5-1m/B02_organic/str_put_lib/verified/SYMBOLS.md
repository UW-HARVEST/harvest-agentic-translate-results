# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cargo build --release
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort -u > c.txt
nm -D --defined-only target/release/libstr_put_lib.so   | awk '{print $3}' | sort -u > r.txt
comm -23 c.txt r.txt      # missing from Rust  -> MUST be empty
comm -13 c.txt r.txt      # extra in Rust      -> informational
```

## Build-time configuration surface

* `c_src/CMakeLists.txt` builds exactly one target from exactly one TU
  (`src/lib.c`); there are no `option()`s, no `#ifdef`-gated features, no
  `-D` flags. `CMAKE_BUILD_TYPE` is unset, therefore **`NDEBUG` is NOT
  defined and `assert()` is live** (confirmed: `__assert_fail@GLIBC_2.2.5`
  appears in `nm -D --undefined-only` of the C `.so`).
* `Cargo.toml` has **no `[features]` table**. The complete set of valid
  feature combinations is therefore the single empty combination:

  | # | combination | command |
  |---|-------------|---------|
  | 1 | *(none / default)* | `cargo check --no-default-features` |
  | 1'| *(none / default)* | `cargo check` (identical — no default features exist) |

  Both were run and both compile cleanly with zero errors and zero warnings.
  There is no backend/module that needs `#[cfg(feature = "…")]` gating.

## Symbol table (16/16 exported by both)

`T` = global text symbol in the dynamic symbol table.

| # | symbol | C `.so` | Rust `.so` | C definition site |
|---|--------|---------|-----------|-------------------|
|  1 | `stbds_arrgrowf`      | T | T | `lib.c:276` |
|  2 | `stbds_arrfreef`      | T | T | `lib.c:312` |
|  3 | `stbds_rand_seed`     | T | T | `lib.c:355` |
|  4 | `stbds_hash_string`   | T | T | `lib.c:477` |
|  5 | `stbds_hash_bytes`    | T | T | `lib.c:553` |
|  6 | `stbds_hmfree_func`   | T | T | `lib.c:571` |
|  7 | `stbds_hmget_key_ts`  | T | T | `lib.c:631` |
|  8 | `stbds_hmget_key`     | T | T | `lib.c:659` |
|  9 | `stbds_hmput_default` | T | T | `lib.c:667` |
| 10 | `stbds_hmput_key`     | T | T | `lib.c:680` |
| 11 | `stbds_shmode_func`   | T | T | `lib.c:796` |
| 12 | `stbds_hmdel_key`     | T | T | `lib.c:807` |
| 13 | `stbds_stralloc`      | T | T | `lib.c:881` |
| 14 | `stbds_strreset`      | T | T | `lib.c:920` |
| 15 | `strkey`              | T | T | `lib.c:939` |
| 16 | `str_put`             | T | T | `lib.c:945` |

### `static` (non-exported) C helpers — correctly NOT in either `.so`

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`, and the `static char buffer[256]` / `static size_t
stbds_hash_seed` objects. All of these are translated in `src/lib.rs` as
private Rust items, so no export wrapper is required or wanted.

### Header-declared but never defined in this TU

`c_src/src/lib.c` also `extern`-declares `stbds_unit_tests(void)` (line 83).
It is **never defined**, so the C `.so` does not export it and neither does
the Rust `.so`. Adding it to Rust would be an *extra* symbol, not parity —
deliberately omitted.

## Result

```
comm -23 c.txt r.txt   ->   (empty)
comm -13 c.txt r.txt   ->   (empty)
```

* **0 symbols missing from the Rust `.so`.**
* **0 extra symbols in the Rust `.so`.**
* Every `nm -D --undefined-only` entry of the Rust `.so` is libc / libgcc
  unwinder / `ld.so` (`malloc`, `realloc`, `free`, `memcpy`, `memmove`,
  `memset`, `bcmp`, `strlen`, `strcmp`, `printf`, `sprintf`, `abort`,
  `_Unwind_*`, `__cxa_*`, `dl_iterate_phdr`, …) — **0 undefined non-libc
  symbols**.

---

## Completion gate (re-verified after all fixes)

| gate | evidence |
|------|----------|
| `nm -D`: 0 missing / 0 extra symbols in the Rust `.so` | `comm -23` and `comm -13` both empty for **both** `target/debug/libstr_put_lib.so` and `target/release/libstr_put_lib.so` (16/16 symbols each) |
| `nm -D`: 0 undefined non-libc symbols in the Rust `.so` | every `--undefined-only` entry matches `@GLIBC` / `@GCC` / `_Unwind*` / `__cxa*` / `_ITM_*` / `__gmon_start__` / `__tls_get_addr` |
| Phase B: every `CONFIGS.md` row passes over randomized inputs | 47/47 rows — see `CONFIGS.md` "Row status" |
| Phase C: every `ERRORS.md` row has a passing differential test | 45/45 rows — 40 tested directly, 5 proved unreachable *and* their `assert` ported anyway (`ERRORS.md` note A) |
| all of the above under **every** feature combination | `Cargo.toml` has no `[features]`, so the single combination is the empty one; `cargo check --no-default-features` and `cargo check` both clean |
| all of the above in **both** cargo profiles | `./run_tests.sh` and `./run_tests.sh --release`: **79 passing test functions, 0 failures** in each |
| the harness is not vacuous | 22 injected mutants, 21 caught, 1 provably equivalent — see `CONFIGS.md` "Mutation evidence" |
| `c_src/` unmodified | `CMakeLists.txt`, `include/lib.h`, `src/lib.c`, `license.txt` all still at their original mtime; only the required `c_src/build/` was added |

### How to reproduce

```sh
./run_tests.sh            # debug profile
./run_tests.sh --release  # release profile (the shipped cdylib)
```

`run_tests.sh` builds the C reference `.so`, runs `cargo check` for every valid
feature combination, builds the cdylib in both the selected profile **and**
release (the crash-parity tests need the release artifact), diffs `nm -D`, and
runs the whole differential suite single-threaded.

> `cargo test` alone is **not** sufficient: cargo does not rebuild a
> `crate-type = ["cdylib"]` target for test runs, so the tests would silently
> load a stale `.so`. `tests/common/mod.rs::assert_fresh` now refuses to run if
> the `.so` is older than `src/lib.rs`.

## Test-suite layout

| file | phase | contents |
|------|-------|----------|
| `tests/common/mod.rs` | — | `libloading` loader for both `.so`s, layout mirrors of the C structs, full-state `snapshot_map`, `MapPair` lock-step driver, seeded xorshift PRNG, `stdout` capture |
| `tests/smoke.rs` | — | both libraries load and agree on `hash_bytes` / `hash_string` / `str_put` |
| `tests/phase_b_hash.rs` | B | `CONFIGS.md` rows 1–8 (hashing, seed chain, `arrgrowf`/`arrfreef`) |
| `tests/phase_b_maps.rs` | B | rows 9–22, 45, 46, 46b (`hmput_key`, `hmget_key`, `hmget_key_ts`, `hmput_default`, `shmode_func`, `hmfree_func`) |
| `tests/phase_b_del.rs` | B | rows 23–36 (`hmdel_key`, tombstones, rebuild, shrink, `keyoffset`, randomized op streams) |
| `tests/phase_b_arena.rs` | B | rows 37–44 (`stralloc`, `strreset`, `strkey`, `str_put`) |
| `tests/phase_c_errors.rs` | C | one test per `ERRORS.md` row, incl. subprocess crash-signal parity |

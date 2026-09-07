# SYMBOLS.md — Phase A symbol surface

Derived mechanically:

```
nm -D --defined-only c_src/build/libharvest-work-6M6KAs.so | awk '{print $3}' | sort
nm -D --defined-only translation/target/release/libhelxo_lib.so | awk '{print $3}' | sort
```

The C library is a single translation unit (`c_src/src/lib.c`) that inlines the
implementation half of `stb_ds.h` plus two extra functions (`strkey`, `helxo`).

## Exported symbols

| # | symbol | C signature | in C .so | in Rust .so |
|---|--------|-------------|----------|-------------|
| 1 | `helxo` | `void helxo(char letter)` | yes | yes |
| 2 | `strkey` | `char *strkey(int n)` | yes | yes |
| 3 | `stbds_rand_seed` | `void stbds_rand_seed(size_t seed)` | yes | yes |
| 4 | `stbds_hash_bytes` | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | yes | yes |
| 5 | `stbds_hash_string` | `size_t stbds_hash_string(char *str, size_t seed)` | yes | yes |
| 6 | `stbds_arrgrowf` | `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | yes | yes |
| 7 | `stbds_arrfreef` | `void stbds_arrfreef(void *a)` | yes | yes |
| 8 | `stbds_hmfree_func` | `void stbds_hmfree_func(void *p, size_t elemsize)` | yes | yes |
| 9 | `stbds_hmget_key` | `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes | yes |
| 10 | `stbds_hmget_key_ts` | `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | yes | yes |
| 11 | `stbds_hmput_default` | `void *stbds_hmput_default(void *a, size_t elemsize)` | yes | yes |
| 12 | `stbds_hmput_key` | `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | yes | yes |
| 13 | `stbds_hmdel_key` | `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | yes | yes |
| 14 | `stbds_shmode_func` | `void *stbds_shmode_func(size_t elemsize, int mode)` | yes | yes |
| 15 | `stbds_stralloc` | `char *stbds_stralloc(stbds_string_arena *a, char *str)` | yes | yes |
| 16 | `stbds_strreset` | `void stbds_strreset(stbds_string_arena *a)` | yes | yes |

## Non-exported (static) C helpers — translated but intentionally private

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`, static `stbds_hash_seed`, static `buffer[256]`.
All are `static` in C and therefore not in `nm -D`; they are private `fn`s in
Rust. They are exercised indirectly through the public API.

## Declared-but-never-defined in the C TU (not in the .so, must NOT be in Rust)

`stbds_unit_tests` — `extern`-declared at `lib.c:83`, never defined. Absent from
both `.so` files. Correct.

## Result

```
comm -23 c_syms.txt rust_syms.txt   # missing in Rust
<empty>
comm -13 c_syms.txt rust_syms.txt   # extra in Rust
<empty>
```

**0 missing, 0 extra. Symbol parity achieved.**

`ldd`/`nm -u` on the Rust `.so` shows only libc imports
(`realloc`, `free`, `memset`, `memcpy`, `memmove`, `memcmp`, `strcmp`,
`strlen`, `printf`, `sprintf`, `__assert_fail`) — the same allocator/CRT surface the C uses, so
memory allocated by one side can be freed by the other.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default (empty) feature set. `cargo check
--no-default-features` and `cargo check --all-features` are therefore identical
to `cargo check`. Enumerated mechanically from `Cargo.toml` and exercised by
`run_matrix.sh`.

The project builds **no binary executable** (`[lib] crate-type = ["cdylib"]`
only; the CMake project declares only `add_library(... SHARED ...)`), so the
"compare C and Rust binary stdout" gate is not applicable. `helxo` — the only
function that writes to stdout — is compared byte-for-byte by redirecting
`stdout` to a pipe from inside the test harness.

## Completion gate (Phase D) — re-verified

- [x] `SYMBOLS.md`: `nm -D` shows **0 missing** and **0 extra** symbols in the
      Rust `.so`; `nm -D --undefined-only` on the Rust `.so` contains no
      `stbds_*` / `helxo` / `strkey` entry, i.e. nothing was stubbed or left
      untranslated. Enforced by `tests/symbols.rs` and re-checked per
      configuration by `run_matrix.sh`.
- [x] Phase B: every one of the **78 `CONFIGS.md` rows** passes across
      randomized inputs (fixed seeds, `Rng` = splitmix64) — 36 tests.
- [x] Binary executable: the project builds **none** (C: `add_library(... SHARED)`
      only; Rust: `crate-type = ["cdylib"]`). The only stdout-producing function,
      `helxo`, is compared byte-for-byte for **all 256** `char` values by
      redirecting fd 1 into a scratch file around each call
      (`tests/lowlevel.rs::rows75_76_helxo_stdout_all_bytes`,
      `tests/errors.rs::err_row63_helxo_every_char`).
- [x] Phase C: every one of the **64 `ERRORS.md` rows** has a passing
      differential test asserting the *same* sentinel / error value — 19 tests, two of
      them out-of-process (`tests/aborts.rs`) because the C terminates the
      process. All seven C `assert`s are translated, not omitted.
- [x] All of the above hold under **every** feature combination. `Cargo.toml`
      declares no `[features]` table, so the only configuration is the empty /
      default one; `run_matrix.sh` enumerates the combinations mechanically from
      `Cargo.toml` and additionally runs the entire suite against **both** the
      debug and the release cdylib (the release profile differs materially:
      `opt-level = 3` and `panic = "abort"`).

### Reproducing

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && ./run_matrix.sh
```

Result: `ALL CONFIGURATIONS PASSED` — 58 tests × 2 profiles × 2 heap modes
(plain and `MALLOC_CHECK_=3 MALLOC_PERTURB_=42`), 0 failures, 0 compiler
warnings.

The hardened pass matters because both `.so`s allocate from the *same* process
heap through `realloc`/`free`: `MALLOC_CHECK_=3` aborts on any double free or
heap overflow and `MALLOC_PERTURB_` poisons freed memory, so a divergence in
either library's allocate/free pattern (a missing `free`, a wrong pointer handed
to `free`, a use-after-free) is caught rather than silently tolerated.

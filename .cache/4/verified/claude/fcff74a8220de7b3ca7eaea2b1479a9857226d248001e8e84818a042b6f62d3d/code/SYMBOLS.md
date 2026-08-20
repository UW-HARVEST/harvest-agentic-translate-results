# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on the C shared library
`c_src/build/libtranslated_rust.so` (built with
`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`).

Rust shared library: `target/debug/libhelxo_lib.so` (`crate-type = ["cdylib"]`,
`[lib] name = "helxo_lib"`).

The C library is a single translation unit (`c_src/src/lib.c`) — an inlined,
lightly modified copy of Sean Barrett's `stb_ds.h` (the public header
`c_src/include/lib.h` only declares `void helxo(char num);`).  Everything the
C `.so` exports is listed below.

## Exported (dynamic, defined) symbols

| # | symbol | C definition site (`c_src/src/lib.c`) | in Rust `.so`? | Rust definition site |
|---|--------|---------------------------------------|----------------|----------------------|
| 1 | `stbds_arrgrowf`      | L276 | yes | `src/array.rs`   `stbds_arrgrowf` |
| 2 | `stbds_arrfreef`      | L312 | yes | `src/array.rs`   `stbds_arrfreef` |
| 3 | `stbds_rand_seed`     | L355 | yes | `src/hash.rs`    `stbds_rand_seed` |
| 4 | `stbds_hash_string`   | L477 | yes | `src/hash.rs`    `stbds_hash_string` |
| 5 | `stbds_hash_bytes`    | L553 | yes | `src/hash.rs`    `stbds_hash_bytes` |
| 6 | `stbds_hmfree_func`   | L571 | yes | `src/hashmap.rs` `stbds_hmfree_func` |
| 7 | `stbds_hmget_key_ts`  | L631 | yes | `src/hashmap.rs` `stbds_hmget_key_ts` |
| 8 | `stbds_hmget_key`     | L659 | yes | `src/hashmap.rs` `stbds_hmget_key` |
| 9 | `stbds_hmput_default` | L667 | yes | `src/hashmap.rs` `stbds_hmput_default` |
| 10 | `stbds_hmput_key`    | L680 | yes | `src/hashmap.rs` `stbds_hmput_key` |
| 11 | `stbds_shmode_func`  | L796 | yes | `src/hashmap.rs` `stbds_shmode_func` |
| 12 | `stbds_hmdel_key`    | L807 | yes | `src/hashmap.rs` `stbds_hmdel_key` |
| 13 | `stbds_stralloc`     | L881 | yes | `src/arena.rs`   `stbds_stralloc` |
| 14 | `stbds_strreset`     | L920 | yes | `src/arena.rs`   `stbds_strreset` |
| 15 | `strkey`             | L939 | yes | `src/demo.rs`    `strkey` |
| 16 | `helxo`              | L945 | yes | `src/demo.rs`    `helxo` |

`diff` of the two sorted symbol lists is **EMPTY** (verified, see
`scripts/check_symbols.sh`): 16 symbols in C, 16 in Rust, exact name match.

## C symbols that are *declared* but not defined (therefore not exported)

These appear in the `extern` prototype block of `lib.c` but have no definition,
so they are absent from the C `.so` and must also be absent from the Rust `.so`:

* `stbds_unit_tests` — declared L83, never defined (no `STB_DS_UNIT_TESTS`
  body was inlined). Not exported by C → not exported by Rust. Correct.

## `static` (internal, non-exported) C functions

Translated as private Rust functions; deliberately **not** `#[no_mangle]`
because the C `.so` does not export them either:

| C static | Rust |
|----------|------|
| `stbds_probe_position` (L367) | `hash::stbds_probe_position` |
| `stbds_log2` (L375)           | `hash::stbds_log2` |
| `stbds_make_hash_index` (L385)| `hash::stbds_make_hash_index` |
| `stbds_siphash_bytes` (L498)  | `hash::stbds_siphash_bytes` |
| `stbds_is_key_equal` (L558)   | `hashmap::stbds_is_key_equal` |
| `stbds_hm_find_slot` (L586)   | `hashmap::stbds_hm_find_slot` |
| `stbds_strdup` (L870)         | `arena::stbds_strdup` |
| `static char buffer[256]` (L938) | `demo::BUFFER` |
| `static size_t stbds_hash_seed` (L353) | `hash::STBDS_HASH_SEED` |

## Undefined (imported) symbols

C imports: `__assert_fail`, `free`, `malloc`, `memcmp`, `memcpy`, `memmove`,
`memset`, `printf`, `realloc`, `sprintf`, `strcmp`, `strlen` (all libc).

Rust imports the same libc entry points (`realloc`, `free`, `memcmp`,
`memmove`, `memcpy`, `strcmp`, `strlen`, `printf`, `sprintf`) plus the Rust
runtime's own libc/unwind imports (`abort`, `calloc`, `mmap64`, `_Unwind_*`,
…).  **0 missing / unresolved non-libc symbols.**

> NOTE: the C library is compiled with the default (empty) `CMAKE_BUILD_TYPE`,
> so `NDEBUG` is *not* defined and `STBDS_ASSERT` == `assert` is **live**
> (hence the `__assert_fail` import).  See `ERRORS.md` rows A1..A7.

## Feature combinations

`Cargo.toml` has **no `[features]` table** and no `cfg(feature = …)` anywhere in
`src/`, and `c_src/CMakeLists.txt` defines no build options / `#ifdef` knobs
(single `add_library(SHARED src/lib.c)`).  Therefore there is exactly **one**
valid build configuration:

| # | combo | command |
|---|-------|---------|
| 1 | *(no features — default == empty)* | `cargo check --no-default-features` / `cargo test --no-default-features` |

Both `--no-default-features` and the plain default build are the same
configuration and are verified.

## How this was verified

```sh
./scripts/verify.sh          # builds both .so files, checks symbol parity,
                             # runs the whole differential suite per feature combo
./scripts/check_symbols.sh   # just the nm -D parity check
```

Result:

```
C   defines 16 dynamic symbols
RUST defines 16 dynamic symbols
symbol diff EMPTY -- full parity
no unresolved non-libc imports
```

### Gotcha worth knowing

`cargo test --test <name>` does **not** rebuild the `cdylib`: an integration
test has no dependency edge to a `crate-type = ["cdylib"]` target, so the `.so`
that `libloading` opens can silently be an *older* build of `src/`.  This was
observed while mutation-testing the suite (mutations "survived" because the
tested `.so` predated them).  `tests/common/mod.rs::assert_fresh` now compares
the `.so` mtime against every file under `src/` plus `Cargo.toml` and fails the
run with a `STALE cdylib` message instead of verifying a stale binary; always
go through `cargo build && cargo test` (i.e. `./scripts/verify.sh`).

### Suite sensitivity (mutation testing)

To prove the differential tests actually have teeth, 25 mutations were injected
into `src/` one at a time (rebuilding in between) and the suite was re-run:

* 22 behaviour-changing mutations — **all caught** (siphash tail
  sign-extension, `hash_string` rotate constant, `load_32_or_64` constant, seed
  advancement, siphash round count, rehash probe position, stored-index
  off-by-one, `final_index` off-by-one, tombstone marker value, tombstone
  decrement, growth `>=`→`>`, shrink `<`→`<=`, rebuild `>`→`>=`,
  `tombstone_count_threshold` shift, shrink-threshold clamp, `mode >=`→`>`,
  `hmdel` strdup guard `==`→`>=`, symmetric `temp_key` in the wrapped half,
  `hmget_key_ts` NULL-map `temp`, missing `hmput_default` memset, arena
  `remaining` accounting, arena block-saturation boundary, non-masked arena
  shift, `arrgrowf` early-out boundary, `strkey` format string, `helxo` local
  key).
* 3 were *equivalent* mutants that cannot change behaviour and correctly
  survived (`min_cap < 4` → `<= 4` / `< 5` still assign 4; `printf("%i")` ==
  `printf("%d")` for `int`), plus 2 deliberate no-op controls.

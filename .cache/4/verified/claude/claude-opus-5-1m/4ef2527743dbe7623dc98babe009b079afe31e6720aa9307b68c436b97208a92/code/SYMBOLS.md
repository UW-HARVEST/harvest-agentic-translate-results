# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort > /tmp/c.txt
# Rust
cargo build --release
nm -D --defined-only target/release/libagglom_lib.so  | awk '{print $3}' | sort > /tmp/r.txt
diff /tmp/c.txt /tmp/r.txt        # MUST be empty
```

## Build configurations

`Cargo.toml` has **no `[features]` section**, so the complete set of valid
feature combinations is exactly one: the empty set.
`--no-default-features` and the default build are therefore identical, and
`c_src/CMakeLists.txt` has no `option()` / `#ifdef`-selected variants either
(single `add_library(... SHARED src/lib.c)`, `target_link_libraries(... m)`).

| # | feature combo | cargo invocation | status |
|---|---------------|------------------|--------|
| 1 | `<none>` (default == no-default-features) | `cargo test --no-default-features` | PASS (dev + release) |

`./run_all_configs.sh` derives this list mechanically from `Cargo.toml` (so it
keeps working if features are ever added) and runs `cargo check` plus the full
differential suite for every combination in **both** the `dev` and the `release`
profile.

### Build-system notes discovered during verification

* `[lib] crate-type` is `["cdylib", "rlib"]`. The `rlib` is **not** linked by the
  tests (they only `dlopen` the `.so`); it exists because with a `cdylib`-only
  library `cargo test` does **not** treat the lib as a dependency of the
  integration tests and therefore silently runs them against a **stale** `.so`.
  With the `rlib` present the cdylib is always rebuilt.
* `cargo build` uplifts the cdylib to `target/<profile>/`, whereas `cargo test`
  leaves it in `target/<profile>/deps/`. `common::rust_so_path()` searches both
  and picks the newest, and additionally asserts the `.so` is not older than
  `src/lib.rs` / `src/tables.rs`, so a stale-artifact false pass is impossible.
* `[profile.dev]`/`[profile.test]` set `debug-assertions = false`. Rust's
  debug-time UB checks otherwise turn the C's own null-pointer dereference
  (`f4(NULL)`) into a `SIGABRT` panic instead of the `SIGSEGV` the C produces;
  with them off, dev and release behave identically to the C (verified by
  `errors.rs::e32_null_pointer_parity`).

## Symbol table

`GLOBAL DEFAULT TEXT` (`T`) symbols exported by the C `.so`, and their status
in the Rust `.so`:

| # | symbol | C `.so` | Rust `.so` | Rust item |
|---|--------|---------|------------|-----------|
|  1 | `c2V`                | T | T | `#[unsafe(no_mangle)] pub extern "C" fn c2V` |
|  2 | `c2Maxv`             | T | T | `#[unsafe(no_mangle)] pub extern "C" fn c2Maxv` |
|  3 | `c2Minv`             | T | T | `#[unsafe(no_mangle)] pub extern "C" fn c2Minv` |
|  4 | `c2Clampv`           | T | T | `#[unsafe(no_mangle)] pub extern "C" fn c2Clampv` |
|  5 | `c2Sub`              | T | T | `#[unsafe(no_mangle)] pub extern "C" fn c2Sub` |
|  6 | `c2Dot`              | T | T | `#[unsafe(no_mangle)] pub extern "C" fn c2Dot` |
|  7 | `c2CircletoCircle`   | T | T | `#[unsafe(no_mangle)] pub extern "C" fn c2CircletoCircle` |
|  8 | `c2CircletoAABB`     | T | T | `#[unsafe(no_mangle)] pub extern "C" fn c2CircletoAABB` |
|  9 | `c2AABBtoAABB`       | T | T | `#[unsafe(no_mangle)] pub extern "C" fn c2AABBtoAABB` |
| 10 | `f2`                 | T | T | `#[unsafe(no_mangle)] pub unsafe extern "C" fn f2` |
| 11 | `f3`                 | T | T | `#[unsafe(no_mangle)] pub extern "C" fn f3` |
| 12 | `f4`                 | T | T | `#[unsafe(no_mangle)] pub unsafe extern "C" fn f4` |
| 13 | `f5`                 | T | T | `#[unsafe(no_mangle)] pub extern "C" fn f5` |
| 14 | `f7`                 | T | T | `#[unsafe(no_mangle)] pub extern "C" fn f7` |
| 15 | `f9`                 | T | T | `#[unsafe(no_mangle)] pub extern "C" fn f9` |
| 16 | `f10`                | T | T | `#[unsafe(no_mangle)] pub extern "C" fn f10` |
| 17 | `f11`                | T | T | `#[unsafe(no_mangle)] pub unsafe extern "C" fn f11` |
| 18 | `f12`                | T | T | `#[unsafe(no_mangle)] pub unsafe extern "C" fn f12` |
| 19 | `f13`                | T | T | `#[unsafe(no_mangle)] pub unsafe extern "C" fn f13` |
| 20 | `agglom`             | T | T | `#[unsafe(no_mangle)] pub extern "C" fn agglom` |

**Missing from Rust: none.** `diff` of the two sorted `nm -D` name lists is empty
(20 symbols on each side). See `tests/symbol_parity.rs`, which recomputes the
diff at test time and fails if it is ever non-empty.

## C `static` (non-exported) entities — deliberately NOT exported

These are `static` in `c_src/src/lib.c` and therefore have **local** binding
(they do not appear in `nm -D`). They are translated into private Rust items,
so the Rust `.so` must not export them either:

| C entity | binding | Rust counterpart |
|----------|---------|------------------|
| `cn_rnd_next`            | local fn      | `fn cn_rnd_next` (private) |
| `lm_v2`                  | local fn      | `fn lm_v2` (private) |
| `lm_sub2`                | local fn      | `fn lm_sub2` (private) |
| `lm_dot2`                | local fn      | `fn lm_dot2` (private) |
| `tflac_crc16_tables`     | local data    | `tables::TFLAC_CRC16_TABLES` (private, unused in C too) |
| `m__mantissa`            | local data    | `tables::M_MANTISSA` |
| `m__offset`              | local data    | `tables::M_OFFSET` |
| `m__exponent`            | local data    | `tables::M_EXPONENT` |

All four static tables were verified element-by-element against the C source
(2048 + 2048 + 64 + 64 values, all identical — see
`tests/symbol_parity.rs::static_tables_match_c_source`).

## Undefined (imported) symbols

`nm -D --undefined-only`:

* C `.so`: `floorf@GLIBC_2.2.5`, `fmodf@GLIBC_2.2.5` plus the usual
  `__cxa_finalize` / `__gmon_start__` / `_ITM_*` CRT stubs.
* Rust `.so`: **no** `floorf`/`fmodf` import — `compiler-builtins` supplies its
  own implementations, statically linked. Everything else it imports is libc /
  libgcc-unwind runtime (`malloc`, `memcpy`, `_Unwind_*`, …).

Both `fmod` and `floor` are *exactly* specified by IEEE-754 (no rounding
freedom), so the two implementations must agree bit-for-bit on finite inputs;
`NaN` payload handling is the only place they could differ. Rows **C71–C73** of
`CONFIGS.md` therefore sweep the entire `f32` exponent range (sign × exponent
0..255 × 61 mantissa patterns ≈ 65 000 inputs each) through `f11` (which calls
`fmodf`) and `f12` (which calls `floorf`); all agree.

`floorf`'s result in `f12` is additionally only ever consumed by
`(int)` conversion (`cvttss2si`), so even a payload difference there would be
unobservable.

No non-libc undefined symbols on either side.

# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  : `c_src/build/libharvest-work-XCLAh1.so` (built via `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust: `translation/target/release/libhex2bin_lib.so` (`crate-type = ["cdylib"]`)

## C source inventory (completeness check)

The whole library is two files, both accounted for:

| C file | translated in |
|--------|---------------|
| `c_src/include/lib.h` (1 declaration) | `translation/src/lib.rs` (signature) |
| `c_src/src/lib.c` (1 definition)      | `translation/src/lib.rs` (`hex2bin`) |

No C module was skipped; there is nothing left to translate.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `hex2bin` | `T` (text, global) | `T` (text, global) | present in both |

### Raw output

```
$ nm -D --defined-only c_src/build/libharvest-work-XCLAh1.so
0000000000001109 T hex2bin

$ nm -D --defined-only translation/target/release/libhex2bin_lib.so
00000000000116c0 T hex2bin
```

`nm -D --defined-only` on the Rust `.so` reports no additional non-libc
exports beyond `hex2bin` (Rust's own `cdylib` housekeeping symbols such as
`_init`/`_fini`/`__bss_start` are linker-generated and not part of the C API).

## Undefined (imported) symbols

```
$ nm -D --undefined-only c_src/build/libharvest-work-XCLAh1.so
strchr        (libc)

$ nm -D --undefined-only translation/target/release/libhex2bin_lib.so
(only libc/ld startup symbols; `strchr` is reimplemented in Rust as
 `c_strchr_found`, which is intentionally *not* exported)
```

## Result

* Symbols exported by C but missing from Rust: **0**
* Undefined non-libc symbols in the Rust `.so`: **0**
* No stubs / `unimplemented!()` were introduced.

**Phase A / Phase D symbol gate: PASS (diff is empty).**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
buildable configuration is the default one (`--no-default-features` and the
default build are the same code). The feature-combination sweep is
consequently a single cell; it is still executed explicitly by
`scripts/check_features.sh`.

## Reproducing

```
cd translation && ./scripts/run_all.sh
```

That script builds the C `.so`, enumerates the feature combinations from
`Cargo.toml`, builds the Rust cdylib for each (release **and** debug), runs the
full differential suite against each, then re-checks symbol parity
(`scripts/symbol_diff.sh`) and suite adequacy (`scripts/mutation_check.sh`).

Note: plain `cargo test` does **not** build a `crate-type = ["cdylib"]`
artifact. The harness therefore refuses to run against a `.so` older than
`src/lib.rs` (verified: touching `src/lib.rs` makes every test fail with
`STALE ARTIFACT`), so a stale library can never produce a silent pass.

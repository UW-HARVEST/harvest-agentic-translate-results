# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D` on both shared objects.

* C: `c_src/build/libharvest-work-rMwUs1.so` (GCC 11.5.0, CMake default flags = `-O0`)
* Rust: `translation/target/release/libunderhanded_c_nuke_lib.so` (`crate-type = ["cdylib"]`)

Reproduce with `translation/check_symbols.sh`.

## C `.so` — defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | type | present in Rust `.so`? | Rust definition |
|---|--------|------|------------------------|-----------------|
| 1 | `match` | `T` | yes | `src/match.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn r#match` |
| 2 | `spectral_contrast` | `T` | yes | `src/spectral_contrast.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn spectral_contrast` |

There are no macro-generated symbols: `include/match.h` contains exactly one
object-like macro (`N_SMOOTH`), one `typedef` and the two prototypes above.
Everything else in `src/match.c` (`total`, `smoothen`, `differentiate`,
`preprocess`) and `src/spectral_contrast.c` (`dot_product`, `normalize`) is
`static`, hence local (`t`, not `T`) and not part of the ABI:

```
$ nm libharvest-work-rMwUs1.so | grep ' t '
0000000000001227 t differentiate
00000000000014c0 t dot_product
000000000000153a t normalize
00000000000012b9 t preprocess
000000000000117f t smoothen
0000000000001129 t total
```

Those six statics are all translated (private `fn`s in `src/match.rs` and
`src/spectral_contrast.rs`), so no C source is missing. No stubs and no
`unimplemented!()` anywhere in the crate.

**Symbol diff (C-exported minus Rust-exported): EMPTY.**

## Undefined (imported) symbols

| `.so` | undefined symbols |
|-------|-------------------|
| C | `memcpy@GLIBC_2.14`, `sqrt@GLIBC_2.2.5`, plus the weak toolchain set (`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`) |
| Rust | libc/`libgcc_s` only (`memcpy`, `memset`, `__libc_start_main` family, unwinder/TLS helpers, `_ITM_*`, `__cxa_finalize`, `__gmon_start__`) |

0 missing / undefined **non-libc** symbols in the Rust `.so`.

`sqrt` is not imported by the Rust `.so` because `f64::sqrt` lowers to the
`sqrtsd` instruction. That is the same operation glibc's `sqrt` performs on
x86-64 (`normalize` in the C `.so` calls `sqrt@plt`, which resolves to the
`sqrtsd`-based implementation), so results are bit-identical, including NaN
quieting and the negative-operand case.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
build configuration is the default one. `check_features.sh` enumerates the
feature list from `Cargo.toml` and confirms it is empty, so
`cargo test --no-default-features` (the sole non-default combination that
exists) is equivalent to the default build; both are run by that script.

## How this was verified (reproduce)

```bash
# 1. C reference
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Rust cdylib + symbol parity
cd translation && cargo build --release && ./check_symbols.sh

# 3. Phase B + C differential suite (all rows), every feature combo, both profiles
./check_features.sh

# 4. Heavier randomized pass
DIFF_ITERS=60000 cargo test --release --test configs --test errors

# 5. Prove the suite has teeth
./mutation_check.sh
```

`tests/harness.rs` additionally asserts the two loaded `.so`s are distinct files
and that `match` / `spectral_contrast` resolve to different addresses in each,
so the suite cannot silently compare one implementation against itself.

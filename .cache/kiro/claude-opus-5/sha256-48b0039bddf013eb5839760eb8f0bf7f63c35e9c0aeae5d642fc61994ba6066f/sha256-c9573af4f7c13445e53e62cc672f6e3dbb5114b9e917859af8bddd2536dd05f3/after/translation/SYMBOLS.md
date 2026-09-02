# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

## Build commands

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-lz3nl1.so

cd translation && cargo build --release
# -> translation/target/release/libsynth_pair_lib.so
```

## C `.so` exported symbols (`nm -D --defined-only`)

```
0000000000001160 T synth_pair
```

## Rust `.so` exported symbols (`nm -D --defined-only`)

```
00000000000116e0 T synth_pair
```

## Parity table

| # | C symbol | type | present in Rust `.so`? | notes |
|---|----------|------|------------------------|-------|
| 1 | `synth_pair` | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn synth_pair` in `src/lib.rs` |

## Symbols intentionally NOT exported

| C entity | linkage in C | why absent from `nm -D` (both sides) |
|----------|--------------|--------------------------------------|
| `mp3d_scale_pcm` | `static int16_t` — internal linkage | Not a public symbol in the C `.so`; Rust mirrors it as a private `fn`. Exporting it would create a symbol the C library does not have. |
| `mp3d_sample_t` | `typedef int16_t` | Type alias, emits no symbol. Mirrored as `pub type Mp3dSampleT = i16`. |

## Diff result

```
comm -13 <(c symbols) <(rust symbols)   # symbols only in C -> (empty)
```

**MISSING SYMBOLS: 0.** No C source file was left untranslated: `c_src/src/lib.c`
is the only translation unit in `CMakeLists.txt`, and both of its functions
(`synth_pair`, `mp3d_scale_pcm`) are present in `translation/src/lib.rs`.
No stubs / `unimplemented!()` were introduced.

## Undefined (imported) symbols

Every undefined symbol in the Rust `.so` resolves to glibc (all carry an
`@GLIBC_*` version tag) — there are **no unresolved project-level symbols**.
`run_all.sh` asserts this after filtering `@GLIBC` and the platform runtime
(`_ITM_*`, `__cxa_*`, `__gmon_*`, `__tls_get_addr`, `_Unwind_*`):

```
nm -D --undefined-only target/<profile>/libsynth_pair_lib.so
# -> bcmp, close, dl_iterate_phdr, getcwd, getenv, mmap64, ... (all @GLIBC_*)
```

Those come from the Rust standard library (panic machinery, TLS, allocator), not
from the translated code.

## Symbol parity is also asserted from inside the test suite

`tests/differential.rs::harness_symbol_parity` shells out to `nm -D` on both
`.so`s and fails if any C symbol is absent from the Rust `.so`, so the parity
check runs on every `cargo test`, not only in `run_all.sh`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
build configuration is the default one. `--no-default-features` and the
default build are therefore identical; both are exercised by the test script
`run_all.sh`.

# SYMBOLS.md — public symbol surface

Source of truth: `nm -D --defined-only` (text symbols, `T`) on
`c_src/build/libharvest-work-NMS4T0.so` vs `translation/target/release/libagglom_lib.so`.

The C translation unit is a single file (`c_src/src/lib.c`, 1104 lines) with no
namespace-renaming macros, so linker names equal source-level names. Everything
declared `static` in the C (`cn_rnd_next`, `lm_v2`, `lm_sub2`, `lm_dot2`, and the
lookup tables `tflac_crc16_tables`, `m__mantissa`, `m__offset`, `m__exponent`)
is intentionally NOT exported and must not appear in either `.so`.

## Parity table (20 C symbols / 20 Rust symbols)

| # | symbol | C signature | in C `.so` | in Rust `.so` | note |
|---|--------|-------------|-----------|--------------|------|
| 1 | `c2V` | `c2v c2V(float, float)` | T | T | 8-byte struct return in `xmm0` |
| 2 | `c2Maxv` | `c2v c2Maxv(c2v, c2v)` | T | T | |
| 3 | `c2Minv` | `c2v c2Minv(c2v, c2v)` | T | T | |
| 4 | `c2Clampv` | `c2v c2Clampv(c2v, c2v, c2v)` | T | T | |
| 5 | `c2Sub` | `c2v c2Sub(c2v, c2v)` | T | T | |
| 6 | `c2Dot` | `float c2Dot(c2v, c2v)` | T | T | |
| 7 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle, c2Circle)` | T | T | 12-byte struct args |
| 8 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle, c2AABB)` | T | T | 12- + 16-byte struct args |
| 9 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB, c2AABB)` | T | T | |
| 10 | `f2` | `int f2(const void*, C2_TYPE, const void*, C2_TYPE)` | T | T | enum passed as 32-bit int |
| 11 | `f3` | `int f3(int, int)` | T | T | |
| 12 | `f4` | `double f4(cn_rnd_t*)` | T | T | mutates caller state in place |
| 13 | `f5` | `uint32_t f5(uint32_t)` | T | T | |
| 14 | `f7` | `tflac_u32 f7(tflac_u32, tflac_u32, tflac_u32)` | T | T | |
| 15 | `f9` | `lm_vec2 f9(lm_vec2, lm_vec2, lm_vec2, lm_vec2)` | T | T | 8-byte struct return |
| 16 | `f10` | `float f10(uint16_t)` | T | T | |
| 17 | `f11` | `void f11(float*, const float*)` | T | T | writes 3 floats |
| 18 | `f12` | `void f12(float*, const float*)` | T | T | writes 3 floats |
| 19 | `f13` | `void f13(float*, const float*)` | T | T | writes 3 floats |
| 20 | `agglom` | `double agglom(33 scalar args)` | T | T | the only symbol in `include/lib.h` |

## Symbol diff

Reproduce with `./verify.sh` (which rebuilds both libraries first):

```
$ nm -D --defined-only c_src/build/libharvest-work-NMS4T0.so | awk '$2=="T"{print $3}' | sort > c.txt
$ nm -D --defined-only translation/target/so-under-test/release/libagglom_lib.so \
      | awk '$2=="T"{print $3}' | sort > r.txt
$ wc -l c.txt r.txt
20 c.txt
20 r.txt
$ comm -23 c.txt r.txt   # in C, missing from Rust
(empty)
$ comm -13 c.txt r.txt   # extra in Rust
(empty)
```

**Status: 0 missing, 0 extra.** No module of the C source was skipped; the Rust
crate translates all of `lib.c`. No symbol is stubbed or `unimplemented!()` —
every one is exercised by a differential test in Phase B and/or Phase C.

## Undefined (imported) symbols

The Rust `.so` imports only libc / libgcc-unwind symbols
(`memcpy`, `malloc`, `_Unwind_*`, `__cxa_finalize`, ...). It imports **0**
non-libc symbols. The C `.so` additionally imports `floorf` and `fmodf` from
glibc; the Rust build satisfies both statically (LLVM lowers `floorf` to
`roundss`, and `fmodf` comes from `compiler_builtins`) — both are exactly
rounded operations, so this is behaviour-preserving and is covered by the
Phase B `f11`/`f12` rows.

**Status: 0 missing/undefined non-libc symbols in Rust.**

## Cargo features

`translation/Cargo.toml` declares no `[features]` table and no optional
dependencies, so the only feature combination that exists is the default (empty)
one. `--no-default-features` and `--all-features` are therefore identical to the
default build; Phase D still runs all three to prove it.

## Testing note: `cargo test` does not rebuild the `cdylib`

Because the crate is `crate-type = ["cdylib"]`, `cargo test` compiles the library
as an rlib for the test binaries and does **not** regenerate
`target/<profile>/libagglom_lib.so`. Loading that path from a test silently
verifies a **stale** `.so`. This was confirmed empirically: a deliberately
injected bug in `f5` (`0x8000 -> 0x8001`) went completely undetected by the whole
suite.

`tests/harness/mod.rs` therefore shells out to
`cargo build --lib --target-dir target/so-under-test` before `dlopen`ing the
result, so the `.so` under test always matches the current source. Re-running the
same injected `f5` bug afterwards fails
`c20_f5_bit_reversal`, `c48_agglom_full_random_bit_space` and
`c49_agglom_per_subfunction_sweeps`, which is the evidence that the harness has
teeth.

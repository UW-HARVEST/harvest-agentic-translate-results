# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust
cargo build            # -> target/debug/libcontrast_ratio_lib.so
```

## C `.so` — defined dynamic symbols

`nm -D --defined-only c_src/build/libtranslated_rust.so`

| # | type | symbol | present in Rust `.so`? | notes |
|---|------|--------|------------------------|-------|
| 1 | `T`  | `contrast_ratio` | YES | `#[no_mangle] pub extern "C" fn contrast_ratio` in `src/lib.rs` |

`cbLuminance` and `cbContrastRatio` are `static` in `c_src/src/lib.c`, therefore
**not** part of the dynamic surface (they are local `t` symbols only, and only in
the unstripped object). They are correctly translated as private Rust `fn`s
(`cb_linearize` / `cb_luminance` / `cb_contrast_ratio`) and MUST NOT be exported —
exporting them would be a symbol-surface *mismatch* in the other direction.

Confirmation that the statics are not dynamic symbols:

```
$ nm -D --defined-only c_src/build/libtranslated_rust.so | grep -cE 'cbLuminance|cbContrastRatio'
0
```

## Rust `.so` — defined dynamic symbols

`nm -D --defined-only target/debug/libcontrast_ratio_lib.so`

| # | type | symbol |
|---|------|--------|
| 1 | `T`  | `contrast_ratio` |

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so    | awk '{print $2, $3}' | sort) \
       <(nm -D --defined-only target/debug/libcontrast_ratio_lib.so | awk '{print $2, $3}' | sort)
# (no output)
```

**Result: EMPTY DIFF.** 0 missing symbols, 0 extra symbols, identical binding
type (`T`). No C source file was left untranslated: `c_src/src/lib.c` is the only
translation unit in `c_src/CMakeLists.txt`, and every non-`static` function in it
is exported by the Rust `.so`.

## Undefined (imported) symbols

C:

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
U pow@GLIBC_2.29
```

Rust: also imports `U pow@GLIBC_2.29` (from `f64::powf`), i.e. **the exact same
libm entry point** as the C `pow(...)` call. This is required for bit-identical
results: any Rust-side reimplementation of `pow` would diverge from glibc's.

```
$ nm -D --undefined-only target/debug/libcontrast_ratio_lib.so | grep -i pow
                 U pow@GLIBC_2.29
```

**0 missing / undefined non-libc symbols in the Rust `.so`.**

## Codegen notes checked while mapping the surface

* `objdump -d c_src/build/libtranslated_rust.so | grep -cE 'vfmadd|fmadd'` → `0`.
  GCC's default `-ffp-contract=fast` did **not** contract
  `0.2126f*R + 0.7152f*G + 0.0722f*B` into an FMA (baseline `x86-64` has no FMA),
  so the Rust translation must likewise *not* use `mul_add`. It does not.
* All `float` arithmetic in the C is emitted as SSE scalar `divss`/`mulss`/`addss`
  (no x87 excess precision), so plain `f32` ops in Rust are exact matches.
* The `> 0.04045`, `+ 0.055`, `/ 1.055`, `pow(_, 2.4)` and `/ 12.92` steps are all
  `double` in C (usual arithmetic conversions promote the `float` operand against
  the `double` literals); only the ternary result is cast back to `float`. The Rust
  `cb_linearize` reproduces exactly this f32 -> f64 -> f32 sequence.

## ABI note (struct-by-value)

`cb_rgb_255` is a 3-byte, align-1 POD struct. Under the x86-64 SysV ABI it is
classified INTEGER and passed in the low 24 bits of a single general-purpose
register (`rdi` for `A`, `rsi` for `B`) — confirmed in the C disassembly of
`contrast_ratio`, which spills `%rdi`/`%rsi` and then reads bytes
`-0x8/-0x7/-0x6` (A.R/A.G/A.B) and `-0x10/-0xf/-0xe` (B.R/B.G/B.B). The upper 40
bits of each register are padding and must be ignored. Rust's `#[repr(C)]`
struct uses the same classification; this is verified by an explicit
differential test that passes garbage in the padding bits
(`ERRORS.md` rows E5/E6).

## Phase D results

Symbol parity is enforced as a test (`tests/phase_d_symbols.rs`) so it cannot
silently regress:

| test | what it asserts | result |
|------|-----------------|--------|
| `d1_every_c_symbol_is_exported_by_rust` | `nm -D --defined-only` set difference (C minus Rust) is EMPTY; `contrast_ratio` is a global text (`T`) symbol in both; the `static` helpers `cbLuminance`/`cbContrastRatio` are absent from both dynamic surfaces | [x] pass |
| `d2_rust_so_has_no_unresolved_non_libc_symbols` | every undefined symbol in the Rust `.so` is platform-provided (glibc/libgcc symbol-versioned or a compiler builtin); and the Rust `.so` imports libm `pow`, the same entry point the C uses | [x] pass |
| `d3_all_c_symbols_resolvable_from_rust_via_dlsym` | every symbol `nm -D` reports for the C `.so` resolves via `dlsym` on the Rust `.so` — the check that actually matters to a consumer swapping one library for the other | [x] pass |

**Symbol diff: EMPTY. 0 missing, 0 extra.**

## Harness note: `cargo test` does NOT rebuild a `cdylib`

Worth recording because it silently invalidated the first version of this suite.
With `crate-type = ["cdylib"]`, `cargo test` builds the test harnesses and the
lib's unit-test binary but leaves `target/debug/libcontrast_ratio_lib.so`
untouched. Verified directly: mutating `src/lib.rs` and re-running `cargo test`
left the `.so` mtime unchanged and **all tests still passed against the stale
binary**.

`tests/common/mod.rs::build_rust_so` therefore shells out to
`cargo build --lib` (into a dedicated `CARGO_TARGET_DIR`, so it never contends
for the outer cargo build lock), honours `DIFF_FEATURES` /
`DIFF_NO_DEFAULT_FEATURES` and the current profile, and asserts the resulting
`.so` is not older than any file in `src/`. A negative control confirms the
harness now detects divergence: with the luminance coefficients swapped, 16 of
18 Phase B tests fail.

## Mutation-sensitivity audit (`./mutation_audit.sh`)

Passing tests only mean something if they can fail. 31 plausible C-to-Rust
translation bugs were injected into `src/lib.rs` one at a time; the suite must
catch every semantically-observable one.

**Result: 28 of 31 CAUGHT. The 3 survivors are provably semantics-preserving:**

| survivor | why it cannot be detected by any input |
|----------|----------------------------------------|
| threshold `0.04045` -> `0.0405` | no `u8` `n` has `n/255.f` in `(0.04045, 0.0405]` — verified over all 256 values (`10/255.f = 0.039215688`, `11/255.f = 0.043137256`) |
| channel conversion routed through `f64` | verified for all 256 `n` that `f32(n/255 computed in f64) == f32(n)/f32(255)`; double rounding never bites in this range |
| `#[repr(C)]` removed from `cb_rgb_255` | rustc's default layout only reorders fields to reduce padding; three fields of identical type/alignment leave nothing to reorder, so the layout and SysV argument classification coincide. The shipped code keeps `repr(C)`, which makes that guaranteed rather than incidental. |

Caught mutations include: all 3 luminance coefficients swapped/typo'd; the dot
product computed in `f64`, right-associated, or contracted into an FMA; the
linearization done entirely in `f32`; `f64` leaking past the per-channel
`(float)` truncation; the `0.04045` threshold, the `2.4` exponent, the `0.055`
offset and the `1.055`/`12.92` divisors perturbed; the two linearization branches
inverted; `powf` replaced by an `exp(2.4*ln(x))` reimplementation; the
`High < Low` swap removed / forced / inverted; the ratio inverted; a
division-by-zero guard or an epsilon clamp *added* (i.e. "fixing" the C); `/256`
instead of `/255`; struct fields reordered or widened to `u16`; channels
transposed at the call site; `extern "C"` changed; and `#[no_mangle]` removed.

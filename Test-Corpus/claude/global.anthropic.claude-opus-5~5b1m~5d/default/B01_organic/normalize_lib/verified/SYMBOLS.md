# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/lib<parent-dir-name>.so   (CMakeLists derives the project name
#    from the *parent* directory of c_src, so the file name is environment
#    dependent; the tests glob `c_src/build/*.so`.)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libnormalize_lib.so
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C file | translated? | Rust location |
|--------|-------------|---------------|
| `c_src/src/lib.c` | yes | `translation/src/lib.rs` |

`c_src/include/lib.h` declares exactly one prototype:

```c
void normalize(float *dest, const float *src, int size);
```

There is no second module, no macro-generated symbol family, no `#ifdef`-gated
alternative implementation, and no static/internal helper. The whole library is
one function, so no C source was skipped by the translation step.

## `nm -D --defined-only` — C `.so`

```
0000000000001119 T normalize
```

Exported (defined, dynamic) symbols: **1**

## `nm -D --defined-only` — Rust `.so`

```
0000000000011c30 T normalize
```

Exported (defined, dynamic) symbols: **1**

## Symbol diff

| symbol | C `.so` | Rust `.so` | status |
|--------|---------|------------|--------|
| `normalize` | `T` | `T` | present in both — OK |

**Missing from Rust `.so`: none. Extra in Rust `.so`: none. Diff is EMPTY.**

## Undefined (imported) symbols

C `.so` imports: `memset@GLIBC_2.2.5`, `sqrtf@GLIBC_2.2.5`, plus the standard
weak CRT symbols (`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`).

Rust `.so` imports only libc (`memset`, `memcpy`, `malloc`, `free`, `open64`,
`read`, `write`, …), the `_Unwind_*` family from `libgcc_s`, and the same weak
CRT symbols. These are the Rust standard library / panic-machinery imports that
every `cdylib` carries.

**Non-libc / non-runtime undefined symbols in the Rust `.so`: 0.**

Notably the Rust `.so` does *not* import `sqrtf`: `f32::sqrt` lowers to the
`llvm.sqrt.f32` intrinsic, i.e. a single `sqrtss` instruction. Both that
instruction and glibc's `sqrtf` are IEEE-754 correctly-rounded for the
single-precision square root, so results are bit-identical for every input the
C code can reach (`sum > 0.0f`, hence never negative, never NaN).

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only
configurations that exist are the (empty) default feature set and
`--no-default-features`, which are identical. Both are still exercised
explicitly by `run_all.sh` per Phase D.

## Results

`tests/symbols.rs` re-derives both symbol sets by shelling out to `nm -D` at
test time and fails if the C set is not a subset of the Rust set, so the parity
claim above is enforced rather than asserted. Step 3 of `run_all.sh` repeats the
same `comm -23` diff for every feature combination x profile:

```
symbol parity OK  [default/dev]                  (1 C symbol, all present)
symbol parity OK  [default/release]              (1 C symbol, all present)
symbol parity OK  [--no-default-features/dev]    (1 C symbol, all present)
symbol parity OK  [--no-default-features/release](1 C symbol, all present)
```

Missing symbols: **0**. Unresolved non-libc / non-runtime symbols in the Rust
`.so`: **0** (checked by `grep -E '^_ZN|^_R[A-Za-z]'` over `nm -D -u`).

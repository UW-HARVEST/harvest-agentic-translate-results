# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D --defined-only` on the C shared library and on
the Rust `cdylib`.

## Build commands used

```sh
# C
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust (only one configuration exists — see CONFIGS.md)
cargo build --no-default-features            # -> target/debug/libcircle_collide_lib.so
cargo build --no-default-features --release  # -> target/release/libcircle_collide_lib.so
```

## C `.so` exported symbols (`nm -D --defined-only`, 12 total)

| # | symbol | C signature (`c_src/src/lib.c`) | in Rust `.so`? |
|---|--------|---------------------------------|----------------|
| 1 | `c2V`               | `c2v c2V(float x, float y)`                              | YES |
| 2 | `c2Mulvs`           | `c2v c2Mulvs(c2v a, float b)`                            | YES |
| 3 | `c2Maxv`            | `c2v c2Maxv(c2v a, c2v b)`                               | YES |
| 4 | `c2Minv`            | `c2v c2Minv(c2v a, c2v b)`                               | YES |
| 5 | `c2Clampv`          | `c2v c2Clampv(c2v a, c2v lo, c2v hi)`                    | YES |
| 6 | `c2Sub`             | `c2v c2Sub(c2v a, c2v b)`                                | YES |
| 7 | `c2Dot`             | `float c2Dot(c2v a, c2v b)`                              | YES |
| 8 | `c2CircletoCircle`  | `int c2CircletoCircle(c2Circle A, c2Circle B)`           | YES |
| 9 | `c2CircletoAABB`    | `int c2CircletoAABB(c2Circle A, c2AABB B)`               | YES |
| 10 | `c2CircletoCapsule`| `int c2CircletoCapsule(c2Circle A, c2Capsule B)`         | YES |
| 11 | `c2Collided`       | `int c2Collided(const void*A, const void*B, C2_TYPE tB)` | YES |
| 12 | `circle_collide`   | `int circle_collide(float x, float y, float r)`          | YES |

`c_src/include/lib.h` declares only `circle_collide`; the remaining 11 symbols
have external linkage in `src/lib.c` (no `static`), so they are part of the
`.so`'s public ABI and are tested as public entry points too.

## Diff

```
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/release/libcircle_collide_lib.so \
           | awk '$2=="T"{print $3}' | grep -v '^_' | sort)
(empty)
```

**0 symbols missing from the Rust `.so`.** No C source file was left
untranslated: `c_src/` contains exactly one translation unit (`src/lib.c`, 144
lines) plus `include/lib.h`, and every function in it is reproduced in
`src/lib.rs`.

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports (`malloc`, `memcpy`, `_Unwind_*`, …) — no unresolved non-libc symbols.

Automated check: `tests/symbol_parity.rs` re-runs this diff as a test.

## ABI notes (verified by the differential tests)

The struct-by-value shapes deliberately cover all three SysV AMD64 argument
classes, so the `#[repr(C)]` layouts and the `extern "C"` wrappers are all
exercised:

| type | size | SysV class | how it is passed |
|------|------|-----------|-------------------|
| `c2v`       | 8 B  | SSE           | one XMM register (packed 2×f32); returned in XMM0 |
| `c2Circle`  | 12 B | SSE, SSE      | two XMM registers (`p` packed, then `r`) |
| `c2AABB`    | 16 B | SSE, SSE      | two XMM registers |
| `c2Capsule` | 20 B | MEMORY        | passed on the stack |
| `C2_TYPE`   | 4 B  | INTEGER       | plain `int` — accepts any `int` value, not just 0/1/2 |

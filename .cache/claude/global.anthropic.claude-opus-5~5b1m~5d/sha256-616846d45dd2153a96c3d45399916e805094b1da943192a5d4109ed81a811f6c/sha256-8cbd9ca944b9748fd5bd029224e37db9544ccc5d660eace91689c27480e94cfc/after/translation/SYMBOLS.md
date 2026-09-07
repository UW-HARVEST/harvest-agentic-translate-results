# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C  `.so`: `c_src/build/libharvest-work-pvaRaj.so`
- Rust `.so`: `translation/target/release/libcircle_collide_lib.so`

Commands used:

```sh
nm -D --defined-only c_src/build/libharvest-work-pvaRaj.so | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libcircle_collide_lib.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # in C, missing from Rust  -> MUST be empty
```

## Symbol table

All symbols are `T` (global text) in both objects.

| # | symbol | C signature (from `c_src/src/lib.c`) | in C `.so` | in Rust `.so` | status |
|---|--------|--------------------------------------|-----------|--------------|--------|
| 1 | `c2V`               | `c2v c2V(float x, float y)`                              | T | T | OK |
| 2 | `c2Mulvs`           | `c2v c2Mulvs(c2v a, float b)`                            | T | T | OK |
| 3 | `c2Maxv`            | `c2v c2Maxv(c2v a, c2v b)`                               | T | T | OK |
| 4 | `c2Minv`            | `c2v c2Minv(c2v a, c2v b)`                               | T | T | OK |
| 5 | `c2Clampv`          | `c2v c2Clampv(c2v a, c2v lo, c2v hi)`                    | T | T | OK |
| 6 | `c2Sub`             | `c2v c2Sub(c2v a, c2v b)`                                | T | T | OK |
| 7 | `c2Dot`             | `float c2Dot(c2v a, c2v b)`                              | T | T | OK |
| 8 | `c2CircletoCircle`  | `int c2CircletoCircle(c2Circle A, c2Circle B)`           | T | T | OK |
| 9 | `c2CircletoAABB`    | `int c2CircletoAABB(c2Circle A, c2AABB B)`               | T | T | OK |
| 10 | `c2CircletoCapsule`| `int c2CircletoCapsule(c2Circle A, c2Capsule B)`         | T | T | OK |
| 11 | `c2Collided`       | `int c2Collided(const void *A, const void *B, C2_TYPE typeB)` | T | T | OK |
| 12 | `circle_collide`   | `int circle_collide(float x, float y, float r)`           | T | T | OK |

**Missing from Rust: 0.** `comm -23` output is empty. No module of C source was
skipped: `c_src` contains exactly one translation unit (`src/lib.c`, 144 lines)
plus `include/lib.h` (1 line), and every function defined in it is exported by
the Rust `.so` under the identical name.

## Undefined (imported) symbols

```sh
nm -D --undefined-only c_src/build/libharvest-work-pvaRaj.so
nm -D --undefined-only translation/target/release/libcircle_collide_lib.so
```

Neither object imports any non-libc symbol. The C object imports nothing at all
(no libc calls in the source). The Rust object imports only libc/ld.so glue that
`cdylib` linkage always pulls in. After filtering the standard libc set, the
residue reported by `verify.sh` is:

```text
__cxa_thread_atexit_impl@GLIBC_2.18
lseek64@GLIBC_2.2.5
realpath@GLIBC_2.3
```

All three are glibc symbols (versioned `@GLIBC_*`), pulled in by `std`'s
thread-local destructor and `std::fs`/`std::path` machinery — not program
symbols, and not required to be mirrored by the C object. There are **0
undefined non-libc symbols**.

## ABI notes relevant to the FFI tests

SysV AMD64 classification of the by-value struct types, which the Rust
`extern "C"` declarations must reproduce:

| type | size | classes | passed in |
|------|------|---------|-----------|
| `c2v`       | 8  | SSE            | low 8 bytes of one XMM (two packed floats) |
| `c2Circle`  | 12 | SSE, SSE       | `xmm0` = `{p.x,p.y}`, `xmm1` = `{r}` |
| `c2AABB`    | 16 | SSE, SSE       | `xmm0` = `{min.x,min.y}`, `xmm1` = `{max.x,max.y}` |
| `c2Capsule` | 20 | MEMORY (> 16B) | on the stack |

`C2_TYPE` is an enum whose enumerators fit in `int`, so it is passed as a
32-bit `int`; any `int` value is a representable argument (see `ERRORS.md` #1).

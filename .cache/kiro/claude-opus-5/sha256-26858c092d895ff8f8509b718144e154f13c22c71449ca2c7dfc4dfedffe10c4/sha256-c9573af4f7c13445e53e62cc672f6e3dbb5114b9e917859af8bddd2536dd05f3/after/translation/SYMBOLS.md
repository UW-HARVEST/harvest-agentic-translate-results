# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-oWYE5y.so | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libcollided_lib.so | awk '{print $3}' | sort > /tmp/r_syms.txt
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # -> EMPTY
```

The C library is a single translation unit (`c_src/src/lib.c`), built as
`SHARED` by `c_src/CMakeLists.txt` with no version script and no
`-fvisibility=hidden`, so every non-`static` function it defines is exported.
There are no macro-generated symbols and no second C source file, so there is
no "whole module never translated" gap.

| # | C symbol | kind | C signature | exported by Rust `.so` | Rust item |
|---|----------|------|-------------|------------------------|-----------|
| 1 | `c2V` | T | `c2v c2V(float, float)` | YES | `c2V` |
| 2 | `c2Maxv` | T | `c2v c2Maxv(c2v, c2v)` | YES | `c2Maxv` |
| 3 | `c2Minv` | T | `c2v c2Minv(c2v, c2v)` | YES | `c2Minv` |
| 4 | `c2Clampv` | T | `c2v c2Clampv(c2v, c2v, c2v)` | YES | `c2Clampv` |
| 5 | `c2Sub` | T | `c2v c2Sub(c2v, c2v)` | YES | `c2Sub` |
| 6 | `c2Dot` | T | `float c2Dot(c2v, c2v)` | YES | `c2Dot` |
| 7 | `c2CircletoCircle` | T | `int c2CircletoCircle(c2Circle, c2Circle)` | YES | `c2CircletoCircle` |
| 8 | `c2CircletoAABB` | T | `int c2CircletoAABB(c2Circle, c2AABB)` | YES | `c2CircletoAABB` |
| 9 | `c2AABBtoAABB` | T | `int c2AABBtoAABB(c2AABB, c2AABB)` | YES | `c2AABBtoAABB` |
| 10 | `collided` | T | `int collided(const void*, C2_TYPE, const void*, C2_TYPE)` | YES | `collided` |

**Missing from Rust `.so`: 0.**
**Undefined non-libc symbols in the Rust `.so`: 0** (`nm -D -u` shows only the
libc/`libgcc` imports the Rust runtime needs; the C `.so` imports none).

Types (not symbols, listed for completeness): `c2v` {f32 x, f32 y} = 8 B,
`c2Circle` {c2v p, f32 r} = 12 B, `c2AABB` {c2v min, c2v max} = 16 B,
`C2_TYPE` = `int` (verified: `collided` reads its tags with `cmpl` on `%esi`
/ `%ecx`, i.e. a 4-byte integer).

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so the only
configuration is the default (empty) feature set. `--no-default-features` is
therefore identical to the default build; both are exercised by
`scripts/verify_all.sh`.

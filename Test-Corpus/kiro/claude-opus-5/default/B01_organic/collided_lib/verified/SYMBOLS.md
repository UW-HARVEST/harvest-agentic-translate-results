# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C `.so`:    `c_src/build/libharvest-work-LnnZo1.so`
- Rust `.so`: `translation/target/release/libcollided_lib.so`

Command used:

```sh
nm -D --defined-only <so> | awk '{print $3}' | sort
```

## Symbol table

| # | C symbol | in C `.so` | in Rust `.so` | notes |
|---|----------|-----------|---------------|-------|
| 1 | `c2AABBtoAABB`     | yes | yes | `int c2AABBtoAABB(c2AABB, c2AABB)` |
| 2 | `c2CircletoAABB`   | yes | yes | `int c2CircletoAABB(c2Circle, c2AABB)` |
| 3 | `c2CircletoCircle` | yes | yes | `int c2CircletoCircle(c2Circle, c2Circle)` |
| 4 | `c2Clampv`         | yes | yes | `c2v c2Clampv(c2v, c2v, c2v)` |
| 5 | `c2Dot`            | yes | yes | `float c2Dot(c2v, c2v)` |
| 6 | `c2Maxv`           | yes | yes | `c2v c2Maxv(c2v, c2v)` |
| 7 | `c2Minv`           | yes | yes | `c2v c2Minv(c2v, c2v)` |
| 8 | `c2Sub`            | yes | yes | `c2v c2Sub(c2v, c2v)` |
| 9 | `c2V`              | yes | yes | `c2v c2V(float, float)` |
| 10 | `collided`        | yes | yes | `int collided(const void*, C2_TYPE, const void*, C2_TYPE)` — the only symbol declared in the public header |

## Diff result

```
$ comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # in C, missing from Rust
<empty>
```

**0 missing symbols.** No C source module was skipped: the C library is a single
translation unit (`c_src/src/lib.c`, 98 lines) and every non-static function it
defines has a matching `#[unsafe(no_mangle)] pub extern "C"` definition in
`translation/src/lib.rs`. No stubs / `unimplemented!()` were needed.

## Undefined (imported) symbols

The Rust `.so` imports only libc / Rust-runtime symbols. Check:

```sh
nm -D -u translation/target/release/libcollided_lib.so
```

No non-libc undefined symbols that the C `.so` would have to provide.

## Build products

`c_src/CMakeLists.txt` declares exactly one target — `add_library(... SHARED src/lib.c)`.
There is **no binary executable / driver** in this project, so the
"compare C and Rust stdout" clause of Phase B does not apply.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so the only build
configuration is the default one. `--no-default-features` is equivalent to the
default here (verified by running the full test suite under both).

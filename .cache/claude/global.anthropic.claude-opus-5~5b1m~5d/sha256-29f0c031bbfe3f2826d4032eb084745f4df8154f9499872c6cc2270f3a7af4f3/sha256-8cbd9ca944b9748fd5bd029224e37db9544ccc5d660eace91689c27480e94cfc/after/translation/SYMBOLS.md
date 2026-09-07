# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-bvSxNh.so` (cmake, no `CMAKE_BUILD_TYPE` ⇒ `-O0`)
* Rust `.so`: `translation/target/release/libcollided_lib.so` (`crate-type = ["cdylib"]`)

The whole C library is a single translation unit (`c_src/src/lib.c`, 98 lines).
Every function in it has external linkage (none are `static`/`inline`), so all
ten appear in the dynamic symbol table — not just the one function declared in
the public header (`collided`).

## Symbol table

| # | symbol | C signature | in C `.so` | in Rust `.so` | notes |
|---|--------|-------------|-----------|--------------|-------|
| 1 | `c2V`              | `c2v c2V(float, float)`                     | ✅ T | ✅ T | struct returned in one SSE eightbyte |
| 2 | `c2Maxv`           | `c2v c2Maxv(c2v, c2v)`                      | ✅ T | ✅ T | componentwise `a>b?a:b` |
| 3 | `c2Minv`           | `c2v c2Minv(c2v, c2v)`                      | ✅ T | ✅ T | componentwise `a<b?a:b` |
| 4 | `c2Clampv`         | `c2v c2Clampv(c2v, c2v, c2v)`               | ✅ T | ✅ T | `c2Maxv(lo, c2Minv(a, hi))` |
| 5 | `c2Sub`            | `c2v c2Sub(c2v, c2v)`                       | ✅ T | ✅ T | by-value param mutated |
| 6 | `c2Dot`            | `float c2Dot(c2v, c2v)`                     | ✅ T | ✅ T | `a.x*b.x + a.y*b.y` |
| 7 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle, c2Circle)`  | ✅ T | ✅ T | 12-byte structs, 2 eightbytes each |
| 8 | `c2CircletoAABB`   | `int c2CircletoAABB(c2Circle, c2AABB)`      | ✅ T | ✅ T | mixed 12/16-byte structs |
| 9 | `c2AABBtoAABB`     | `int c2AABBtoAABB(c2AABB, c2AABB)`          | ✅ T | ✅ T | 16-byte structs |
| 10 | `collided`        | `int collided(const void*, C2_TYPE, const void*, C2_TYPE)` | ✅ T | ✅ T | only symbol in `include/lib.h` |

## Diff result

```
$ diff <(nm -D --defined-only c_src/build/*.so      | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/*.so \
           | grep ' T ' | awk '{print $3}' | sort)
(empty)
```

**0 missing symbols. 0 extra exported symbols.** No C module was left
untranslated; no `#[no_mangle]` wrapper is missing; no stubs were added.

## Undefined (imported) symbols

The C `.so` imports nothing but the ELF/glibc boilerplate
(`__gmon_start__`, `_ITM_*`, `__cxa_finalize`). The Rust `.so` imports the usual
Rust-runtime libc set (`memcpy`, `_Unwind_*`, `pthread_*`, `dl_iterate_phdr`,
`__cxa_thread_atexit_impl`, …). All are libc/unwinder symbols resolved by the
system loader — **0 missing/undefined non-libc symbols.**

## Non-symbol surface (from `include/lib.h`)

`C2_TYPE` is an unsigned-int-backed enum with exactly two enumerators,
`C2_TYPE_CIRCLE = 0` and `C2_TYPE_AABB = 1`. It is a compile-time construct, so
it produces no dynamic symbol, but it *is* part of the FFI surface: any `int`
value can be passed across the boundary for it (see `ERRORS.md` rows 1–4).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
feature combination that exists is the default (empty) one. `--no-default-features`
and the default build compile the identical code. Verified by
`cargo check --no-default-features` and `cargo test --no-default-features`.

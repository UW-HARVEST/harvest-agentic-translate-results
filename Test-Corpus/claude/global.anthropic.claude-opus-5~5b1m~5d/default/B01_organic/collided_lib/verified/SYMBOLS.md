# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D --defined-only` on the C shared object built by
`c_src/CMakeLists.txt`.

* C `.so`: `c_src/build/libharvest-work-sJkdWP.so` (name comes from the parent
  directory name via `cmake_path(GET parent FILENAME project_name)`).
* Rust `.so`: `translation/target/{debug,release}/libcollided_lib.so`
  (`[lib] name = "collided_lib"`, `crate-type = ["cdylib"]`).

Regenerate / re-check with:

```sh
./symbol_diff.sh            # debug + release, prints the diff and exits non-zero on mismatch
```

## Public symbol table

Every function in `c_src/src/lib.c` has external linkage (no `static`, no
`inline`), so all ten land in the C `.so`'s dynamic symbol table — not just the
single function declared in `include/lib.h` (`collided`). All ten must therefore
be re-exported by the Rust `.so` with the exact same names.

| # | symbol | C signature | in C `.so` | in Rust `.so` | Rust item |
|---|--------|-------------|-----------|---------------|-----------|
| 1 | `c2V` | `c2v c2V(float, float)` | T | T | `#[no_mangle] pub extern "C" fn c2V` |
| 2 | `c2Maxv` | `c2v c2Maxv(c2v, c2v)` | T | T | `#[no_mangle] pub extern "C" fn c2Maxv` |
| 3 | `c2Minv` | `c2v c2Minv(c2v, c2v)` | T | T | `#[no_mangle] pub extern "C" fn c2Minv` |
| 4 | `c2Clampv` | `c2v c2Clampv(c2v, c2v, c2v)` | T | T | `#[no_mangle] pub extern "C" fn c2Clampv` |
| 5 | `c2Sub` | `c2v c2Sub(c2v, c2v)` | T | T | `#[no_mangle] pub extern "C" fn c2Sub` |
| 6 | `c2Dot` | `float c2Dot(c2v, c2v)` | T | T | `#[no_mangle] pub extern "C" fn c2Dot` |
| 7 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle, c2Circle)` | T | T | `#[no_mangle] pub extern "C" fn c2CircletoCircle` |
| 8 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle, c2AABB)` | T | T | `#[no_mangle] pub extern "C" fn c2CircletoAABB` |
| 9 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB, c2AABB)` | T | T | `#[no_mangle] pub extern "C" fn c2AABBtoAABB` |
| 10 | `collided` | `int collided(const void*, C2_TYPE, const void*, C2_TYPE)` | T | T | `#[no_mangle] pub unsafe extern "C" fn collided` |

**Missing from Rust `.so`: none.** No module of the C source was skipped
(`src/lib.c` is the only translation unit in `add_library`), so no additional
translation work was required and no symbol is stubbed.

## Non-libc undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only the platform runtime
(`libc`/`libgcc_s`/`ld-linux` names such as `memcpy`, `__cxa_thread_atexit_impl`,
`pthread_*`, `dl_iterate_phdr`, `_Unwind_*`), all satisfied by the system
loader. There are no unresolved project symbols.

## Types crossing the boundary (ABI notes, SysV AMD64)

| C type | size/align | classification | Rust |
|--------|-----------|----------------|------|
| `c2v {float x, y;}` | 8 / 4 | one SSE eightbyte → packed low half of one `xmm` | `#[repr(C)] struct c2v { x: f32, y: f32 }` |
| `c2Circle {c2v p; float r;}` | 12 / 4 | two SSE eightbytes → `xmm(n)` = `p`, `xmm(n+1)` = `r` | `#[repr(C)] struct c2Circle` |
| `c2AABB {c2v min, max;}` | 16 / 4 | two SSE eightbytes → `xmm(n)` = `min`, `xmm(n+1)` = `max` | `#[repr(C)] struct c2AABB` |
| `C2_TYPE` enum | 4 / 4 | INTEGER, passed in a 32-bit GPR | `pub type C2_TYPE = c_uint` |
| `int` | 4 | INTEGER, returned in `eax` | `c_int` |

Verified against the C disassembly: `c2CircletoCircle` receives
`xmm0=A.p, xmm1=A.r, xmm2=B.p, xmm3=B.r`; `collided` receives
`rdi=A, esi=typeA, rdx=B, ecx=typeB` and compares the tags with `cmpl $0x0` /
`cmpl $0x1`, so only the 32-bit pattern matters (signed vs unsigned enum
representation is unobservable).

## Verification result (Phase D)

`./symbol_diff.sh` — run against both profiles:

```
--- debug:   C exports 10, Rust exports 10
OK: 0 missing symbols (debug)
--- release: C exports 10, Rust exports 10
OK: 0 missing symbols (release)
```

The symbol diff is **empty in both directions**: the Rust `.so` exports exactly
the same ten names as the C `.so` and nothing else (no leaked Rust runtime
symbols). All undefined symbols in the Rust `.so` are glibc imports
(`memcpy`, `malloc`, `pthread_key_create`, `dl_iterate_phdr`, …).

Additional guarantee: `tests/harness_sanity.rs` parses `/proc/self/maps` and
asserts that every C function pointer lies inside the C `.so`'s mapping and
every Rust function pointer inside `libcollided_lib.so`'s mapping, so the
differential tests provably call two different libraries through `dlopen`.

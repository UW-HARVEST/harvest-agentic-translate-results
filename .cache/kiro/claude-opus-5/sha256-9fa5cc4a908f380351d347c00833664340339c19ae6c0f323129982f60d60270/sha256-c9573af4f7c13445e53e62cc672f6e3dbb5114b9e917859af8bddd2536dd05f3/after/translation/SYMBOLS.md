# SYMBOLS.md — Symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-EB1pzP.so
nm -D --defined-only translation/target/release/libcircle_collide_lib.so
```

C source surface: `c_src/src/lib.c` (144 lines) — the ONLY translation unit in
`c_src/CMakeLists.txt`. Public header `c_src/include/lib.h` declares only
`circle_collide`, but the C file defines every other function with external
linkage too (no `static`), so all 12 land in the dynamic symbol table.

## Defined dynamic symbols

| # | C symbol | C ABI signature (from `src/lib.c`) | in C `.so` | in Rust `.so` | status |
|---|----------|------------------------------------|-----------|---------------|--------|
| 1 | `c2V`              | `c2v (float, float)`                    | T | T | OK |
| 2 | `c2Mulvs`          | `c2v (c2v, float)`                      | T | T | OK |
| 3 | `c2Maxv`           | `c2v (c2v, c2v)`                        | T | T | OK |
| 4 | `c2Minv`           | `c2v (c2v, c2v)`                        | T | T | OK |
| 5 | `c2Clampv`         | `c2v (c2v, c2v, c2v)`                   | T | T | OK |
| 6 | `c2Sub`            | `c2v (c2v, c2v)`                        | T | T | OK |
| 7 | `c2Dot`            | `float (c2v, c2v)`                      | T | T | OK |
| 8 | `c2CircletoCircle` | `int (c2Circle, c2Circle)`              | T | T | OK |
| 9 | `c2CircletoAABB`   | `int (c2Circle, c2AABB)`                | T | T | OK |
|10 | `c2CircletoCapsule`| `int (c2Circle, c2Capsule)`             | T | T | OK |
|11 | `c2Collided`       | `int (const void*, const void*, C2_TYPE)` | T | T | OK |
|12 | `circle_collide`   | `int (float, float, float)`             | T | T | OK |

**Missing from Rust `.so`: 0.** No `#[no_mangle]` wrapper had to be added and
no C module was left untranslated — `src/lib.c` is the whole library and every
one of its 12 external-linkage functions has a real Rust implementation (no
stubs, no `unimplemented!()`).

## Extra symbols exported by Rust but not C

None. `nm -D --defined-only` on the Rust `.so` yields exactly the same 12 names.

## Undefined (imported) symbols

* C `.so`: only the 4 weak CRT/ITM/gmon placeholders (`_ITM_*`,
  `__cxa_finalize`, `__gmon_start__`). No libm — all math is inline SSE.
* Rust `.so`: libc (`memcpy`, `malloc`, `abort`, …), the `_Unwind_*` personality
  routines and the std panic-machinery imports. **All are libc / language-runtime
  symbols; 0 undefined non-libc symbols**, i.e. nothing from the translated
  library itself is left dangling.

## Struct ABI classification (must match for by-value passing)

| type | size | SysV class | passed as |
|------|------|-----------|-----------|
| `c2v`       |  8 | SSE            | 1 xmm register |
| `c2Circle`  | 12 | SSE, SSE       | 2 xmm registers |
| `c2AABB`    | 16 | SSE, SSE       | 2 xmm registers |
| `c2Capsule` | 20 | MEMORY (>16 B) | on the stack |

`#[repr(C)]` on all four Rust structs reproduces these, verified empirically by
the differential tests in `tests/` (a mis-classified `c2Capsule` would corrupt
`c2CircletoCapsule` immediately).

## Verification gate

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc / non-runtime symbols in the Rust `.so`.
- [x] Name-for-name diff of the two defined-symbol lists is empty.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
build configuration is the default one (`cargo test`,
`cargo test --no-default-features` and `cargo test --all-features` are all the
same code path). `check_features.sh` derives the list from `Cargo.toml` rather
than hard-coding it, and runs the whole suite for each combination under **both**
cargo profiles (release and debug). Verified output:

```
combinations to verify: 3
  []
  [--no-default-features]
  [--all-features]
RESULT: all 3 feature combination(s) x 2 profiles PASSED
```

Running debug as well as release is not redundant: debug-assertions turn Rust's
"misaligned pointer dereference" check on, which is how the `c2Collided`
unaligned-load defect was caught (see `ERRORS.md`).

## Independent re-verification command

```sh
diff <(nm -D --defined-only --format=posix c_src/build/*.so            | awk '{print $1}' | sort) \
     <(nm -D --defined-only --format=posix translation/target/release/libcircle_collide_lib.so \
                                                                       | awk '{print $1}' | sort)
# -> no output: the symbol diff is empty
```

`tests/symbol_parity.rs` performs this same diff as an assertion, additionally
requires the C table to contain exactly 12 entries (so a shrunken C build cannot
make the diff pass vacuously), and *calls* all 12 symbols through `dlsym` with
inputs whose answers differ, so a symbol that exists but is a constant stub
fails.

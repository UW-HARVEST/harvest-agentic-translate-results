# SYMBOLS.md — Phase A: exported-symbol surface

## Source inventory (mechanical)

`c_src/CMakeLists.txt` builds ONE shared library from exactly two translation
units:

```
add_library(${project_name} SHARED src/match.c src/spectral_contrast.c)
target_link_libraries(${project_name} m)
```

`c_src/include/match.h` (the only public header, 5 lines) declares:

```c
#define N_SMOOTH 16              /* not a symbol — a macro */
typedef double float_t;          /* not a symbol — a typedef */

int    match(float_t *test, float_t *reference, int bins, double threshold);
double spectral_contrast(float_t *a, float_t *b, int length);
```

Every other function in the two `.c` files is declared `static`, so it is a
local symbol (`t`, lowercase) and is NOT part of the exported ABI:

| C function | file | linkage | exported? |
|---|---|---|---|
| `total` | `src/match.c` | `static` | no (local `t`) |
| `smoothen` | `src/match.c` | `static` | no (local `t`) |
| `differentiate` | `src/match.c` | `static` | no (local `t`) |
| `preprocess` | `src/match.c` | `static` | no (local `t`) |
| `match` | `src/match.c` | external | **YES** |
| `dot_product` | `src/spectral_contrast.c` | `static` | no (local `t`) |
| `normalize` | `src/spectral_contrast.c` | `static` | no (local `t`) |
| `spectral_contrast` | `src/spectral_contrast.c` | external | **YES** |

No macro-generated symbols exist (no symbol-generating macros anywhere in the
sources). There is no binary/driver target in `CMakeLists.txt` — library only,
so there is no stdout comparison to make.

## `nm -D` on the C `.so`

`c_src/build/libharvest-work-cvRa1v.so`, defined non-weak symbols:

```
0000000000001322 T match
00000000000015cd T spectral_contrast
```

Undefined (imported from libc/libm), expected and irrelevant to parity:

```
U memcpy@GLIBC_2.14
U sqrt@GLIBC_2.2.5
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

## `nm -D` on the Rust `.so`

`translation/target/release/libunderhanded_c_nuke_lib.so`, defined non-weak:

```
00000000000129b0 T match
0000000000012bc0 T spectral_contrast
```

Undefined (imported) in the Rust `.so`, all of them platform-runtime plumbing
rather than API surface, exactly analogous to the C `.so`'s `memcpy@GLIBC_2.14`
and `sqrt@GLIBC_2.2.5`:

* `@GLIBC_*` — libc (`memcpy`, `malloc`, `write`, …)
* `@GCC_*` — libgcc's unwinder (`_Unwind_Resume`, `_Unwind_GetIP`, …), pulled in
  by the Rust standard library
* weak: `__cxa_finalize`, `__cxa_thread_atexit_impl`, `__gmon_start__`,
  `gettid`, `statx`, `_ITM_registerTMCloneTable`,
  `_ITM_deregisterTMCloneTable`

After filtering those, the Rust `.so` has **0 unresolved non-libc symbols**.
`run_all.sh` step 4 performs this check mechanically for every feature
combination.

## Parity table

| C symbol | type | present in Rust `.so` | Rust item | notes |
|---|---|---|---|---|
| `match` | `T` | **yes** | `#[unsafe(no_mangle)] pub unsafe extern "C" fn r#match` | `match` is a Rust keyword, so `r#match` is used; `no_mangle` still emits the plain name `match` |
| `spectral_contrast` | `T` | **yes** | `#[unsafe(no_mangle)] pub unsafe extern "C" fn spectral_contrast` | |

## Diff

```
$ comm -23 <(c_defined_T) <(rust_defined_T)      # in C, missing from Rust
<empty>
```

**0 missing symbols. 0 undefined non-libc symbols in the Rust `.so`.**
No translation gaps: both `.c` files are fully translated (`src/lib.rs`
contains Rust bodies for all 8 C functions, including the 6 `static` helpers,
which are kept private exactly as in C).

## Cargo build note

`translation/Cargo.toml` originally declared `name = "underhanded-c-nuke_lib"`
for the `[lib]` target, which Cargo rejects (`library target names cannot
contain hyphens`) — the crate did not even parse. Renamed to
`underhanded_c_nuke_lib`; the produced object is
`libunderhanded_c_nuke_lib.so`. `libloading = "0.8"` added to
`[dev-dependencies]`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, and `src/lib.rs`
contains **no `#[cfg(feature = ...)]`** attributes:

```
$ grep -c 'cfg(feature' src/lib.rs   -> 0
$ grep -c '^\[features\]' Cargo.toml -> 0
```

The C likewise has no `#ifdef`-selected code (`grep -c '#if' c_src/src/*.c
c_src/include/*.h` -> 0). Therefore the only feature combination is the
default one, and `--no-default-features` is equivalent to it. Both are still
exercised (see `run_all.sh`) to satisfy the Phase D gate.

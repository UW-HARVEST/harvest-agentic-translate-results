# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-k6ca19.so` (cmake, default flags, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust `.so`: `translation/target/release/libnext_double_lib.so` (`crate-type = ["cdylib"]`, `[lib] name = "next_double_lib"`)

## C exported (defined) dynamic symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-k6ca19.so
0000000000001164 T next_double
```

Weak/undefined entries in the C `.so` (toolchain boilerplate, not API):
`_ITM_deregisterTMCloneTable` (w), `_ITM_registerTMCloneTable` (w),
`__cxa_finalize@GLIBC_2.2.5` (w), `__gmon_start__` (w).

## Rust exported (defined) dynamic symbols

```
$ nm -D --defined-only translation/target/release/libnext_double_lib.so
00000000000116a0 T next_double
```

## Parity table

| # | C symbol | type | exported by Rust `.so`? | Rust item | notes |
|---|----------|------|-------------------------|-----------|-------|
| 1 | `next_double` | `T` (global text) | YES — exact name | `#[unsafe(no_mangle)] pub unsafe extern "C" fn next_double(rnd: *mut cn_rnd_t) -> c_double` | only public API symbol |

### Symbols in C but NOT in Rust

**(none)** — the diff is empty.

```
$ diff <(nm -D --defined-only C.so   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only RUST.so| awk '{print $NF}' | sort)
(no output)
```

### Non-API symbols, for completeness

* `cn_rnd_next` is `static` in `c_src/src/lib.c`, so it is **not** a dynamic
  symbol of the C `.so`. It is translated as a private Rust `fn cn_rnd_next`
  and is correspondingly **not** exported. Not exporting it is required parity,
  not an omission.
* `cn_rnd_t` is a `typedef struct` — a type, not a symbol. Translated as
  `#[repr(C)] pub struct cn_rnd_t { pub state: [u64; 2] }`. Layout parity is
  asserted at runtime by the differential tests, which hand the same 16-byte
  buffer to both libraries.

### Undefined symbols in the Rust `.so`

All `U`/`w` entries are libc / libgcc-unwind imports pulled in by the Rust
standard library (`malloc`, `memcpy`, `mmap64`, `_Unwind_*`, `pthread_key_*`,
`dl_iterate_phdr`, …). There are **0 undefined non-libc/non-runtime symbols**,
i.e. nothing from the translated library itself is left unresolved.

```
$ nm -D --undefined-only RUST.so | grep -vE '@GLIBC|@GCC|_ITM_|__gmon_start__|gettid|statx' | wc -l
0
```

## Completeness of the translation

`c_src` contains exactly one translation unit (`src/lib.c`, 20 lines) and one
header (`include/lib.h`, 7 lines). Both are fully translated: no C source file,
function, or symbol was skipped, and no Rust symbol is a stub.

| C source | lines | translated? | Rust location |
|----------|-------|-------------|---------------|
| `include/lib.h` — `cn_rnd_t` | 3–5 | yes | `src/lib.rs` `struct cn_rnd_t` |
| `include/lib.h` — `next_double` decl | 7 | yes | `src/lib.rs` `next_double` |
| `src/lib.c` — `cn_rnd_next` (static) | 3–12 | yes | `src/lib.rs` `fn cn_rnd_next` |
| `src/lib.c` — `next_double` | 14–20 | yes | `src/lib.rs` `next_double` |

## Phase D verification (automated)

`run_all.sh` step 4 re-checks the diff for every feature combination and both
profiles, and fails the run if it is ever non-empty:

```
=== 4. symbol parity (nm -D), every combination x profile ===
  OK    symbol diff empty      combo=default profile=debug
  OK    symbol diff empty      combo=default profile=release
  OK    symbol diff empty      combo=none    profile=debug
  OK    symbol diff empty      combo=none    profile=release
```

Checklist:

* [x] every C-exported symbol is exported by the Rust `.so` under the exact same name
* [x] symbol diff is empty (1 of 1 symbols present)
* [x] 0 unresolved non-libc/non-runtime symbols in the Rust `.so`
* [x] no stubbed, faked, or `unimplemented!()` symbol — the single export is a
      complete, line-by-line translation of `next_double` + `cn_rnd_next`
* [x] holds for both feature configurations (`default`, `--no-default-features`)
      and both profiles

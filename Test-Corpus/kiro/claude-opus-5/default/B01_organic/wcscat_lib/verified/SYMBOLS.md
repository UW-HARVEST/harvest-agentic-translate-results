# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## C shared library

`c_src/build/libharvest-work-CWGPNw.so` (built from the single translation unit
`c_src/src/lib.c`; the only declaration in `c_src/include/lib.h` is `wcscat`).

```
$ nm -D --defined-only c_src/build/libharvest-work-CWGPNw.so
00000000000010f9 T wcscat
```

## Rust shared library

`translation/target/release/libwcscat_lib.so` (`crate-type = ["cdylib"]`).

```
$ nm -D --defined-only translation/target/release/libwcscat_lib.so
0000000000011690 T wcscat
```

## Parity table

| # | symbol   | type in C `.so` | exported by Rust `.so` | notes |
|---|----------|-----------------|------------------------|-------|
| 1 | `wcscat` | `T` (global text) | yes (`T`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn wcscat` in `src/lib.rs` |

## Symbol diff

```
$ diff <(nm -D --defined-only <C .so>  | awk '{print $NF}' | sort) \
       <(nm -D --defined-only <Rust .so> | awk '{print $NF}' | sort)
(empty)
```

**Missing from Rust: none.** No C module/file was left untranslated: `c_src` has
exactly one source file (`src/lib.c`, 21 lines) defining exactly one function.

## Undefined (imported) non-libc symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` resolves entirely against glibc and
the Rust runtime — after filtering the `__*`/`_*` runtime and unwinder imports
the remainder is:

```
fstat64@GLIBC_2.33  getcwd@GLIBC_2.2.5  gettid@GLIBC_2.30  lseek64@GLIBC_2.2.5
realpath@GLIBC_2.3  stat64@GLIBC_2.33   statx@GLIBC_2.28
```

All libc. **0 unresolved project symbols.** Independently confirmed by
`libloading::Library::new` succeeding and a call through the loaded symbol
returning correct results (`phase_d_symbols::rust_so_has_no_unresolved_project_symbols`).

## Harness self-check (mutation / negative control)

To prove the differential suite has real detection power rather than passing
vacuously, three mutants of the Rust source were built and run against the
unmodified C `.so`; each was caught:

| mutant | change | caught by |
|--------|--------|-----------|
| `m1` | `return 22` → `return 23` | all 6 Phase C null/zero-length rows + the absolute-code anchor test |
| `m2` | drop `*dst = 0` on the `34` overflow path | 9 Phase B rows (10, 13–17, 19–21) |
| `m3` | `end = dst + numElem` → `dst + numElem - 1` (off-by-one bound) | 7 Phase B rows (04–06, 08–11) |

`src/lib.rs` was restored byte-identically afterwards (verified with `diff`).

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one. There is no `src/main.rs` / `[[bin]]`, so the
project builds no driver executable (nothing to compare on stdout).

# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libdriver.so
```

## C `.so` full `nm -D` output

| symbol | type | meaning |
|--------|------|---------|
| `driver` | `T` | **defined, exported** — the only public symbol |
| `__ctype_b_loc@GLIBC_2.3` | `U` | undefined import (glibc); pulled in by the `isXXX()` *macros* in `<ctype.h>` |
| `printf@GLIBC_2.2.5` | `U` | undefined import (glibc) |
| `setlocale@GLIBC_2.2.5` | `U` | undefined import (glibc) |
| `tolower@GLIBC_2.2.5` | `U` | undefined import (glibc) — real function call, not a macro |
| `toupper@GLIBC_2.2.5` | `U` | undefined import (glibc) — real function call, not a macro |
| `_ITM_deregisterTMCloneTable` | `w` | weak, toolchain-generated |
| `_ITM_registerTMCloneTable` | `w` | weak, toolchain-generated |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | weak, toolchain-generated |
| `__gmon_start__` | `w` | weak, toolchain-generated |

Note: the presence of `__ctype_b_loc` as the only classifier-related import
confirms that the twelve `isXXX` calls in `driver.c` are expanded from glibc's
`__isctype()` **macro**, which yields the *raw masked table bits* (e.g.
`iscntrl('\0') == 2`), not a normalised `0`/`1`. `tolower`/`toupper` remain real
calls into glibc. Both facts are reproduced by the Rust translation.

## Symbol parity table

Every symbol **defined** by the C `.so` must also be defined by the Rust `.so`
with the exact same name.

| # | C-defined symbol | present in Rust `.so`? | Rust type | status |
|---|------------------|------------------------|-----------|--------|
| 1 | `driver`         | yes                    | `T`       | [x] OK |

No macro-generated / mangled extra exports exist in the C `.so` (`driver.c` has
no other external linkage: no globals, no non-`static` helpers).

### Diff

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $NF}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
```

Result: **empty** — 0 missing symbols.

The Rust `.so` additionally has no undefined non-libc symbols; its `U`/`w`
entries are all glibc (`printf`, `setlocale`, `malloc`, `memcpy`, `write`, ...)
plus the `_Unwind_*` family from `libgcc`, which is the normal `std` runtime
surface for a `cdylib` and is satisfied by the loader.

Verified `U`-symbol check:

```sh
nm -D -u translation/target/release/libdriver.so \
  | awk '{print $NF}' | grep -v '@GLIBC' | grep -v '@GCC' \
  | grep -v '_ITM_\|__gmon_start__'
```

Result: **empty** — 0 unresolved non-libc symbols.

## Completeness of the translation

`c_src` contains exactly one translation unit (`src/driver.c`, 48 lines) and one
public header (`include/driver.h`, declaring `void driver(char c);`). No C source
file was skipped; `translation/src/lib.rs` covers the whole library. There are no
stubs, no `unimplemented!()`, and no `todo!()` in the Rust crate:

```sh
grep -rn 'unimplemented!\|todo!\|unreachable!' translation/src/   # -> no matches
```

## Import-surface parity

Beyond exported symbols, the Rust `.so` now resolves the **same** ctype/locale
entry points as the C, which is what makes the behaviour identical rather than
merely similar:

| import | C `.so` | Rust `.so` |
|--------|---------|------------|
| `__ctype_b_loc@GLIBC_2.3` | yes | yes |
| `tolower@GLIBC_2.2.5` | yes | yes |
| `toupper@GLIBC_2.2.5` | yes | yes |
| `setlocale@GLIBC_2.2.5` | yes | yes |
| `printf@GLIBC_2.2.5` | yes | yes |

An earlier revision of the translation imported **none** of the first three: it
had frozen the `"C"`-locale ctype tables into the crate and reimplemented the
case mapping. That is invisible to `nm -D --defined-only` and to any happy-path
test, but it diverges from the C under a `uselocale()` thread locale. Two of the
three bugs verification found were exactly that (see `CONFIGS.md`), which is why
the import surface is checked here and not just the export surface.

## Note on the exported prototype

The Rust `driver` is declared `extern "C" fn driver(c_arg: c_int)` rather than
`(c: c_char)`, and narrows to 8 bits in the body. The exported *symbol* is
unchanged and the two are indistinguishable to any conforming caller; the reason
is that GCC's code for `void driver(char c)` discards bits 8..31 of the argument
register while rustc's `signext` `i8` parameter trusts them. Matching the C's
behaviour for a caller that leaves garbage there requires the explicit narrowing.
Details in `ERRORS.md` row 8 and in the doc comment on `driver`.

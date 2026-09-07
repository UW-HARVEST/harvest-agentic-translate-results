# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
C   : c_src/build/libdriver.so
Rust: translation/target/release/libdriver.so
```

## Public (defined, dynamic) symbols exported by the C `.so`

`nm -D --defined-only c_src/build/libdriver.so`

| # | symbol | type | C declaration (`c_src/include/driver.h`) | exported by Rust `.so`? |
|---|--------|------|------------------------------------------|-------------------------|
| 1 | `driver` | `T` (global text) | `void driver(int x, int y);` | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(x: c_int, y: c_int)` |

The C library is a single translation unit (`c_src/src/driver.c`) containing a
single non-static function. There are no macro-generated symbols, no exported
data symbols, no `static` helpers promoted to global linkage, and no additional
C source files. Therefore the complete public surface is the one row above.

### Verification

```
$ nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort > /tmp/c.syms
$ nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort > /tmp/r.syms
$ comm -23 /tmp/c.syms /tmp/r.syms      # in C but missing from Rust
<empty>
```

**0 symbols missing from the Rust `.so`.** No module of the C source was left
untranslated; no stubs were introduced.

## Undefined (imported) symbols

The C `.so` imports, from libc:

| symbol | used by |
|--------|---------|
| `div@GLIBC_2.2.5` | `driver` |
| `printf@GLIBC_2.2.5` | `driver` |
| `__cxa_finalize`, `__gmon_start__`, `_ITM_*registerTMCloneTable` | weak, toolchain-generated |

The Rust `.so` imports the same two functional symbols — `div@GLIBC_2.2.5` and
`printf@GLIBC_2.2.5` — because the translation deliberately delegates to the
identical libc routines rather than re-implementing them. This is what makes
the degenerate inputs (see `ERRORS.md`) trap identically.

The Rust `.so` additionally imports the usual Rust runtime/libc set
(`_Unwind_*`, `malloc`, `memcpy`, `mmap64`, `pthread_key_create`, `dl_iterate_phdr`,
`abort`, …). These are **all libc / libgcc_s symbols pulled in by the Rust
standard library**, not undefined project symbols:

```
$ nm -D --undefined-only translation/target/release/libdriver.so \
    | grep -v 'GLIBC\|GCC_\|_ITM_\|__gmon_start__'
<empty>
```

**0 missing/undefined non-libc symbols in the Rust `.so`.** ✔

## Completion checklist for this artifact

- [x] Every `nm -D` defined symbol of the C `.so` is exported by the Rust `.so` with the exact same name.
- [x] `comm -23 c.syms r.syms` is empty.
- [x] No `unimplemented!()`, `todo!()`, or behaviour-faking stub exists in `translation/src`.
- [x] Every undefined symbol in the Rust `.so` resolves to libc/libgcc.

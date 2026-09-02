# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D` on both shared objects.

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Source inventory (completeness check)

`c_src` contains exactly one translation unit and one public header:

| C file | functions defined | translated in |
|--------|-------------------|---------------|
| `c_src/src/goto.c` | `forward_goto_example`, `open_with_cleanup`, `driver` | `translation/src/goto.rs` |
| `c_src/include/goto.h` | declares `driver` only | — |

No C source file is untranslated, so no module needed to be written from
scratch for Phase A/D.

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `driver` | `T` | `T` | declared in `include/goto.h`; documented entry point |
| 2 | `forward_goto_example` | `T` | `T` | not in the header but `extern`-visible in the C `.so`, so the Rust `.so` must export it too |
| 3 | `open_with_cleanup` | `T` | `T` | ditto; returns `FILE*` |

**Missing from Rust `.so`: 0.** No `#[no_mangle]` wrapper needed to be added
and no C module needed to be translated.

## Undefined (imported) symbols

The C `.so` imports only `stdio` entry points plus CRT glue:
`fclose ferror fgets fopen fprintf fwrite printf stderr`,
`__cxa_finalize __gmon_start__ _ITM_*`.

The Rust `.so` imports that same set (it calls the host `stdio` directly via
`src/cstdio.rs`) plus the standard Rust runtime's libc/libgcc dependencies
(`_Unwind_*`, `malloc`, `memcpy`, `abort`, `pthread_key_*`, …). All of these
resolve against `libc.so.6` / `libgcc_s.so.1`:

```
$ ldd translation/target/release/libdriver.so
	libgcc_s.so.1 => /lib64/libgcc_s.so.1
	libc.so.6 => /lib64/libc.so.6
```

**0 missing/undefined non-libc symbols.** Because both objects bind to the
same `libc.so.6` at run time, they share one set of `FILE` objects for
`stdout`/`stderr`, which is what makes byte-level output comparison in the
same test process meaningful.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one. `--no-default-features` and the
default build are therefore the same code. Both are still exercised in
Phase D for completeness.

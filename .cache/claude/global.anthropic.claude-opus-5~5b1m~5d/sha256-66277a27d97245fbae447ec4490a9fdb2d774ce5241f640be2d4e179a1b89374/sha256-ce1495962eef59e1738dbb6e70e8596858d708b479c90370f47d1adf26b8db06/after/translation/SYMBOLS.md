# SYMBOLS.md — public symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001109 T driver
(plus only-weak/absolute entries: _init, _fini, __bss_start, _edata, _end,
 and the glibc weak refs __gmon_start__/__cxa_finalize which are UNDEFINED)

$ nm -D --defined-only translation/target/release/libdriver.so
0000000000011700 T driver
```

## Symbol table

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `driver` | `T` (global text) | `T` (global text) | `void driver(int x, int y)` — the only symbol declared in `include/driver.h`; exported from Rust with `#[unsafe(no_mangle)] pub extern "C"` |

## Linker-synthesized / toolchain symbols (not API, not required to match)

| symbol | C | Rust | notes |
|--------|---|------|-------|
| `_init` / `_fini` | present | absent | crt glue emitted by GNU ld for the C `.so`; not part of the API and never called by a consumer |
| `__bss_start`, `_edata`, `_end` | present | present | linker-provided absolute section markers (`A`/`B`), not functions |
| `__gmon_start__`, `__cxa_finalize`, `_ITM_*`, `__tls_get_addr` | `w`/`U` | `w`/`U` | weak/undefined libc hooks |

## Undefined (imported) symbols

| `.so` | undefined non-libc symbols |
|-------|----------------------------|
| C | none (only `puts`/`printf` from libc) |
| Rust | none (only `printf` + `libc`/`ld` runtime symbols) |

Verification command used:

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so      | awk '$2=="T"{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '$2=="T"{print $3}' | sort)
```

Result: **empty diff** → 0 missing symbols in the Rust `.so`. No C source file was
left untranslated: `c_src/src/driver.c` is the only translation unit and its only
function is `driver`.

> Note: the C compiler lowers `printf("loop\n")` to `puts("loop")`; the Rust
> translation calls `printf` with the same literal. Both go through the *same*
> process-wide libc `stdout` FILE, so the emitted bytes and buffering are
> identical. This is verified byte-for-byte by the differential tests.

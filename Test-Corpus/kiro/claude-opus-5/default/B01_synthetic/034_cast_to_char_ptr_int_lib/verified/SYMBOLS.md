# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (`c_src/`)

The whole library is two files:

| file | lines | contents |
|------|-------|----------|
| `c_src/include/driver.h` | 28 | declares `void driver(int x);` |
| `c_src/src/driver.c` | 37 | `static void print_hex(unsigned char *p, int len)`, `void driver(int x)` |

There is no second module, so there is no un-translated C file. Both C
functions are present in `translation/src/lib.rs` (`print_hex` is `static` in C
and therefore stays private in Rust — it is intentionally NOT exported, matching
the C `.so`).

## Exported (defined, dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `driver` | `T` @0x1173 | `T` @0x11730 | `void driver(int)` — the only public API |

`print_hex` is absent from both `.so`s (C: `static`; Rust: private `unsafe fn`).
This is correct parity, not a gap.

### Symbol diff

```
comm -3 <(nm -D --defined-only c_src/build/libdriver.so    | awk '{print $NF}' | sort -u) \
        <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
```

Result for the *defined* set, after filtering the toolchain-emitted weak
symbols (`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`, `__cxa_thread_atexit_impl`, `gettid`,
`statx`) that both objects get from their respective runtimes:

**EMPTY — 0 symbols missing from the Rust `.so`.**

## Undefined (imported) symbols

C `.so` imports exactly:

| symbol | source |
|--------|--------|
| `printf@GLIBC_2.2.5` | `printf("%02x", ...)` |
| `putchar@GLIBC_2.2.5` | compiler rewrite of `printf("\n")` |

Rust `.so` imports the same two (`printf`, `putchar` — LLVM applies the same
`printf("\n")` → `putchar('\n')` rewrite), plus the Rust standard-library /
unwinder support imports (`_Unwind_*`, `malloc`, `free`, `memcpy`,
`dl_iterate_phdr`, …). All of these resolve against libc / libgcc, i.e. there
are **0 unresolved non-libc undefined symbols**.

## Completion gate item

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.

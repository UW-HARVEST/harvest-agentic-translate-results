# SYMBOLS.md — Public symbol parity (C `.so` vs Rust `.so`)

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libdriver.so             | awk '{print $3}' | sort -u > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort -u > /tmp/rs_syms.txt
comm -23 /tmp/c_syms.txt /tmp/rs_syms.txt   # missing from Rust
```

## Translation units in `c_src`

| C source file | translated? | Rust location |
|---|---|---|
| `c_src/src/lib.c` | yes | `translation/src/lib.rs` |

`c_src/include/lib.h` declares exactly one entry point; there are no other
`.c` files, so no module was skipped.

## Exported (defined, global) symbols

| # | C symbol | type in C `.so` | exported by Rust `.so` | notes |
|---|----------|-----------------|------------------------|-------|
| 1 | `tool_basename` | `T` (global text) | **yes**, `T` | `#[unsafe(no_mangle)] pub unsafe extern "C"`. Header has no renaming macros, so the linker name is verbatim. |

The C `.so` exports **1** non-`libc` symbol. Weak/compiler-generated entries
(`_init`, `_fini`, `__bss_start`, `_edata`, `_end`, `__gmon_start__`,
`_ITM_*`, `__cxa_finalize`) are toolchain artifacts, not part of the library
API, and are excluded on both sides.

## Diff result

- Symbols in C `.so` missing from Rust `.so`: **0** (`comm -23` output empty).
- Undefined symbols in the Rust `.so`: all resolve to `libc`/`libgcc_s`
  (`memcpy`, `strlen`, `malloc`, `free`, `__errno_location`, `_Unwind_*`, …)
  — pulled in by the Rust runtime, **0** non-libc undefined symbols.

Nothing was stubbed. `tool_basename` is a real translation of the C body,
including its private helper `strrchr` (re-implemented rather than delegated to
libc so the pointer-comparison semantics are identical).

## Verdict

- [x] `nm -D` shows **0** missing symbols in Rust.
- [x] `nm -D` shows **0** undefined non-libc symbols in Rust.

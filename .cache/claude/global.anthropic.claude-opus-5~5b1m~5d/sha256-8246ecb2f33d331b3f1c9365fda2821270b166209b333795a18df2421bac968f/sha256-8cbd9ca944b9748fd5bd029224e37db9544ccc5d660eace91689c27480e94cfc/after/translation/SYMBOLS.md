# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C:    `c_src/build/libdriver.so`
- Rust: `translation/target/release/libdriver.so`

## Defined (exported) dynamic symbols

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `driver` | `T` (yes) | `T` (yes) | `void driver(int x)` — declared in `include/driver.h`; only public API of the library |

C source files: `c_src/src/driver.c` (only one). It defines exactly one
function, `driver`. No macro-generated symbols, no global data, no static
tables, no other translation units. Therefore the complete public surface is
the single symbol above; nothing was skipped in the translation.

## Symbol diff

```
$ comm -3 <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $NF}' | sort) \
          <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
(empty)
```

**Missing from Rust: 0.**  **Extra in Rust: 0.**

## Undefined (imported) symbols

The C `.so` imports only `printf` (plus the usual weak glibc/CRT hooks
`__cxa_finalize`, `__gmon_start__`, `_ITM_*TMCloneTable`).

The Rust `.so` imports `printf` as well, plus the libc/`libgcc` symbols pulled
in by the Rust standard library and its unwinder (`malloc`, `memcpy`,
`_Unwind_*`, `pthread_key_*`, …). **All Rust imports are libc / libgcc runtime
symbols — there are 0 missing or undefined non-libc symbols.**

## Verification result

Re-checked after all work (`verify_all.sh` step "Symbol parity"):

```
-- PASS: exported symbol sets identical
```

- missing from Rust `.so`: **0**
- undefined non-libc symbols in Rust `.so`: **0**
- nothing was stubbed: `driver` is a real translation of `c_src/src/driver.c`
  (`grep -c 'unimplemented!\|todo!\|panic!("stub' src/lib.rs` → 0).

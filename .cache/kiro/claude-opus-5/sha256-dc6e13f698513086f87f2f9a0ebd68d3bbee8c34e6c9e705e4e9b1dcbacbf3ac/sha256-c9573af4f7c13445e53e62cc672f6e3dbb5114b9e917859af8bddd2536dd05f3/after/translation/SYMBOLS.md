# SYMBOLS.md — dynamic symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory

The entire C library is two files:

| file | lines | function definitions |
|------|-------|----------------------|
| `c_src/include/driver.h` | 30 | none (declares `driver`) |
| `c_src/src/driver.c` | 43 | `print_foo`, `driver` |

`grep -n '^[a-zA-Z].*(' c_src/src/driver.c` yields exactly those two definitions,
plus the `foo_t` typedef. There is no macro-generated symbol machinery, no
second translation unit, and no conditionally compiled module. So the complete
expected export set has **two** entries — no C source was left untranslated.

## Exported (T) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `driver`    | T | T | `void driver(unsigned int, unsigned int, bool, int)` |
| 2 | `print_foo` | T | T | `void print_foo(const foo_t *)` — not declared in the public header, but has external linkage in C and is therefore part of the ABI surface |

**Missing from Rust: 0.**

Note on `print_foo`: it is *not* `static` in `driver.c`, so the C compiler gives
it external linkage and it appears in `nm -D`. The Rust translation correctly
exports it too (`#[unsafe(no_mangle)] pub unsafe extern "C" fn print_foo`), so
an external caller can reach the lower-level entry point in both libraries. It
is treated as a first-class public entry point throughout Phases B and C.

## Undefined (U) symbols — imports

| symbol | C `.so` | Rust `.so` | libc? |
|--------|---------|------------|-------|
| `printf` | U | U | yes (glibc) |

Both libraries import `printf` from the platform libc and nothing else outside
the standard C runtime / loader boilerplate (`__cxa_finalize`,
`__gmon_start__`, `_ITM_*` on the C side; the Rust `cdylib` adds only its own
`std`-internal statics, all local). There are **0 undefined non-libc symbols**
in the Rust `.so`.

## Deliberately NOT exported

Nothing. The Rust crate exports no symbol that the C library does not, other
than the linker-generated section symbols every ELF shared object carries
(`_init`, `_fini`, `_edata`, `_end`, `__bss_start`), which are present in both.

## ABI nuance: the internal `driver` → `print_foo` call

The C `driver` calls `print_foo` **through the PLT**
(`call 1040 <print_foo@plt>`), so under symbol interposition the call could bind
to a different definition. The Rust `driver` calls its own `print_foo` directly.
This is unobservable here: the two `print_foo` implementations are byte-identical
(proved by `CONFIGS.md` rows 9–13 and `ERRORS.md` rows 12–14), so whichever one
the C's PLT binds to produces the same output. Recorded for completeness.

## Verification checklist

- [x] `nm -D` shows 0 symbols present in C `.so` but missing from Rust `.so`.
      Verified in both `debug` and `release` profiles:
      `diff <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort) \
            <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)`
      → empty.
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.
      Checked mechanically in `tests/phase_d_symbol_parity.rs::d02` by resolving
      every undefined symbol against the union of the `.so`'s own `ldd`
      dependencies — not against a hand-written allowlist.
- [x] The Rust `.so` exports nothing the C `.so` does not (`d03`).
- [x] Both symbols are reachable via `dlsym`, not merely present in `nm` (`d05`).
- [x] No stubs, no `unimplemented!()`, no `todo!()` anywhere in `src/`
      (asserted by `d04`).

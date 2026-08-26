# SYMBOLS.md — Phase A symbol surface

## How this was derived

`c_src/CMakeLists.txt` builds `src/main.c` as an **executable** (`add_executable`).
The same translation unit also builds cleanly as a shared object, which is how the
exported-symbol surface is enumerated:

```
gcc -shared -fPIC -O0 -o capi_build/libdriver_c.so c_src/src/main.c
nm -D --defined-only capi_build/libdriver_c.so
```

The Rust crate mirrors both shapes (see `Cargo.toml`):

* `[[bin]] driver` → `target/release/driver`, compared against `c_src/build/driver`
* `[lib] crate-type = ["cdylib"]` → `target/release/libdriver.so`, compared against
  `capi_build/libdriver_c.so`

## Exported symbol table

`main.c` gives external linkage to five functions. `goodG2B` and `goodB2G` are declared
`static`, so they have internal linkage and are **correctly absent** from both shared
objects — the Rust side must not export them either, and does not.

| # | C symbol | C declaration | in C `.so` | in Rust `.so` | Rust definition |
|---|----------|---------------|-----------|---------------|-----------------|
| 1 | `printLine`    | `void printLine(const char *line)` | T | T | `src/lib.rs` |
| 2 | `printIntLine` | `void printIntLine(int intNumber)` | T | T | `src/lib.rs` |
| 3 | `bad`          | `void bad()`                       | T | T | `src/lib.rs` |
| 4 | `good`         | `void good()`                      | T | T | `src/lib.rs` |
| 5 | `main`         | `int main(int argc, char *argv[])` | T | T | `src/lib.rs` |

> Exporting `main` from the cdylib collides with the entry point cargo's *unit-test*
> harness generates (`error: entry symbol 'main' declared multiple times`). `Cargo.toml`
> sets `test = false` / `doctest = false` on `[lib]`; every test in this crate is an
> integration test under `tests/`, so nothing is lost. The binary target avoids the same
> collision by pulling in the implementation with `#[path = "imp.rs"] mod imp;` instead of
> linking the library crate.

### Deliberately NOT exported (internal linkage in C)

| C symbol | C declaration | reason |
|----------|---------------|--------|
| `goodG2B` | `static void goodG2B()` | `static` → not in `.dynsym`; Rust keeps it private (`imp::good_g2b`) |
| `goodB2G` | `static void goodB2G()` | `static` → not in `.dynsym`; Rust keeps it private (`imp::good_b2g`) |

## Symbol diff

Produced by `tests/symbol_parity.rs` and by `scripts/verify.sh`:

```
symbols in C .so but MISSING from Rust .so:   (empty)
```

The diff is **empty**: every symbol the C shared object exports is exported by the Rust
shared object under the exact same name. No symbol is stubbed — each export forwards to
a real translation of the corresponding C body in `src/imp.rs`.

## Undefined (imported) symbols

The C `.so` imports only libc: `printf`, `puts`, `fgets`, `atoi`, `stdin`,
plus the usual `__stack_chk_fail` / `_ITM_*` / `__gmon_start__` / `__cxa_finalize`
toolchain weak references.

The Rust `.so` imports libc as well (via `std`). Its `DT_NEEDED` list is
`libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2` — system libraries only — so there
are **0 undefined non-libc symbols** and it is loadable standalone exactly as the C `.so`
is. Verified by `dlopen(..., RTLD_NOW)` (which resolves *every* relocation eagerly and
therefore fails loudly on any unresolved symbol) followed by `dlsym` of all five names;
both shared objects pass identically. `scripts/verify.sh` re-runs this check.

> Note: `puts` appears in the C imports because gcc rewrites `printf("%s\n", line)`
> into `puts(line)`. That is a pure code-generation detail with identical observable
> output and does not affect the symbol contract.

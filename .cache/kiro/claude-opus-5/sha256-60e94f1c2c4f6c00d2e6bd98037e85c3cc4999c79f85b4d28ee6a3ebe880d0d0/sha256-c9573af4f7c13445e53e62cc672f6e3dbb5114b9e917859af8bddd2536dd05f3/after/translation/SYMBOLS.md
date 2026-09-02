# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (`c_src/src/driver.c`, `c_src/include/driver.h`)

| C function | linkage in C | must be exported? |
|---|---|---|
| `printLine`        | external (`void printLine(const char*)`) | YES |
| `printHexCharLine` | external (`void printHexCharLine(char)`)  | YES |
| `bad`              | external (`void bad(void)`)               | YES |
| `goodG2B`          | **`static`** — file-local                | no (must NOT be exported) |
| `goodB2G`          | **`static`** — file-local                | no (must NOT be exported) |
| `good`             | external (`void good(void)`)              | YES |
| `driver`           | external (`void driver(int)`), declared in `driver.h` | YES |

There is exactly one translation unit (`src/driver.c`); no module was skipped.
No macro-generated symbols exist in this library.

## Dynamic-symbol table comparison

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `printLine`        | `T` | `T` | MATCH |
| 2 | `printHexCharLine` | `T` | `T` | MATCH |
| 3 | `bad`              | `T` | `T` | MATCH |
| 4 | `good`             | `T` | `T` | MATCH |
| 5 | `driver`           | `T` | `T` | MATCH |

`goodG2B` / `goodB2G` are absent from BOTH `.so` files — correct, they are
`static` in C and private (`unsafe fn`, no `#[no_mangle]`) in Rust.

**Symbol diff (C exported minus Rust exported): EMPTY.**
**Extra non-libc symbols exported by Rust beyond the C set: NONE.**

### ABI note on `printHexCharLine`

The symbol name matches, but the Rust wrapper deliberately declares its
parameter as `c_int` and narrows to `c_char` itself. C's `char` parameter is
narrowed by the *callee* (gcc emits `mov %edi,%eax; mov %al,-0x4(%rbp);
movsbl -0x4(%rbp),%eax`), whereas rustc assumes an `i8` parameter was already
narrowed by the caller and forwards the whole register (`mov %edi,%esi`). This
was a real divergence found by the differential tests — see ERRORS.md row E10.
Post-fix the Rust emits `movsbl %dil,%esi`, matching C's semantics. The change
is ABI-compatible with any well-formed `char` caller.

## Undefined (imported) symbols

C `.so` imports: `printf`, `puts`, plus weak ITM/`__cxa_finalize`/`__gmon_start__`.

> Note: gcc rewrites `printf("%s\n", line)` into `puts(line)`, which is why the
> C `.so` imports `puts`. The Rust translation calls `printf("%s\n", ...)`
> directly. Byte stream on stdout is identical; this is a codegen detail, not a
> behavioural difference. (Confirmed by the differential tests.)

Rust `.so` imports: `printf`, `puts` (via `std`), and otherwise only libc /
libgcc-unwind symbols (`malloc`, `memcpy`, `dl_iterate_phdr`, `_Unwind_*`, …)
pulled in by the Rust standard library.

**0 missing / undefined non-libc symbols in the Rust `.so`.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
complete set of feature combinations is the single default (empty) combination.
Verified mechanically — see `check_features.sh`, which parses the `[features]`
table, builds the powerset, and runs `cargo check` + the full differential suite
for each. Both `--default` and `--no-default-features` are exercised, and each
against **both** cargo profiles (`debug`, which unwinds, and `release`, which is
`panic = "abort"` and fully optimised) since the two produce different codegen.

Result: 4 build/test rounds, each 41/41 passing, each with an empty symbol diff.

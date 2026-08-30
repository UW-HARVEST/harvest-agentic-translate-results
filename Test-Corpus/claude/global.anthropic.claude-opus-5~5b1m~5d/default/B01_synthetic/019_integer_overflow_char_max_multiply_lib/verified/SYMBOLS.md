# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C   : `c_src/build/libdriver.so` (cmake, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust: `translation/target/release/libdriver.so` (`crate-type = ["cdylib"]`)

## Defined (exported) symbols

`nm -D --defined-only <so>`:

| # | symbol | C signature (`driver.h` / `driver.c`) | in C `.so` | in Rust `.so` | status |
|---|--------|---------------------------------------|------------|---------------|--------|
| 1 | `printLine`        | `void printLine(const char *line)` | T | T | ✅ present |
| 2 | `printHexCharLine` | `void printHexCharLine(char charHex)` | T | T | ✅ present (Rust wrapper declares the parameter `c_int` and truncates explicitly — see ERRORS.md E7; same symbol name, same machine-level ABI as GCC's callee) |
| 3 | `bad`              | `void bad(void)` | T | T | ✅ present |
| 4 | `good`             | `void good(void)` | T | T | ✅ present |
| 5 | `driver`           | `void driver(int useGood)` | T | T | ✅ present |

**Symbol diff (C exported \ Rust exported): EMPTY.**

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $3}' | sort -u) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort -u)
(no output)
```

## Deliberately NOT exported (matching C)

| C symbol | reason |
|----------|--------|
| `goodG2B` | declared `static` in `driver.c` → internal linkage, absent from `nm -D` of the C `.so`. Translated as a private `unsafe fn goodG2B()` in Rust — correctly *not* exported. |
| `goodB2G` | declared `static` in `driver.c` → same as above. |

Both static functions ARE translated (they are reachable through the exported
`good()` / `driver(1)`), they are simply not given `#[no_mangle]` because the C
library does not expose them either. Exporting them would be a *surplus* symbol,
not a parity fix.

## Undefined / imported symbols

`nm -D --undefined-only`:

* C `.so`: `printf`, `puts` (GCC lowers `printf("%s\n", p)` to `puts(p)`),
  plus the usual weak CRT hooks (`__cxa_finalize`, `__gmon_start__`,
  `_ITM_*`).
* Rust `.so`: `printf`, `puts`, plus the Rust `std` runtime's libc/libgcc
  imports (`malloc`, `memcpy`, `write`, `_Unwind_*`, `pthread_key_*`, …).

**0 missing / unresolvable non-libc symbols in the Rust `.so`.** Every
non-weak undefined symbol resolves out of `libc.so.6` / `libgcc_s.so.1`,
verified by successfully `dlopen`ing the Rust `.so` in every integration test
(a missing dependency would make `libloading::Library::new` fail).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
possible configuration is the default (empty) feature set. `cargo test`,
`cargo test --no-default-features` and
`cargo test --all-features` are therefore all the same build; the
`scripts/check_features.sh` helper enumerates and runs them anyway.

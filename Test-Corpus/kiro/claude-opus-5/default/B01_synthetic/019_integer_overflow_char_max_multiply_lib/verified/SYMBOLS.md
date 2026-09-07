# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Source of truth: `nm -D --defined-only` on the two shared libraries.

- C:    `c_src/build/libdriver.so`      (built from `c_src/src/driver.c` only — CMakeLists builds a single SHARED lib, no executable target)
- Rust: `translation/target/release/libdriver.so` (`crate-type = ["cdylib"]`)

## C source inventory (completeness check)

`c_src` contains exactly one translation unit and one public header:

| C file | translated? |
|---|---|
| `c_src/include/driver.h` | yes — declares only `void driver(int)` |
| `c_src/src/driver.c` | yes — all 7 function definitions are present in `translation/src/lib.rs` |

Function-by-function coverage of `driver.c`:

| C function | C linkage | present in Rust? | Rust item |
|---|---|---|---|
| `printLine(const char *)` | external | yes | `printLine` (`#[no_mangle] extern "C"`) |
| `printHexCharLine(char)` | external | yes | `printHexCharLine` (`#[no_mangle] extern "C"`) |
| `bad(void)` | external | yes | `bad` (`#[no_mangle] extern "C"`) |
| `goodG2B(void)` | **static** | yes | private `unsafe fn goodG2B` — correctly NOT exported |
| `goodB2G(void)` | **static** | yes | private `unsafe fn goodB2G` — correctly NOT exported |
| `good(void)` | external | yes | `good` (`#[no_mangle] extern "C"`) |
| `driver(int)` | external | yes | `driver` (`#[no_mangle] extern "C"`) |

No module/file was skipped; nothing is stubbed or `unimplemented!()`.

## Dynamic symbol table — defined (exported)

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `bad`              | T | T | match |
| 2 | `driver`           | T | T | match |
| 3 | `good`             | T | T | match |
| 4 | `printHexCharLine` | T | T | match |
| 5 | `printLine`        | T | T | match |

`goodG2B` / `goodB2G` are absent from BOTH tables (they are `static` in C) — correct.

**Symbol diff (C-exported minus Rust-exported): EMPTY.**

Verification command (must print nothing):

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
```

## Undefined symbols in the Rust `.so`

0 missing/undefined **non-libc** symbols. Every `U`/`w` entry resolves to glibc
(`printf`, `puts`, `memcpy`, `malloc`, `write`, …) or to the platform unwinder
(`_Unwind_*`, `__gmon_start__`, `_ITM_*`), which the Rust runtime always
references. The C `.so` references the same `printf`/`puts` pair.

Note: gcc rewrites the C `printf("%s\n", line)` into `puts(line)`; the Rust side
calls `printf` directly. Both emit the identical byte stream to `stdout` and use
the same stdio buffer, so this is not an observable difference (asserted by the
differential tests in `tests/`).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the complete
set of feature combinations is:

| # | combination | cargo invocation |
|---|---|---|
| 1 | default (empty) | `cargo test` |
| 2 | `--no-default-features` (identical to #1) | `cargo test --no-default-features` |

Both are exercised by `run_all_features.sh`.

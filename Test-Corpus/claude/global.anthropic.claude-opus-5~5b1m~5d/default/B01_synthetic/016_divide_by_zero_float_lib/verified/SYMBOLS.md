# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared libraries.

- C:    `c_src/build/libdriver.so`
- Rust: `translation/target/release/libdriver.so`

## Exported (dynamic, defined) symbols

| # | symbol | C `.so` | Rust `.so` | C signature | notes |
|---|--------|---------|------------|-------------|-------|
| 1 | `printLine`    | T | T | `void printLine(const char *line)` | NULL-guarded; `printf("%s\n", line)` |
| 2 | `printIntLine` | T | T | `void printIntLine(int intNumber)`  | `printf("%d\n", intNumber)` |
| 3 | `bad`          | T | T | `void bad(float data)`              | unguarded `(int)(100.0 / data)` |
| 4 | `good`         | T | T | `void good(float data)`             | calls `goodG2B()` then `goodB2G(data)` |
| 5 | `driver`       | T | T | `void driver(float goodData, float badData)` | header-declared entry point |

**Symbol diff (C minus Rust): EMPTY.** All 5 exported C symbols are exported by
the Rust `.so` under the exact same names. No symbol required a new
`#[no_mangle]` wrapper and no C module was left untranslated.

## Deliberately NOT exported (correctly so)

These are `static` in `c_src/src/driver.c`, therefore have internal linkage and
do **not** appear in `nm -D` of the C `.so`. The Rust translation likewise keeps
them as private `fn`s, which is the correct parity:

| C symbol | linkage in C | Rust counterpart |
|----------|--------------|------------------|
| `goodG2B` | `static void goodG2B()`          | private `fn goodG2B()` |
| `goodB2G` | `static void goodB2G(float data)` | private `fn goodB2G(data: c_float)` |

They are still covered by the differential tests indirectly, because `good` and
`driver` are the only ways to reach them — exactly as in C.

## Undefined (imported) symbols

The Rust `.so` imports only libc symbols. Notably it deliberately imports
`printf` rather than using `std::io::stdout`, so that output ordering,
buffering and formatting are produced by the *same* libc `printf` the C library
uses. This is what makes byte-identical capture through a single redirected
`stdout` fd possible.

Verification command used:

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, and
`src/lib.rs` contains **no `#[cfg(...)]` / `feature =` gates** (verified by
grep). Therefore there is exactly ONE build configuration, and the default
`cargo test` run covers the complete feature surface. `--no-default-features`
is equivalent to the default here.

# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Commands:

```
nm -D --defined-only c_src/build/libdriver.so | sort
nm -D --defined-only translation/target/release/libdriver.so | sort
```

## C `.so` exported (defined, global) symbols

| symbol | type | declared in header? | source |
|--------|------|---------------------|--------|
| `driver` | `T` (text/global) | yes — `c_src/include/driver.h:27` | `c_src/src/driver.c:37` |
| `foo`    | `T` (text/global) | no (external linkage, still exported) | `c_src/src/driver.c:29` |

The C translation unit is a single file (`src/driver.c`), so there is no
untranslated module. `CMakeLists.txt` builds exactly one target,
`add_library(driver SHARED src/driver.c)` — no executable/driver binary, so the
"compare binary stdout" clause of the task has no applicable target. (Verified:
`CMakeLists.txt` contains no `add_executable`.)

`strchr` is the only libc dependency of the algorithm; `printf` is the only
libc dependency of `driver`. Both are *undefined* imports in the C `.so`, not
exports, so they are not part of the parity requirement.

## Rust `.so` exported symbols

| symbol | Rust item | file |
|--------|-----------|------|
| `driver` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` | `src/lib.rs:107` |
| `foo`    | `#[unsafe(no_mangle)] pub unsafe extern "C" fn foo`    | `src/lib.rs:86`  |

## Parity diff

```
$ comm -3 <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $3}' | sort) \
          <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(empty)
```

- Symbols in C but missing from Rust: **0**
- Undefined non-libc symbols in the Rust `.so`: **0** (only `printf`,
  `_Unwind_Resume`-class runtime and glibc symbols are undefined, matching the
  C `.so`'s own reliance on `printf`/`strchr`).

Status: **PASS** — no export needs adding, no C module needs translating.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, therefore the only build configuration is the default one.
`cargo check --no-default-features` is equivalent to `cargo check`. Verified by
grepping Cargo.toml for `[features]` (0 hits). Phase D's "every feature
combination" therefore collapses to a single combination, plus the debug/release
profile split which is also exercised.

# SYMBOLS.md — Phase A: public symbol surface

Source of truth: `nm -D --defined-only` on the C shared object
`c_src/build/libharvest-work-wY6FQr.so`, compared against the Rust cdylib
`translation/target/release/libldexp_q2_lib.so`.

## C source inventory (completeness check)

The whole C tree is:

```
c_src/CMakeLists.txt
c_src/include/lib.h     (1 line  — single prototype)
c_src/src/lib.c         (12 lines — single function definition)
```

`CMakeLists.txt` compiles exactly one translation unit (`src/lib.c`) into one
`SHARED` library. There are no other modules, no `#ifdef`-selected files, no
generated sources, and no executable/driver target. Therefore the complete
public surface is the single prototype in `include/lib.h`:

```c
float ldexp_q2(float y, int exp_q2);
```

No macro-generated symbols exist (the C file contains no function-defining
macros; the `min`-style expression inside `ldexp_q2` is an inline ternary, not a
macro that expands to a symbol).

## Symbol table

| # | symbol | C `.so` | Rust `.so` | binding/type | notes |
|---|--------|---------|------------|--------------|-------|
| 1 | `ldexp_q2` | ✅ `T` | ✅ `T` | global text | `extern "C"` + `#[unsafe(no_mangle)]` in `src/lib.rs` |

### Missing from Rust

None. The symbol diff is EMPTY.

Verification command:

```sh
diff <(nm -D --defined-only c_src/build/libharvest-work-wY6FQr.so \
        | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libldexp_q2_lib.so \
        | awk '{print $3}' | sort)
```

### Undefined (imported) symbols

The C `.so` imports only the usual libc/ld startup relocations
(`__cxa_finalize`, `_ITM_*`, `__gmon_start__`). The Rust `.so` imports only
libc/`ld` symbols as well (`memcpy`, `__cxa_thread_atexit_impl`, unwinding
stubs, etc.). There are 0 undefined *non-libc* symbols in the Rust `.so`, i.e.
nothing that would fail to resolve at load time — confirmed by the fact that
`libloading::Library::new` succeeds on it in the integration tests.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only build
configuration is the default one. `cargo check --no-default-features` and
`cargo check` are equivalent here; both are exercised by
`tests/feature_matrix.sh`.

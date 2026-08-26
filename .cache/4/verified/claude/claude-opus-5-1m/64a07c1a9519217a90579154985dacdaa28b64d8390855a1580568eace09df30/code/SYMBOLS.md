# SYMBOLS.md — Exported symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
# C shared library (out-of-tree so nothing under c_src/ is modified)
cmake -S c_src -B target/c_build -DCMAKE_POSITION_INDEPENDENT_CODE=ON
cmake --build target/c_build          # -> target/c_build/libdriver.so

# Rust shared library (crate-type = ["cdylib"])
cargo build --offline                 # -> target/debug/libdriver.so
```

## Feature / configuration surface

`Cargo.toml` has **no `[features]` section** and `c_src/CMakeLists.txt` defines
**no build options / `option()` / `target_compile_definitions`**. There are no
`#ifdef`/`#if` configuration branches anywhere in `c_src/src/driver.c` or
`c_src/include/driver.h` (only the `DRIVER_H_` include guard).

Therefore the complete set of valid feature combinations is exactly one:

| # | cargo feature combination | command |
|---|---------------------------|---------|
| 1 | *(none — the empty set, which is also the default)* | `cargo check --no-default-features` / `cargo test --no-default-features` |

`cargo check --no-default-features` → **clean, 0 errors, 0 warnings**.

## Defined (exported) dynamic symbols

`nm -D --defined-only`:

| symbol | C `.so` | Rust `.so` | status |
|--------|---------|------------|--------|
| `driver` | `T driver` | `T driver` | ✅ present in both |
| `run`    | `T run`    | `T run`    | ✅ present in both |

`run` is **not** declared in `c_src/include/driver.h`, but it is a
non-`static` function in `driver.c` and therefore an exported symbol / real
public entry point of the C `.so`. The Rust side exports it with
`#[unsafe(no_mangle)] pub unsafe extern "C" fn run(...)`, so an external caller
sees the same ABI (`house_t*`, `int`). Phase B tests call it directly.

The remaining C functions (`add_floor`, `add_bedrooms`, `print_house`,
`parse_val`) are `static` in the C source and are **not** exported by the C
`.so`; they are correctly translated as private Rust `fn`s and must not appear
in `nm -D`. Verified: neither `.so` exports them.

### Symbol diff

```
comm -23 <(nm -D --defined-only target/c_build/libdriver.so | awk '{print $NF}' | sort) \
         <(nm -D --defined-only target/debug/libdriver.so   | awk '{print $NF}' | sort)
```

→ **empty**. Every symbol exported by the C `.so` is exported by the Rust `.so`
with the exact same name. 0 missing symbols. No whole C module was skipped:
`driver.c` is the only translation unit in `CMakeLists.txt`, and every one of its
functions is present in `src/lib.rs`.

## Undefined (imported) symbols

Not a parity requirement (the Rust std runtime legitimately imports more), but
recorded because the *libc* imports must match for byte-identical formatting:

| import | C `.so` | Rust `.so` | note |
|--------|---------|------------|------|
| `printf@GLIBC_2.2.5`  | U | U | same libc formatter → identical `%d` / `%.1f` bytes |
| `puts@GLIBC_2.2.5`    | U | – | gcc rewrote `printf("An error occurred\n")` → `puts("An error occurred")`; byte-identical output, so the Rust `printf` call is equivalent |
| `strtol@GLIBC_2.2.5`  | U | U | same libc parser → identical accept/reject and `endptr` |
| `__errno_location@GLIBC_2.2.5` | U | U | same thread-local `errno` cell |

There are **0 undefined non-libc / non-runtime symbols** in the Rust `.so`: every
`U` entry resolves to `libc.so.6`, `libgcc_s.so.1` (`_Unwind_*`) or is a weak
optional symbol. Confirmed loadable by `libloading::Library::new` in every test.

## Verification

`tests/phase_d_symbols.rs` re-runs `nm -D` on both `.so`s at test time and
asserts the diff above is empty, so this table cannot silently rot. Verified
empty for **both** the `debug` and the `release` Rust `.so`.

`./run_all.sh` enumerates the feature combinations straight out of `Cargo.toml`
(no hard-coding) and runs `cargo check` + the whole differential suite for each,
in the `dev` and `release` profiles. Latest run:

```
### feature combination: <none/default>
  cargo check                          -> Finished, 0 errors, 0 warnings
  phase_b_configs   25 passed; 0 failed
  phase_c_errors    15 passed; 0 failed
  phase_d_symbols    4 passed; 0 failed
### release profile (panic = "abort")
  phase_b_configs   25 passed; 0 failed
  phase_c_errors    15 passed; 0 failed
  phase_d_symbols    4 passed; 0 failed
```

Also re-run green with `RUSTFLAGS="-C debug-assertions=on" cargo test --release`,
which proves the pointer-access fix described in `CONFIGS.md` is robust to the
UB-check flags rather than depending on a particular profile.

Note: the tests load the Rust library **only** through
`libloading::Library::new(...libdriver.so)` + `dlsym`, never by linking the crate
(`crate-type = ["cdylib"]` only), so the `#[no_mangle] extern "C"` wrappers and
the C ABI are themselves part of what is verified.

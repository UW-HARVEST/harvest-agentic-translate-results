# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Libraries compared:

* C:    `c_src/build/libtranslated_rust.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust: `target/debug/libpow43_lib.so` (`crate-type = ["cdylib"]`)

Commands used:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so | sort
nm -D --defined-only target/debug/libpow43_lib.so      | sort
diff <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort -u) \
     <(nm -D --defined-only target/debug/libpow43_lib.so      | awk '{print $NF}' | sort -u)
```

## Defined (exported) dynamic symbols

| # | C symbol (`nm -D`) | type | present in Rust `.so` | Rust item |
|---|--------------------|------|-----------------------|-----------|
| 1 | `pow43`            | `T` (global text) | **yes** (`T pow43`) | `#[unsafe(no_mangle)] pub extern "C" fn pow43(x: c_int) -> f32` in `src/lib.rs` |

`diff` of the two defined-symbol name lists is **empty** (exit status 0):
the Rust `.so` exports exactly the same public surface as the C `.so`.

Notes on the C source's non-exported items (nothing to export, kept internal in
Rust too, exactly like the C):

| C item | linkage in C | Rust counterpart | exported? |
|--------|--------------|------------------|-----------|
| `static const float g_pow43[129 + 16]` (`c_src/src/lib.c:3`) | `static` → internal, not in `nm -D` | `static g_pow43: [f32; 129 + 16]` (private) | no — matches C |

Public header `c_src/include/lib.h` declares exactly one prototype
(`float pow43(int x);`), so there are no further entry points to translate and
no whole C file / module was skipped: `c_src/src/lib.c` is the only translation
unit in `c_src/CMakeLists.txt` (`add_library(${project_name} SHARED src/lib.c)`)
and it is fully translated in `src/lib.rs` (table + function).

## Undefined (imported) symbols

The C `.so` imports only the four weak toolchain symbols
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`).

The Rust `.so` imports those same weak symbols plus the Rust-standard-library
runtime's libc/libgcc dependencies (`malloc`, `memcpy`, `abort`,
`_Unwind_*`, `pthread_key_create`, …). All of them are **libc / libgcc**
symbols provided by the platform, i.e. there are **0 missing or unresolved
non-libc symbols**:

```sh
$ ldd -r target/debug/libpow43_lib.so   # no "undefined symbol" lines
```

## Automated as tests

The comparison above is not only documented, it is enforced by
`tests/symbols.rs` (run by `cargo test`), which shells out to `nm`/`ldd`:

| test | asserts |
|------|---------|
| `phase_d_rust_exports_every_c_symbol` | every `nm -D --defined-only` symbol of the C `.so` is also defined by the Rust `.so` (set difference must be empty) |
| `phase_d_rust_exports_no_extra_public_api` | the Rust `.so` exports nothing *beyond* the C surface; the set is exactly `{pow43}` |
| `phase_d_no_unresolved_non_libc_symbols` | `ldd -r` reports no `undefined symbol`, and no unresolved mangled Rust symbol (`_ZN…`/`_RN…`) remains |

`./verify.sh symbols` performs the same diff from the shell.

## Status

* [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with
  the exact same name (`pow43`).
* [x] Symbol diff is empty in both directions (no extra public symbols either).
* [x] `nm -D` shows 0 missing/undefined non-libc symbols for the Rust `.so`.
* [x] Verified for the single build configuration of this crate (see
  `CONFIGS.md` §0 — the crate has no non-empty feature combinations).

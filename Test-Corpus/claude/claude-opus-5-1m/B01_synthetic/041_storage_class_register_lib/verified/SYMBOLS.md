# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#   -> translated_rust/c_src/build/libdriver.so

cd translated_rust && cargo build --offline
#   -> translated_rust/target/debug/libdriver.so
```

## C source inventory (whole-library completeness check)

`c_src/CMakeLists.txt` builds exactly one translation unit:

| C file | translated? | Rust location |
|--------|-------------|---------------|
| `c_src/src/driver.c` (1 function: `driver`) | yes | `src/lib.rs` (`pub extern "C" fn driver`) |
| `c_src/include/driver.h` (1 declaration: `void driver(int x);`) | yes | `src/lib.rs` |

No C file / module is missing from the translation; there is nothing to stub.

## Exported (defined) dynamic symbols

`nm -D --defined-only`:

| symbol | C `.so` | Rust `.so` | status |
|--------|---------|-----------|--------|
| `driver` | `T driver` | `T driver` | present in both |

Counts: C exports 1 defined dynamic symbol, Rust exports 1 defined dynamic
symbol. **Symbol diff (C-exported minus Rust-exported) is EMPTY.**

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort) \
           <(nm -D --defined-only target/debug/libdriver.so | awk '{print $NF}' | sort)
(no output)
```

## Weak / undefined symbols (informational)

C `.so` undefined/weak: `printf@GLIBC_2.2.5` (U), plus the usual toolchain
weak symbols `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`.

Rust `.so` undefined/weak: the same libc `printf@GLIBC_2.2.5` plus only
libc / libgcc-unwind imports pulled in by the Rust runtime
(`malloc`, `free`, `memcpy`, `write`, `_Unwind_*`, `pthread_key_*`, …).

**0 missing / unresolved non-libc symbols in the Rust `.so`** — `ldd` resolves
everything against `libc.so.6`, `libgcc_s.so.1`, `ld-linux-x86-64.so.2`:

```
$ ldd target/debug/libdriver.so
	linux-vdso.so.1
	libgcc_s.so.1 => /lib64/libgcc_s.so.1
	libc.so.6 => /lib64/libc.so.6
	/lib64/ld-linux-x86-64.so.2
```

No macro-generated symbol families exist in this C source (no function-generating
macros, no `#ifdef`-gated alternative names), so there are no hidden exports to
reproduce.

## Feature combinations

`Cargo.toml` has **no `[features]` table**, so the complete enumeration of valid
feature combinations is:

| # | combination | cargo invocation |
|---|-------------|------------------|
| 1 | (none — default, empty feature set) | `cargo check/test --no-default-features` |
| 2 | (default features == empty set, i.e. identical to #1) | `cargo check/test` |

`c_src/CMakeLists.txt` likewise defines no options, no `target_compile_definitions`
and the C source contains no `#ifdef` other than the `DRIVER_H_` include guard,
so the C library has exactly one build configuration too.

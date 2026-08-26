# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## How this was produced

```sh
# C reference
cd c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D libdriver.so

# Rust translation
cargo build --offline
nm -D target/debug/libdriver.so
```

The C build (`c_src/CMakeLists.txt`) compiles exactly one translation unit,
`src/driver.c`, into `libdriver.so`. Its only public header is
`include/driver.h`, which declares one function: `void driver(char c);`.
No `#ifdef`s, no CMake options, no macro-generated symbol names.

## `nm -D` on the C `.so` — full output

```text
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 U __ctype_b_loc@GLIBC_2.3
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001149 T driver
                 U printf@GLIBC_2.2.5
                 U setlocale@GLIBC_2.2.5
                 U tolower@GLIBC_2.2.5
                 U toupper@GLIBC_2.2.5
```

## Symbol table

`nm` class legend: `T` = defined, exported text symbol (this is what "the
library exports"); `U` = undefined import that the loader must satisfy;
`w` = weak undefined (optional, supplied by the CRT/toolchain, never by this
library).

| # | symbol | C class | in Rust `.so`? | Rust class | notes |
|---|--------|---------|----------------|------------|-------|
| 1 | `driver` | `T` (defined) | **YES** | `T` (defined) | The complete exported public ABI. Rust: `#[unsafe(no_mangle)] pub extern "C" fn driver(c: c_int)` in `src/lib.rs`, which narrows to `char` (`c as u8 as c_char`) and delegates to `driver_impl`. The parameter is the widened type the C ABI actually delivers — declaring it `c_char` let an optimised build skip the narrowing that the C callee always performs; see `CONFIGS.md` § "Divergence found and fixed". |
| 2 | `__ctype_b_loc` | `U` (import) | yes | `U` | Imported by both — glibc's `isalnum`/`isalpha`/… macros expand to `(*__ctype_b_loc())[(int)c] & mask`. Not an export. |
| 3 | `printf` | `U` (import) | yes | `U` | Both call libc `printf` — same `FILE *stdout`, same formatting, same buffering. Not an export. |
| 4 | `setlocale` | `U` (import) | yes | `U` | Both call `setlocale(LC_ALL, "C")`. Not an export. |
| 5 | `tolower` | `U` (import) | no (see note) | — | The C `.so` is built with no `-O` flag, so `__OPTIMIZE__` is undefined and `tolower` stays an out-of-line glibc call. glibc defines it as `c >= -128 && c < 256 ? (*__ctype_tolower_loc())[c] : c`, so for every `char` it is exactly the table lookup Rust performs via `__ctype_tolower_loc`. Verified byte-identical for all 256 `char` values (Phase B row 1). Not an export. |
| 6 | `toupper` | `U` (import) | no (see note) | — | Same as `tolower`, via `__ctype_toupper_loc`. Not an export. |
| 7 | `_ITM_deregisterTMCloneTable` | `w` | yes | `w` | Toolchain weak stub. Not an export. |
| 8 | `_ITM_registerTMCloneTable` | `w` | yes | `w` | Toolchain weak stub. Not an export. |
| 9 | `__cxa_finalize` | `w` | yes | `w` | Toolchain weak stub. Not an export. |
| 10 | `__gmon_start__` | `w` | yes | `w` | Toolchain weak stub. Not an export. |

## Defined-symbol diff (the parity gate)

```sh
nm -D --defined-only c_src/build/libdriver.so   | awk '{print $NF}' | sort -u  # -> driver
nm -D --defined-only target/debug/libdriver.so  | awk '{print $NF}' | sort -u  # -> driver
diff <(...) <(...)                                                             # -> empty
```

* C `.so` defined/exported symbols: **1** (`driver`)
* Rust `.so` defined/exported symbols: **1** (`driver`)
* **Missing from Rust: 0. Extra in Rust: 0. Diff is empty.**

This diff is asserted automatically by
`tests/phase_d_symbols.rs::rust_so_exports_every_symbol_the_c_so_exports`, so
it is re-checked on every `cargo test` run rather than only by hand.

## Undefined-symbol audit for the Rust `.so`

Requirement: *0 missing/undefined non-libc symbols*.

Every `U`/`w` entry in `nm -D target/debug/libdriver.so` is either

* a glibc symbol (`printf`, `setlocale`, `__ctype_b_loc`,
  `__ctype_tolower_loc`, `__ctype_toupper_loc`, `malloc`, `memcpy`,
  `pthread_key_create`, `open64`, `write`, …), or
* a `libgcc_s` unwinder symbol (`_Unwind_*`) pulled in by Rust `std`'s
  panic-unwind runtime.

There are **no undefined symbols that this library itself should have
defined**. Confirmed to actually resolve at load time — `dlopen` of the Rust
`.so` succeeds in every test (`ldd -r` likewise reports no unresolved
symbols), which would fail if any non-libc symbol were missing.

## Completeness of the translation

Nothing in `c_src/` was skipped:

| C source file | lines | translated to | status |
|---|---|---|---|
| `c_src/include/driver.h` | 28 (23 = licence header) | `src/lib.rs` (`driver` signature) | complete |
| `c_src/src/driver.c` | 48 (23 = licence header) | `src/lib.rs` (`driver` body, `fmt::*` strings), `src/ctype.rs` (the glibc `<ctype.h>` macros), `src/ffi.rs` (libc bindings) | complete |

No stubs, no `unimplemented!()`, no `todo!()` anywhere in `src/`
(`grep -rn "unimplemented!\|todo!\|panic!" src/` → no matches).

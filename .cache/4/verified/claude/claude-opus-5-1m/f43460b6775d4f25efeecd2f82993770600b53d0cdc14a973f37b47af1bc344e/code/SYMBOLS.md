# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## How the two shared objects are produced

`c_src/CMakeLists.txt` only declares `add_executable(driver src/main.c src/lib.c)`,
so cmake alone produces no `.so`. The reference **library** translation unit is
`c_src/src/lib.c`; it is compiled into a shared object by `./build_c_so.sh`
using the same (empty) optimisation settings cmake uses when no
`CMAKE_BUILD_TYPE` is set, so the `.so` behaves exactly like the cmake-built
`driver` binary. Nothing under `c_src/` is modified.

| artifact | path | produced by |
|---|---|---|
| C reference executable | `c_src/build/driver` | `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .` |
| C reference library | `c_build/libcdecisions.so` | `./build_c_so.sh` (`gcc -shared -fPIC c_src/src/lib.c`) |
| Rust library | `target/debug/libdriver.so` | `cargo build` (`crate-type = ["cdylib", "rlib"]`) |
| Rust executable | `target/debug/driver` | `cargo build` |

## Defined (exported) symbols

`nm -D --defined-only c_build/libcdecisions.so`:

```
00000000000010f9 T process_decisions
```

`nm -D --defined-only target/debug/libdriver.so`:

```
0000000000012b10 T process_decisions
```

| # | C symbol | type | present in Rust `.so`? | notes |
|---|----------|------|------------------------|-------|
| 1 | `process_decisions` | `T` (global text) | **yes** | `#[no_mangle] pub unsafe extern "C" fn` in `src/lib.rs` |

**Symbol diff (C exports minus Rust exports): EMPTY.** ✔

### Why there is only one symbol

Every other function in `c_src/src/lib.c` is declared `static`, so it has
internal linkage and is deliberately *not* part of the ABI:

| C function | linkage | Rust counterpart |
|---|---|---|
| `process_decisions` | external | `driver::process_decisions` (`#[no_mangle]`) + `driver::decisions::process_decisions` |
| `parse_bool` | `static` | `decisions::parse_bool` (private) |
| `apply_permissions` | `static` | `decisions::apply_permissions` (private) |
| `evaluate_conditions` | `static` | `decisions::evaluate_conditions` (private) |
| `configure_flags` | `static` | `decisions::configure_flags` (private) |
| `validate_sequence` | `static` | `decisions::validate_sequence` (private) |
| `main` (in `main.c`) | external, but only in the *executable* | `src/main.rs` `fn main` |

No C source file was skipped by the translation: `c_src/src/` contains exactly
`lib.c` (→ `src/decisions.rs` + `src/lib.rs`) and `main.c` (→ `src/main.rs`).
No symbol is stubbed or `unimplemented!()`.

## Undefined symbols

The C `.so` imports nothing but the usual glibc/ELF weak hooks:

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

The Rust `.so` additionally imports libc (`malloc`, `memcpy`, `write`, …) and
the `_Unwind_*` family from the Rust standard library's panic runtime. **Zero
undefined non-libc / non-runtime symbols**, i.e. the library is fully linkable
and `dlopen`-able. ✔

## ABI of the exported symbol

```c
int process_decisions(char *decision_string, size_t length, int operation, int param);
```

```rust
#[no_mangle]
pub unsafe extern "C" fn process_decisions(
    decision_string: *mut core::ffi::c_char,
    length: usize,
    operation: core::ffi::c_int,
    param: core::ffi::c_int,
) -> core::ffi::c_int;
```

Note that `decision_string` is a `*mut` (not `*const`): operation 3
(`validate_sequence`) aliases the caller's buffer through a `bool *` and
**rewrites every byte in `[0, length)` in place**. That mutation is observable
by the caller across the FFI boundary, so it is part of the contract and is
reproduced byte-for-byte by the Rust translation (see `tests/differential.rs`,
`op3_buffer_rewrite_matches`).

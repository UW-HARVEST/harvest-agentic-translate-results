# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

Artifacts compared:

* C   : `c_src/build/libtranslated_rust.so`
  (`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust: `target/debug/libmemchra2_lib.so` (`cargo build`, `crate-type = ["cdylib"]`)

Regenerate / re-verify with `./check_symbols.sh`.

## C source inventory (`c_src/src/lib.c`)

| C function | linkage | exported from C `.so`? | Rust counterpart | exported from Rust `.so`? |
|---|---|---|---|---|
| `memchra`             | `static` | no (internal) | `memchra` (`unsafe fn`)             | no (matches C) |
| `process_buffer`      | `static` | no (internal) | `process_buffer` (`unsafe fn`)      | no (matches C) |
| `int_to_float_bits`   | `static` | no (internal) | `int_to_float_bits` (`unsafe fn`)   | no (matches C) |
| `process_strings`     | `static` | no (internal) | `process_strings` (`unsafe fn`)     | no (matches C) |
| `safe_sum_array`      | `static` | no (internal) | `safe_sum_array` (`unsafe fn`)      | no (matches C) |
| `interpret_as_int`    | `static` | no (internal) | `interpret_as_int` (`unsafe fn`)    | no (matches C) |
| `count_occurrences`   | `static` | no (internal) | `count_occurrences` (`unsafe fn`)   | no (matches C) |
| `complex_iteration`   | `static` | no (internal) | `complex_iteration` (`unsafe fn`)   | no (matches C) |
| `memchra2`            | extern   | **yes**       | `memchra2` (`#[unsafe(no_mangle)] pub extern "C"`) | **yes** |

`c_src/include/lib.h` declares exactly one entry point:

```c
int memchra2(int a, int b, int c, int d);
```

## `nm -D --defined-only` — C `.so`

```
00000000000013e1 T memchra2
```

1 global symbol.

## `nm -D --defined-only` — Rust `.so` (default config, no features)

```
0000000000013b40 T memchra2
```

1 global symbol (addresses are link-order dependent and are not compared).

## Symbol diff (C exports that the Rust `.so` does not export)

```
(empty)
```

**0 missing symbols. 0 undefined non-libc symbols in the Rust `.so`.**

`nm -D --undefined-only` on the Rust `.so` lists only libc/libgcc imports
(`memcpy`, `malloc`, `_Unwind_*`, `__errno_location`, …) — all satisfied by the
platform, none unresolved project symbols. The C `.so` likewise imports only
`snprintf`, `strlen`, `strncmp`.

Nothing needed translating for symbol parity: `c_src` contains a single
translation unit (`src/lib.c`) and every one of its nine functions has a real,
fully translated Rust counterpart (no stubs, no `unimplemented!()`).

## Optional test-only surface: feature `internal_test_api`

The eight `static` C helpers are not reachable across the `.so` boundary, so to
run Phase B/C differential tests against them the crate has an **off-by-default**
feature `internal_test_api` which adds `#[unsafe(no_mangle)] extern "C"`
wrappers:

| exported symbol (feature on) | wraps |
|---|---|
| `itest_memchra`           | `memchra` |
| `itest_process_buffer`    | `process_buffer` |
| `itest_int_to_float_bits` | `int_to_float_bits` |
| `itest_process_strings`   | `process_strings` |
| `itest_safe_sum_array`    | `safe_sum_array` |
| `itest_interpret_as_int`  | `interpret_as_int` |
| `itest_count_occurrences` | `count_occurrences` |
| `itest_complex_iteration` | `complex_iteration` |
| `itest_format_buffer`     | `snprintf_test_pattern` — the `snprintf("test%d-%d-%d-%d", …)` call site of `memchra2`, so the `%d` emulation can be diffed against glibc |

The C side of that comparison is `tests/cshim/shim.c`, which `#include`s
`c_src/src/lib.c` verbatim (c_src is never modified) and exports the same nine
`itest_*` names. With the feature **off** — the shipped configuration — the Rust
`.so` exports exactly `memchra2` and nothing else, i.e. byte-for-byte the C
`.so`'s public surface.

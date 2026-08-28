# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on the built libraries.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-nTzw79.so   (project name = parent dir name,
#                                             see cmake_path() in CMakeLists.txt)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libjumpnode_lib.so
```

## C `.so` dynamic symbols

`nm -D --defined-only c_src/build/libharvest-work-nTzw79.so`

| symbol | type | in Rust `.so`? |
|--------|------|----------------|
| `jumpnode` | `T` (global text) | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn jumpnode` |

**That is the entire exported surface: exactly one symbol.**

Undefined (imported) symbols in the C `.so`, for reference — these are libc/libm
and are NOT part of the surface the Rust must export:

| symbol | source |
|--------|--------|
| `sprintf@GLIBC_2.2.5` | libc — used by `jumpnode` case `0003` |
| `strlen@GLIBC_2.2.5` | libc — used by `compute_size_metric` |
| `sqrt` | libm — used by `jumpnode` case `0004` |
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__` | weak, toolchain-generated |

## C file-local (`static`) symbols — NOT exported, so NOT required in Rust

`nm` (static table) shows these as `t`/`b` (local). They are reachable only from
inside the translation unit. Each one still has a faithful Rust counterpart so
that the behaviour of `jumpnode` matches; none of them may be exported, because
the C does not export them.

| C local symbol | kind | Rust counterpart | exported? |
|----------------|------|------------------|-----------|
| `find_node_by_id` | `t` static fn | `find_node_by_id` (private) | no (correct) |
| `add_node` | `t` static fn | `add_node` (private) | no (correct) |
| `process_backward` | `t` static fn | `process_backward` (private) | no (correct) |
| `compute_size_metric` | `t` static fn | `compute_size_metric` (private) | no (correct) |
| `safe_double_to_int` | `t` static fn | `safe_double_to_int` (private) | no (correct) |
| `initialize_test_data` | `t` static fn | `initialize_test_data` (private) | no (correct) |
| `node_storage` | `b` static array | `NODE_STORAGE` (private `static mut`) | no (correct) |
| `node_count` | `b` static int | `NODE_COUNT` (private `static mut`) | no (correct) |

### Note on `initialize_test_data`

In the C source `initialize_test_data()` is `static` and **is never called** by
any reachable code path. Consequently, in the shipped `.so`:

* `node_count` is permanently `0`,
* `node_storage` is permanently all-zero,
* `find_node_by_id()` therefore *always* returns `NULL`,
* so `jumpnode` cases `0001`, `0002` and `0004` *always* take their
  early-return error branch.

This is faithfully reproduced by the Rust translation (the private
`initialize_test_data` exists and is likewise never called).

## Feature combinations

`Cargo.toml` declares one optional feature, giving two combinations:

| # | feature set | Rust exported symbols | matches C? |
|---|-------------|-----------------------|------------|
| 1 | *(default / none)* | `jumpnode` | YES — exact parity |
| 2 | `expose_init_test_data` | `jumpnode`, `jumpnode_initialize_test_data` | superset (see below) |

Combination 2 additionally exports `jumpnode_initialize_test_data`, a test-only
hook that invokes the otherwise-unreachable `initialize_test_data`. It is
**not** part of the C surface, and it is off by default, so the default build
has exact symbol parity with the C `.so`.

To differentially verify the *deep* code paths that `initialize_test_data`
unlocks (cases `0001`/`0002`/`0004` bodies, `add_node`, `process_backward`,
`safe_double_to_int`), the test suite compiles a **C shim** from the
*unmodified* `c_src/src/lib.c`:

```c
#include "<c_src>/src/lib.c"
void jumpnode_initialize_test_data(void) { initialize_test_data(); }
```

giving a C library with exactly the same two exports, so combination 2 is also
compared against real C rather than against assumptions. `c_src/` itself is
never modified.

## Result

* Symbols exported by C but missing from Rust: **0**
* Undefined non-libc symbols in the Rust `.so`: **0**

Verified by `tests/symbols.rs` (`symbol_parity_c_vs_rust`) and by
`check_all_features.sh`.

# SYMBOLS.md — Phase A: public symbol surface

Source of truth: `nm -D --defined-only` on the C shared library
`c_src/build/libharvest-work-X6YTtt.so`, compared against the Rust
`translation/target/release/libjumpnode_lib.so`.

## C `.so` exported (defined, dynamic) symbols

| # | symbol | type | present in Rust `.so`? | notes |
|---|--------|------|------------------------|-------|
| 1 | `jumpnode` | `T` (text, global) | YES (`#[unsafe(no_mangle)] pub unsafe extern "C" fn jumpnode`) | the only public entry point; declared in `c_src/include/lib.h` |

`nm -D --defined-only` output, verbatim:

```
C:    000000000000136b T jumpnode
Rust: 0000000000011b40 T jumpnode
```

**Symbol diff (C exported \ Rust exported): EMPTY.**

## Static (file-local) C functions — deliberately NOT exported

These are `static` in `c_src/src/lib.c` and therefore absent from the C `.so`'s
dynamic symbol table. The Rust translation keeps them private too, so the
default-feature Rust `.so` exports exactly the same set as the C `.so`.

| C symbol | Rust counterpart | exported? |
|----------|------------------|-----------|
| `find_node_by_id` | `find_node_by_id` | no (private, matches C) |
| `add_node` | `add_node` | no (private, matches C) |
| `process_backward` | `process_backward` | no (private, matches C) |
| `compute_size_metric` | `compute_size_metric` | no (private, matches C) |
| `safe_double_to_int` | `safe_double_to_int` | no (private, matches C) |
| `initialize_test_data` | `initialize_test_data` | no (private, matches C) |
| `node_storage` (static array) | `NODE_STORAGE` | no (private, matches C) |
| `node_count` (static int) | `NODE_COUNT` | no (private, matches C) |

No C source file was left untranslated: `c_src/src/lib.c` is the whole library
and every one of its 6 functions + 2 file-static objects has a Rust counterpart.

## Undefined (imported) symbols

The Rust `.so` imports only libc / libgcc-unwind symbols (`memcpy`, `malloc`,
`abort`, `_Unwind_*`, …) — 0 missing/undefined non-libc symbols.
The C `.so` imports `sprintf`, `sqrt`, `strlen`; the Rust translation
reimplements `sprintf`/`strlen` internally (`c_sprintf_node_depth`,
`c_strlen`) and uses `f64::sqrt`, which is a legitimate implementation
difference and is verified behaviourally in Phase B.

## Feature-gated extra symbols (non-default)

All of `c_src/src/lib.c`'s helpers are `static`, and the only caller of
`add_node` / `initialize_test_data` is itself dead code, so none of them are
reachable through the shipped `.so`. To verify them differentially anyway, the
crate exposes matching test hooks behind the non-default
`expose_init_test_data` feature, and `tests/c_harness/harness.c` exposes hooks
with **identical names** over the original C by `#include`ing
`c_src/src/lib.c` verbatim. `c_src/` is never modified.

| symbol | C helper it drives |
|--------|--------------------|
| `jumpnode_initialize_test_data` | `initialize_test_data` |
| `jumpnode_test_add_node` | `add_node` (incl. the `>= MAX_NODES` rejection) |
| `jumpnode_test_set_node_count` | writes `node_count` |
| `jumpnode_test_get_node_count` | reads `node_count` |
| `jumpnode_test_find_node_index` | `find_node_by_id` (as an index; `-1` for NULL) |
| `jumpnode_test_compute_size_metric` | `compute_size_metric` |
| `jumpnode_test_safe_double_to_int` | `safe_double_to_int` |
| `jumpnode_test_process_backward` | `process_backward` |
| `jumpnode_test_get_node` | reads `node_storage[i]` |
| `jumpnode_test_sizeof_node` | `sizeof(Node)` (layout parity check) |
| `jumpnode_test_max_nodes` | `MAX_NODES` |

None are in the C `.so`, and none are compiled into the default build.

Verified by `verify.sh` under all 4 configurations (2 feature combos x
release/debug):

* `--no-default-features` → Rust `.so` exports exactly `{jumpnode}`, i.e.
  **symbol-identical to the C `.so`**.
* `--features expose_init_test_data` → exports `jumpnode` plus the 11 hooks
  above.

In every configuration `comm -23 <C syms> <Rust syms>` is **empty**.

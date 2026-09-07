# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from:

```
C_SO=c_src/build/libharvest-work-4Q7mHQ.so
R_SO=translation/target/release/libmaxnmin_lib.so
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only "$R_SO" | awk '{print $3}' | grep -v '^_ZN' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms      # symbols in C but NOT in Rust  -> MUST be empty
```

The C library is built from a single translation unit (`c_src/src/lib.c`); there
is no second module, so there is no "whole file never translated" gap. All seven
C `extern` functions have real Rust implementations (no stubs, no
`unimplemented!()`).

## Symbol table

| # | C symbol | C signature | exported by Rust `.so`? | Rust item |
|---|----------|-------------|-------------------------|-----------|
| 1 | `add_node` | `int add_node(int id, int parent_id, const char *name, double value)` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn add_node` |
| 2 | `find_node_by_id` | `Node *find_node_by_id(int id)` | yes | `#[unsafe(no_mangle)] pub extern "C" fn find_node_by_id` |
| 3 | `get_children_count` | `int get_children_count(int parent_id)` | yes | `#[unsafe(no_mangle)] pub extern "C" fn get_children_count` |
| 4 | `calculate_subtree_sum` | `double calculate_subtree_sum(int node_id)` | yes | `#[unsafe(no_mangle)] pub extern "C" fn calculate_subtree_sum` |
| 5 | `process_string` | `int process_string(char *str)` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn process_string` |
| 6 | `safe_double_to_int` | `int safe_double_to_int(double d)` | yes | `#[unsafe(no_mangle)] pub extern "C" fn safe_double_to_int` |
| 7 | `maxnmin` | `int maxnmin(int a, int b, int c, int d)` (the only symbol in `include/lib.h`) | yes | `#[unsafe(no_mangle)] pub extern "C" fn maxnmin` |

Note: only `maxnmin` appears in the public header, but the other six are
non-`static` in `lib.c` and therefore part of the `.so` ABI surface. They are
treated as first-class public entry points and tested directly (see
`CONFIGS.md`), not just through the `maxnmin` wrapper.

## Non-exported C state (intentionally not symbols)

| C declaration | Rust counterpart | exported? |
|---|---|---|
| `static Node node_storage[MAX_NODES]` | `static mut NODE_STORAGE: [Node; 100]` | no (correct — `static` in C) |
| `static int node_count` | `static mut NODE_COUNT: c_int` | no (correct — `static` in C) |

Both sides keep this as library-global state that persists across calls, so the
differential tests must drive both `.so`s with the *same call sequence* from the
*same fresh state*. The tests achieve a pristine `node_count == 0` by `dlopen`ing
a per-test private copy of each `.so` (see `tests/common/mod.rs`).

## Undefined-symbol check

`nm -D -u` on the Rust `.so` lists only libc/`libgcc` unwinder imports
(`memcpy`, `malloc`, `_Unwind_*`, …). There are **no undefined non-libc
symbols**, i.e. nothing from the C library is left dangling.

## Result

```
comm -23 /tmp/c.syms /tmp/r.syms   ->   (empty)
```

**0 missing symbols. 0 undefined non-libc symbols. Symbol parity: PASS.**
See `tests/symbol_parity.rs`, which re-derives this diff at test time and fails
if it is ever non-empty.

## Phase D re-verification (automated)

`scripts/verify_all.sh` re-derives the diff on every run and across every feature
combination. Latest output:

```
=== Configuration: default -- symbol diff (nm -D) ===
C exports:    7
Rust exports: 7
symbol diff: EMPTY (all C symbols present in Rust)
```

Feature combinations enumerated from `Cargo.toml`: **0 optional features, so 1
configuration (the default)**. The script builds the power set automatically if
features are ever added, so the loop does not need editing.

No binary/driver target exists on either side (`c_src/CMakeLists.txt` has no
`add_executable`; `Cargo.toml` has no `[[bin]]` and there is no `src/bin/`), so
the "compare C and Rust stdout" requirement is not applicable. The script asserts
this stays true and fails if a binary target ever appears.

## No translation gaps

`c_src` is a single translation unit (`src/lib.c`, 176 lines) plus a one-line
header. Every function in it has a real Rust body — there are no stubs, no
`unimplemented!()`, no `todo!()`:

```
$ grep -c 'unimplemented!\|todo!\|unreachable!' translation/src/lib.rs
0
```

so no symbol "appears" in `nm -D` while lying about its behaviour.

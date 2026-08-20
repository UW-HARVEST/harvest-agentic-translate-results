# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared libraries.

* C   `.so`: `c_src/build/libtranslated_rust.so` (cmake + gcc 11.5.0, no `CMAKE_BUILD_TYPE` ⇒ `-O0`)
* Rust `.so`: `target/release/libinreftree_lib.so` (`cargo build --release`)

Reproduce with:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only target/release/libinreftree_lib.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms          # must be EMPTY
```

## Build-time configuration surface

| axis | values | source of truth |
|------|--------|-----------------|
| Cargo features | **none declared** — `Cargo.toml` has no `[features]` table, so the powerset of feature combinations is exactly `{ {} }` | `Cargo.toml` |
| CMake options | **none** — no `option()`, no `target_compile_definitions`, no `if()` in `CMakeLists.txt` | `c_src/CMakeLists.txt` |
| C preprocessor conditionals | **none** — no `#if/#ifdef/#ifndef/#else/#elif` anywhere in `c_src/` | `c_src/src/lib.c`, `c_src/include/lib.h` |

⇒ There is exactly **one** valid build configuration. `cargo check --no-default-features --features ''`
and `cargo check` (default) are therefore the complete enumeration; both were run and both succeed
(see `run_verification.sh`, which regenerates the powerset from `Cargo.toml` mechanically rather than
hard-coding it).

## Defined (exported) symbols

`T` = text/function, `B` = BSS/data object.

| # | symbol | kind | C `.so` | Rust `.so` | size (C) | size (Rust) | status |
|---|--------|------|---------|------------|----------|-------------|--------|
| 1  | `add_op`             | T | ✓ | ✓ | — | — | OK |
| 2  | `multiply_op`        | T | ✓ | ✓ | — | — | OK |
| 3  | `subtract_op`        | T | ✓ | ✓ | — | — | OK |
| 4  | `divide_op`          | T | ✓ | ✓ | — | — | OK |
| 5  | `modulo_op`          | T | ✓ | ✓ | — | — | OK |
| 6  | `find_node_by_id`    | T | ✓ | ✓ | — | — | OK |
| 7  | `add_tree_node`      | T | ✓ | ✓ | — | — | OK |
| 8  | `calculate_tree_sum` | T | ✓ | ✓ | — | — | OK |
| 9  | `parse_operation`    | T | ✓ | ✓ | — | — | OK |
| 10 | `get_operation_func` | T | ✓ | ✓ | — | — | OK |
| 11 | `inreftree`          | T | ✓ | ✓ | — | — | OK |
| 12 | `node_count`         | B | ✓ | ✓ | 4      | 4      | OK — `int` |
| 13 | `node_table`         | B | ✓ | ✓ | 0xa28 = 2600 | 0xa28 = 2600 | OK — `TreeNode[50]`, `sizeof(TreeNode) == 52` |

**Symbol diff (`comm -23 c.syms r.syms`): EMPTY.** No symbol exported by the C `.so` is missing from
the Rust `.so`, so no C source file was left untranslated and no `#[no_mangle]` wrapper is missing.
`c_src/src/lib.c` is the only C translation unit, and all 11 functions + 2 globals it defines are
present in `src/lib.rs`.

`inreftree` is the only symbol declared in the public header `c_src/include/lib.h`; the other 10
functions and both globals have external linkage (no `static`) and are therefore part of the ABI
surface as well, which is why they are all tested directly through `dlsym`.

## Undefined (imported) symbols

The C `.so` imports `strchr@GLIBC_2.2.5` and `strncpy@GLIBC_2.2.5`. The Rust translation reimplements
both as private helpers (`c_strchr`, `c_strncpy`), so those imports are absent from the Rust `.so` —
this is expected and not a parity failure (the requirement is on *exported* symbols; the Rust `.so`
has **0 undefined non-libc symbols**: every `U`/`w` entry resolves to glibc / libgcc unwinder /
weak ELF hooks, all satisfied at load time, which is proven by the fact that `dlopen` succeeds in
every test).

Verified: `ldd -r target/release/libinreftree_lib.so` reports no unresolved symbols.

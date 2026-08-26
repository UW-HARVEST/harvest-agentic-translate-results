# SYMBOLS.md - dynamic symbol parity

Artifacts compared:

* C: `cbuild/libdriver_c.so`, built from **all three** C translation units with
  `gcc -shared -fPIC -O0 -g -Ic_src/include c_src/src/hashmap.c c_src/src/tree.c c_src/src/main.c`
  (`c_src/CMakeLists.txt` links the same three units into an executable; the
  shared-object build keeps every non-`static` symbol of the package, including
  the ones from `main.c`).
* Rust: `target/debug/libdriver.so` (`crate-type = ["cdylib"]`).

Command used for both: `nm -D --defined-only <so> | awk '{print $NF}' | sort`

Result: **35 symbols exported by the C `.so`, 35/35 exported by the Rust `.so`,
0 missing, 0 extra.** (`comm -23` and `comm -13` of the two sorted lists are both
empty - see `tests/phase_d_symbols.rs`, which re-checks this mechanically.)

Every `static` C function (`hash_function`, `should_resize`, `hashmap_resize`,
`tree_free_node`, `tree_remove_subtree`, `tree_print_helper`) is correctly
*absent* from both symbol tables; they are translated as private Rust `fn`s.

| # | symbol | C definition | Rust definition | exported by Rust `.so` |
|---|--------|--------------|-----------------|------------------------|
| 1 | `hashmap_clear` | `c_src/src/hashmap.c` | `src/hashmap.rs` | yes |
| 2 | `hashmap_contains` | `c_src/src/hashmap.c` | `src/hashmap.rs` | yes |
| 3 | `hashmap_create` | `c_src/src/hashmap.c` | `src/hashmap.rs` | yes |
| 4 | `hashmap_destroy` | `c_src/src/hashmap.c` | `src/hashmap.rs` | yes |
| 5 | `hashmap_get` | `c_src/src/hashmap.c` | `src/hashmap.rs` | yes |
| 6 | `hashmap_put` | `c_src/src/hashmap.c` | `src/hashmap.rs` | yes |
| 7 | `hashmap_remove` | `c_src/src/hashmap.c` | `src/hashmap.rs` | yes |
| 8 | `hashmap_size` | `c_src/src/hashmap.c` | `src/hashmap.rs` | yes |
| 9 | `main` | `c_src/src/main.c` | `src/lib.rs` | yes |
| 10 | `test_hashmap_basic` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 11 | `test_hashmap_collisions` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 12 | `test_tree_add_children` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 13 | `test_tree_add_root` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 14 | `test_tree_complex_structure` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 15 | `test_tree_count_descendants` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 16 | `test_tree_creation` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 17 | `test_tree_deep_hierarchy` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 18 | `test_tree_duplicate_id` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 19 | `test_tree_find_path` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 20 | `test_tree_max_children` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 21 | `test_tree_remove_leaf` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 22 | `test_tree_remove_root` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 23 | `test_tree_remove_subtree` | `c_src/src/main.c` | `src/driver.rs` | yes |
| 24 | `tree_add_node` | `c_src/src/tree.c` | `src/tree.rs` | yes |
| 25 | `tree_contains` | `c_src/src/tree.c` | `src/tree.rs` | yes |
| 26 | `tree_count_descendants` | `c_src/src/tree.c` | `src/tree.rs` | yes |
| 27 | `tree_create` | `c_src/src/tree.c` | `src/tree.rs` | yes |
| 28 | `tree_delete` | `c_src/src/tree.c` | `src/tree.rs` | yes |
| 29 | `tree_find_path` | `c_src/src/tree.c` | `src/tree.rs` | yes |
| 30 | `tree_get_depth` | `c_src/src/tree.c` | `src/tree.rs` | yes |
| 31 | `tree_get_height` | `c_src/src/tree.c` | `src/tree.rs` | yes |
| 32 | `tree_get_node` | `c_src/src/tree.c` | `src/tree.rs` | yes |
| 33 | `tree_print` | `c_src/src/tree.c` | `src/tree.rs` | yes |
| 34 | `tree_remove_node` | `c_src/src/tree.c` | `src/tree.rs` | yes |
| 35 | `tree_size` | `c_src/src/tree.c` | `src/tree.rs` | yes |

## Undefined symbols in the Rust `.so`

`nm -D -u target/debug/libdriver.so` lists only C runtime / unwinder imports
(`malloc`, `calloc`, `free`, `fwrite`, `fflush`, `abort`, `memcpy`, `stdout`,
`stderr`, `_Unwind_*`, `__libc_start_main`, ... ) - i.e. **0 missing/undefined
non-libc symbols**.

## Whole-program equivalence

`c_src/build/driver` (cmake, default configuration) and `target/debug/driver`
produce byte-identical `stdout` (1499 bytes) and `stderr` (72 bytes) and both
exit with status 0.

## How the differential suite is run

```sh
# C shared object + C executable (built automatically by the tests as well)
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
gcc -shared -fPIC -O0 -g -Ic_src/include \
    -o cbuild/libdriver_c.so c_src/src/hashmap.c c_src/src/tree.c c_src/src/main.c

# Rust cdylib + the three differential suites (only one feature combination
# exists — the crate declares no [features])
cargo build --offline
cargo test  --offline --no-default-features     # == default
cargo test  --offline --release
```

`tests/common/mod.rs::rust_so_path()` refuses to use a `libdriver.so` that is
older than `src/*.rs` (and rebuilds it), because `cargo test` does not always
re-emit a `cdylib` that no test target links against — a stale `.so` would make
every comparison silently pass.

## Test-suite sensitivity (mutation check)

Each of these deliberate single-line defects was injected into the Rust
translation and the suite failed every time (then the defect was reverted):

| mutation | detected by |
|----------|-------------|
| FNV prime `1099511628211` → `1099511628213` | Phase B C2–C15, C17–C36 |
| `load > 0.75` → `load >= 0.75` in `should_resize` | Phase B C3, C4, C5, … (9 rows) |
| `deleted_count += 1` dropped in `hashmap_remove` | Phase B C6, C9, … |
| `max_length` clamped to `>= 0` in `tree_find_path` | Phase C E45, B3 |
| `strncpy` length `MAX_DATA_LENGTH-1` → `-2` | Phase C B4+B5+B6 |
| root's `parent_id = 0` assignment removed | Phase B C17, C33 |
| `tree_get_height` returns `max_height` instead of `+1` | Phase B C18–C20, … |
| `printf("[%lu] ")` → `printf("[%lu]")` | Phase B C32 (and C34/C35) |
| `#[no_mangle]` removed from `hashmap_clear` | Phase D D1-missing |
| `wrapping_sub` → `-=` on `deleted_count` / `node_count` | Phase C E48, E49 |

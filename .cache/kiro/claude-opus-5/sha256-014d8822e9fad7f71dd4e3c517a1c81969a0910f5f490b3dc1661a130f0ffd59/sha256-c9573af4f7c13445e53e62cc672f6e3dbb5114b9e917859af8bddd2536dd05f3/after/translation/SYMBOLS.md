# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D` on both shared objects.

## C `.so`

Build: `cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`
Artifact: `c_src/build/libharvest-work-CrZxKw.so` (name derives from the parent
directory name via `cmake_path(... FILENAME project_name)` in `CMakeLists.txt`).

```
$ nm -D --defined-only c_src/build/libharvest-work-CrZxKw.so
000000000000136b T jumpnode
```

## Rust `.so`

Build: `cd translation && cargo build --release`
Artifact: `translation/target/release/libjumpnode_lib.so` (`crate-type = ["cdylib"]`,
`name = "jumpnode_lib"`).

```
$ nm -D --defined-only translation/target/release/libjumpnode_lib.so
0000000000011a30 T jumpnode
```

## Parity table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `jumpnode` | `T` (global text) | `T` (global text) | MATCH |

**Symbol diff (C minus Rust): EMPTY.** 0 missing, 0 undefined non-libc symbols.

## Internal-linkage C functions (correctly NOT exported by either side)

`c_src/src/lib.c` declares these `static`, so they have internal linkage and are
absent from `nm -D` on the C `.so`. The Rust translation keeps each as a private
item, so it likewise exports none of them. Each one *is* translated (nothing is
stubbed):

| C symbol (static) | Rust counterpart | reachable from `jumpnode`? |
|-------------------|------------------|-----------------------------|
| `find_node_by_id` | `find_node_by_id` | yes (modes `0001`, `0002`, `0004`) |
| `add_node` | `add_node` | no — only caller is `initialize_test_data` |
| `process_backward` | `process_backward` | mode `0002` only, after a non-null node lookup |
| `compute_size_metric` | `compute_size_metric` | yes (mode `0003`) |
| `safe_double_to_int` | `safe_double_to_int` | modes `0001`, `0004` |
| `initialize_test_data` | `initialize_test_data` | **no — dead code in the C original** |
| `strlen` (libc) | `c_strlen` | yes (via `compute_size_metric`) |
| `sprintf` (libc) | `c_sprintf_node_depth` + `write_bytes_str` / `write_bytes_int` | yes (mode `0003`) |

### Behaviourally critical consequence

`initialize_test_data()` has **no callers anywhere in the translation unit**. It
is not exported, and `jumpnode` never invokes it. Therefore `node_count` is `0`
for the entire lifetime of the process, `find_node_by_id` can never match, and
modes `0001`, `0002` and `0004` *always* take their null-node early-return
branch. The Rust translation reproduces this exactly (it also never calls
`initialize_test_data`). This is deliberate fidelity to the C, not an oversight:
"fixing" it by calling the initializer would break byte-identical behaviour.

## Undefined-symbol check

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports pulled in by Rust `std` (`malloc`, `memcpy`, `strlen`, `_Unwind_*`,
`__cxa_finalize`, pthread TLS helpers, etc.). **0 undefined non-libc symbols.**

```
$ nm -D --undefined-only translation/target/ffi-cdylib-release/release/libjumpnode_lib.so \
    | awk '{print $2}' | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$'
(empty)
```

## Verified command

```
$ nm -D --defined-only c_src/build/*.so       | awk '$2~/^[TDB]$/{print $3}' | sort > c.syms
$ nm -D --defined-only .../libjumpnode_lib.so | awk '$2~/^[TDB]$/{print $3}' | sort > r.syms
$ comm -23 c.syms r.syms | wc -l
0
```

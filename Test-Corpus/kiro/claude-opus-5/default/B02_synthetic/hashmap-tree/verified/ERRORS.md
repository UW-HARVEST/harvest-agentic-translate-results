# Differential verification of the C → Rust translation

The C program in `c_src/` is the ground truth. This document records every
mismatch found while diffing the two executables, what caused it, and what was
changed. It also lists the branches that no input can reach and why.

## What is being compared, and how

`c_src/src/main.c` is `int main(void)`. It reads nothing from stdin, ignores
`argv`, and runs a fixed sequence of self-checks. Its entire observable surface
is therefore: the bytes on stdout, the bytes on stderr, and the exit status.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> target/release/driver
```

Tests (`cd translation && cargo test`) run both *binaries* as subprocesses and
compare stdout, stderr and exit status for every case. Nothing loads the Rust
crate as a library.

| Test target | What it covers |
| --- | --- |
| `tests/driver_differential.rs` | The program as shipped: bare run, argv variations, stdin variations, merged `2>&1` streams, output to a file, a broken stdout pipe, an unwritable stdout, environment changes, run-to-run determinism. |
| `tests/library_branches.rs` | 21 scenarios driving the `tree.c` / `hashmap.c` branches `main.c` never calls, via a second pair of probe binaries. |

The Phase C probes are `tests/cprobe/probe.c` (compiled against the
**unmodified** `c_src/src/*.c`) and `src/bin/probe.rs`. They implement the same
scenario table and are selected by `argv[1]`, one scenario per process. Nothing
in `c_src/` is modified; the only addition there is the `build/` directory the
documented build command creates.

## Mismatches found

### 1. Exit status on a broken stdout pipe — translation bug, fixed

**Symptom.** With the reader end of stdout closed:

```
$ c_src/build/driver | true          ; echo ${PIPESTATUS[0]}   ->  141
$ translation/.../driver | true      ; echo ${PIPESTATUS[0]}   ->  0
```

stdout and stderr were identical (both empty), so a bytes-only comparison passed
while the exit status was wrong.

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs, so a failing write returns `EPIPE`, which `cout.rs` deliberately ignores
(mirroring the C code ignoring `printf`'s return value). A C program keeps the
default disposition and is killed by signal 13, which a shell reports as
`128 + 13 = 141`.

**Fix.** `src/cout.rs` now exposes `init_c_runtime()`, which restores
`SIGPIPE` to `SIG_DFL` via `signal(2)`; `main()` calls it first. Both programs
now die from signal 13.

Covered by `broken_stdout_pipe_kills_both_the_same_way`.

### 2. `hash_function` byte order — latent, tightened

`hashmap.c` hashes the key by aliasing `uint64_t` as `uint8_t *`, i.e. in the
machine's **native** byte order. The translation used `to_le_bytes()`, which is
correct on x86-64 and aarch64 but would silently reorder the FNV-1a input on a
big-endian target and change every probe sequence. Changed to `to_ne_bytes()`,
which is what the C actually does. No behavioural change on the tested platform.

### 3. Probe scaffolding: unspecified argument evaluation order — test bug

The `tombstones`, `max_children` and `stress_tree` scenarios initially
mismatched, for example:

```
C:    put 0 -> 0 (size=5 deleted=5)
Rust: put 0 -> 0 (size=6 deleted=4)
```

**Cause.** My probe's `printf` mixed a mutating call with reads of the mutated
state in a single call:

```c
printf("put %lu -> %d (size=%zu deleted=%zu)\n", k,
       hashmap_put(map, k, v), hashmap_size(map), map->deleted_count);
```

C leaves the order of argument evaluation unspecified; gcc evaluated
right-to-left, so the sizes were read *before* the `put`. Rust evaluates
left-to-right. This was a defect in the comparison harness, not in the
translation — the library itself never depends on evaluation order.

**Fix.** Both probes now compute each value into a local in an explicit order.
No production code changed.

### 4. Two build-configuration traps (no code change, guarded by a test)

*`NDEBUG` disables the entire program.* `main.c` performs every mutation
*inside* `assert()`: `assert(tree_add_node(tree, 1, 0, "root") == 0)`. Building
with `cmake -DCMAKE_BUILD_TYPE=Release` defines `NDEBUG`, deletes all of those
calls, and the program then prints `(empty tree)` and writes nothing to stderr.
The translation keeps the side effects, matching the plain
`cmake .. && cmake --build .` build the task specifies.
`c_reference_was_built_with_assertions_enabled` fails loudly if the reference
binary was built the other way, so a mis-built reference is never mistaken for a
translation bug.

*Rust profile parity.* `cargo test` builds the binaries with the dev profile,
which defaults to trapping integer overflow — behaviour C does not have.
`Cargo.toml` now sets `overflow-checks = false` and `debug-assertions = false`
for `[profile.dev]` so `cargo test` measures the same program as
`cargo build --release`. Both profiles were verified against the C binary
independently.

## Branch inventory

Every conditional in `hashmap.c` and `tree.c` was walked. Reached and diffed
(scenario name in parentheses):

- `tree_print`: the `!has_root` early return (`empty_print`, `remove_missing`).
- `tree_add_node`: duplicate id, including a duplicate root and a duplicate of
  `UINT64_MAX` (`duplicate_ids`, `big_ids`); `data == NULL` (`null_data`);
  first-node-becomes-root with `parent_id` forced to 0 (`remove_root_then_add`);
  parent not found (`parent_missing`); `child_count >= MAX_CHILDREN`
  (`max_children`).
- `strncpy(node->data, data, MAX_DATA_LENGTH - 1)` plus the forced terminator, at
  lengths 0, 1, 254, 255, 256, 300 and 1024 (`data_lengths`).
- `tree_remove_node`: node not found on an empty and on a populated tree, and
  removing an id twice (`remove_missing`, `subtree_cascade`); the
  `id == root_id` path (`remove_root_then_add`, `id_zero`); the child-list shift
  from the first, middle and last position, including the stale trailing slot
  (`remove_child_positions`).
- `tree_remove_subtree`: recursion over descendants and the `node == NULL` early
  return (`subtree_cascade`, `deep_chain`).
- `tree_get_depth` / `tree_get_height` / `tree_count_descendants`: absent node,
  leaf, and multi-level recursion (`queries_missing`, `deep_chain`,
  `max_children`).
- `tree_find_path`: absent node; `max_length` of 0, negative, less than, equal to
  and greater than the true length (`path_bounds`); and a 1100-deep chain that
  exhausts the fixed `temp_path[1000]` scratch array without ever reaching the
  root (`deep_chain`).
- `hashmap_put`: empty slot, key-match update, and tombstone reuse — including
  the case where reuse stores the **same key twice**, because `hashmap_put`
  claims the first tombstone without checking the rest of the probe chain
  (`collision_probing`, `tombstones`). The C behaves that way, so the Rust does
  too.
- `hashmap_get` / `hashmap_remove`: hit, miss on an unoccupied slot, a hit found
  only by probing *past* a tombstone, and a double remove (`collision_probing`,
  `tombstones`).
- `should_resize` / `hashmap_resize`: growth from capacity 16 up through 512, and
  the rehash that drops accumulated tombstones (`resize_map`).
- `hashmap_clear`: flags reset while keys and values are left in place
  (`clear_map`).
- Node id `0` used as the root, which collides with the "no root" sentinel that
  `tree_get_depth`'s loop condition compares against (`id_zero`).
- Long pseudo-random operation mixes (identical LCG on both sides) that dump
  every hashmap slot and every live node: 4000 hashmap operations
  (`stress_map`) and 1500 tree operations producing 34 KB of stderr diagnostics
  (`stress_tree`).

Not reachable by any input, and therefore not diffed:

- Allocation-failure paths: `malloc`/`calloc` returning `NULL` in
  `hashmap_create`, `hashmap_resize`, `tree_create` and `tree_add_node`, and the
  `hashmap_put` failure branch in `tree_add_node`.
- `NULL`-receiver guards: `if (!map)` in every `hashmap_*` function and
  `if (!tree)` in every `tree_*` function. The Rust translation takes `&self` /
  `&mut self`, so these states cannot be constructed; there is no caller in
  `c_src` that passes `NULL` either.
- `hashmap_put`'s `return -1` after the probe loop, and the equivalent
  loop-exhausted returns in `hashmap_get` and `hashmap_remove`. The 0.75 load
  factor guarantees at least a quarter of the slots are unoccupied, so the loop
  always terminates early.
- `tree_remove_node`'s `parent == NULL` fallback and `tree_print_helper`'s
  `node == NULL` return: removal always cascades to descendants and always
  unlinks a removed non-root id from its parent, so no live node can reference a
  missing parent or child.
- `main.c`'s `TEST_FAIL` macro, which is defined but never expanded.

## Known, deliberate limitation

`src/cout.rs` buffers all of stdout and flushes once, emulating glibc's full
buffering of a non-tty stdout. glibc actually flushes in `st_blksize` chunks
(4096 bytes in practice), so a program writing more than one block to a stdout
that *shares a descriptor* with stderr would interleave differently from this
emulation. It cannot affect the program under test: `driver` writes 1499 bytes
to stdout, well under one block, and
`merged_streams_interleave_identically` confirms the `2>&1` byte stream matches
exactly. The Phase C probes, which do exceed one block, are compared on stdout
and stderr separately.

`%lu` in `tree.c` is used to print `tree_id_t` (`uint64_t`), which is only
correct where `unsigned long` is 64 bits. Verified on x86-64 Linux.

## Status

- Both programs build with no errors or warnings.
- 17 tests, 0 ignored, 0 skipped, 0 disabled: `cargo test` is green.
- All 21 probe scenarios plus every driver invocation class produce identical
  stdout, stderr and exit status, in both the dev and the release profile.
- `c_src/` is unmodified; only `c_src/build/` (build output) was added.

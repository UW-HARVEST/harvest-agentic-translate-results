# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C   `.so`: `c_src/build/libharvest-work-SE8kRb.so`
- Rust`.so`: `translation/target/release/libjumpnode_lib.so`

## C exported (non-libc, `T`/`D`/`B`) symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-SE8kRb.so | grep -v ' [a-z] '
000000000000136b T jumpnode
```

Exactly **one** public symbol. `c_src/include/lib.h` confirms the surface:

```c
int jumpnode(int a, int b, int c, int d);
```

## Rust exported symbols

```
$ nm -D --defined-only translation/target/release/libjumpnode_lib.so | grep -v '__'
0000000000011a30 T jumpnode
```

## Parity table

| # | C symbol | type | present in Rust `.so`? | how |
|---|----------|------|------------------------|-----|
| 1 | `jumpnode` | `T` (func) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn jumpnode` in `src/lib.rs` |

**Symbol diff: EMPTY.** No missing symbols, no stubs.

## Internal-linkage (`static`) C functions — NOT part of the ABI

These have internal linkage in C and therefore must NOT appear in `nm -D`.
They are translated as private Rust `fn`s so `jumpnode`'s observable
behaviour is identical. Verified absent from both `.so` dynamic tables.

| C static function | Rust counterpart | exported? (must be no) |
|---|---|---|
| `find_node_by_id` | `find_node_by_id` (private) | no |
| `add_node` | `add_node` (private) | no |
| `process_backward` | `process_backward` (private) | no |
| `compute_size_metric` | `compute_size_metric` (private) | no |
| `safe_double_to_int` | `safe_double_to_int` (private) | no |
| `initialize_test_data` | `initialize_test_data` (private) | no |

## Modules / files

`c_src/CMakeLists.txt` lists exactly one translation unit (`src/lib.c`).
`translation/src/lib.rs` is the sole Rust module. **No C source file was
skipped**, so no re-translation of a missing module is required.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no
`[[bin]]`/`src/main.rs`, so the only build configuration is the default
one. `--no-default-features` is equivalent to the default here (verified in
Phase D). The project builds **no binary executable**, so the
"compare binary stdout" clause of the completion gate is not applicable.

## Critical ABI/behaviour note carried into Phases B/C

`initialize_test_data()` is `static` in C and **has no callers anywhere in
the translation unit**. Therefore `node_count` is `0` for the entire
lifetime of the process and `find_node_by_id()` **always returns NULL**.
Consequences that the differential tests must confirm rather than assume:

- `operation_mode == 0o1` always returns `STATUS_ERROR | 0o20` = `18`
- `operation_mode == 0o2` always returns `STATUS_ERROR | 0o40` = `34`
- `operation_mode == 0o4` always returns `STATUS_ERROR | 0o100` = `66`
- `operation_mode == 0o3` is the only arm that performs real computation
- any other `operation_mode` returns `STATUS_ERROR | 0o200` = `130`

The Rust must reproduce this, including keeping `initialize_test_data`
uncalled.

## Verification (re-run after every change)

```
$ diff <(nm -D --defined-only c_src/build/lib*.so         | awk '$2 ~ /^[TDBR]$/ {print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libjumpnode_lib.so \
                                                          | awk '$2 ~ /^[TDBR]$/ {print $3}' | grep -v '^__' | sort)
# (no output — symbol diff EMPTY)
```

Automated in `tests/phase_d_symbols.rs`:

- `symbol_sets_are_identical` — set difference in BOTH directions must be empty
  (a missing C symbol fails, and so does an extra Rust symbol, which would mean
  an internal-linkage `static` leaked).
- `internal_statics_are_not_exported` — the six C `static`s, `node_storage`,
  `node_count`, and anything matching `probe*` must be absent from both objects.
  This is what guarantees the `#[cfg(test)]` probe module used by
  `tests/phase_b_internals.rs` never reaches a real build.
- `rust_so_has_no_unexpected_undefined_symbols` — no undefined non-libc symbols,
  so nothing is left unimplemented in a way that would only fail at call time.
- `both_objects_dlsym_jumpnode` — both objects resolve `jumpnode` through
  `dlsym` with the same ABI and agree on a known value.

`translation/verify_all.sh` re-runs all of the above for every feature
combination (extracted from `Cargo.toml`, not hard-coded) across both the `dev`
and `release` profiles.

### Staleness guard

`cargo test` does **not** rebuild the `cdylib` — no test target links it — so
the differential tests would otherwise `dlopen` a stale `.so` from an earlier
`cargo build` and pass against code no longer in `src/lib.rs`. This was a real
defect in the first version of the harness: removing `#[no_mangle]` produced
zero failures. `tests/common/mod.rs` now runs `cargo build --lib` itself before
loading and panics if the artifact's mtime predates `src/lib.rs`.

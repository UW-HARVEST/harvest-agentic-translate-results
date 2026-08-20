# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

## Build inputs

* C library: `c_src/CMakeLists.txt` builds **one** translation unit, `src/lib.c`,
  as `SHARED` → `c_src/build/libtranslated_rust.so`.
* Rust library: `Cargo.toml` `[lib] name = "jumpnode_lib"`, `crate-type = ["cdylib"]`
  → `target/{debug,release}/libjumpnode_lib.so`.
* Public header `c_src/include/lib.h` declares exactly one entry point:
  `int jumpnode(int a, int b, int c, int d);`

## Feature combinations

`c_src/CMakeLists.txt` defines **no `option()` / `add_definitions()` /
`target_compile_definitions()`**, and `c_src/src/lib.c` contains **no
`#ifdef`-guarded code**, so the C side has exactly one build configuration.

`Cargo.toml` originally had no `[features]` table. Verification added one
non-default, test-only feature (`shadow_probe`, see below), giving this complete
set of valid configurations:

| # | cargo invocation | `shadow_probe` | notes |
|---|------------------|----------------|-------|
| 1 | `cargo …` (default) | off | the shipping configuration; symbol set identical to the C `.so` |
| 2 | `cargo … --no-default-features` | off | identical to #1 (there is no default feature set) |
| 3 | `cargo … --all-features` | **on** | adds the `probe_*` test wrappers |
| 4 | `cargo … --no-default-features --features shadow_probe` | **on** | identical to #3 |

All four are checked, built and tested by `./verify_all_features.sh`
(Phase D). Result: **ALL CONFIGURATIONS PASSED**, symbol diff **EMPTY**.

### Why `shadow_probe` exists

`initialize_test_data()` is `static` and never called in `lib.c`, so `node_count`
is permanently `0`, `find_node_by_id()` always returns `NULL`, and `jumpnode`'s
modes `0001`/`0002`/`0004` always take their error return. Through the public API
alone, most of the algorithm — `add_node`, `process_backward`,
`safe_double_to_int`, `compute_size_metric`, the parent walk, the sqrt
accumulation and the backward scan — is unreachable and therefore
**unverifiable**.

`shadow_probe` exports thin `probe_*` wrappers around exactly those `static`
helpers. The C side gets a matching set from `shadow_c/lib_shadow.c`, which
`#include`s the **untouched** `c_src/src/lib.c` so the statics land in the same
translation unit (nothing in `c_src/` is copied or modified). `tests/deep_paths.rs`
then compares the low-level functions directly and drives `jumpnode` with
populated node storage.

The feature is **off by default**, and
`symbol_parity.rs::phase_d_default_build_symbol_set_is_exactly_the_c_set`
asserts the default build exports *exactly* `{jumpnode}` — the same set as the C
`.so`, no extras.

## Defined dynamic symbols

`nm -D --defined-only` on each `.so` (Rust internal `_ZN…` mangled symbols and
the standard ELF bookkeeping symbols are not part of the C surface and are
excluded from the comparison):

| symbol | in C `.so` | in Rust `.so` | status |
|--------|-----------|---------------|--------|
| `jumpnode` | `T` (yes) | `T` (yes) | ✅ present in both |

**Symbol diff (C-exported minus Rust-exported): EMPTY.**

In the default configuration the two sets are not merely a subset relation but
*equal*: both `.so`s export exactly `{jumpnode}`.

With `--features shadow_probe` the Rust `.so` additionally exports the 15
`probe_*` wrappers, matched one-for-one by `shadow_c/build/libshadow_c.so`:
`probe_add_node`, `probe_compute_size_metric`, `probe_find`, `probe_init`,
`probe_node_count`, `probe_node_data`, `probe_node_id`, `probe_node_parent_id`,
`probe_node_value`, `probe_process_backward`, `probe_reset`,
`probe_safe_double_to_int`, `probe_sizeof_node`, `probe_status`.

The C `.so` exports exactly one non-bookkeeping symbol. Every `static` function
in `lib.c` has internal linkage and is deliberately *not* exported by the C
`.so`, so the Rust translation must not export it either:

| C symbol | linkage in C | exported by C `.so`? | Rust counterpart | exported by Rust `.so`? |
|----------|--------------|----------------------|------------------|-------------------------|
| `jumpnode` | external | yes | `jumpnode` (`#[unsafe(no_mangle)] pub unsafe extern "C" fn`) | yes ✅ |
| `find_node_by_id` | `static` | no | `find_node_by_id` (private) | no ✅ |
| `add_node` | `static` | no | `add_node` (private) | no ✅ |
| `process_backward` | `static` | no | `process_backward` (private) | no ✅ |
| `compute_size_metric` | `static` | no | `compute_size_metric` (private) | no ✅ |
| `safe_double_to_int` | `static` | no | `safe_double_to_int` (private) | no ✅ |
| `initialize_test_data` | `static` | no | `initialize_test_data` (private) | no ✅ |
| `node_storage` | `static` object | no | `NODE_STORAGE` (private) | no ✅ |
| `node_count` | `static` object | no | `NODE_COUNT` (private) | no ✅ |

All six `static` functions and both `static` objects are translated in
`src/lib.rs` — nothing was skipped, so no Phase-A "translate the missing C
source" work is required.

## Undefined (imported) symbols

| library | non-libc undefined symbols |
|---------|----------------------------|
| C `.so` | none (`sprintf`, `strlen`, `sqrt` are libc/libm; `_ITM_*`, `__gmon_start__`, `__cxa_finalize` are weak ELF bookkeeping) |
| Rust `.so` | none (all `U` entries are glibc/`libgcc_s` unwinder symbols) |

> **Loading note.** `c_src/CMakeLists.txt` does *not* link `m`, so the C `.so`
> carries an unresolved `U sqrt`. The differential tests therefore `dlopen`
> `libm.so.6` with `RTLD_GLOBAL | RTLD_NOW` before opening the C `.so`
> (see `tests/common/mod.rs`). This is a property of the provided C build, not a
> translation defect, and `c_src/` was not modified.

## Completion checklist

* [x] `nm -D` shows **0** symbols missing from the Rust `.so`.
* [x] `nm -D` shows **0** undefined non-libc symbols in the Rust `.so`.
* [x] No `static`-linkage C symbol is accidentally exported by the Rust `.so`.
* [x] Holds for every feature combination (all four; `verify_all_features.sh`).

## How to run the verification

```bash
./verify_all_features.sh        # all 4 configurations, end to end
```

or any single configuration directly — the harness is self-sufficient and will
build `c_src`, `shadow_c` and the cdylib itself if needed:

```bash
cargo test                                                   # config 1
cargo test --no-default-features                             # config 2
cargo test --all-features                                    # config 3
cargo test --no-default-features --features shadow_probe      # config 4
```

### Pitfall this harness defends against

`cargo test` does **not** rebuild the `cdylib` artifact. Integration tests
cannot link a cdylib, so cargo builds the lib only as an rlib for them, and
`target/debug/libjumpnode_lib.so` is left over from the last `cargo build` —
possibly with a *different* feature set. A naive harness that just `dlopen`s
that path would silently verify the wrong configuration, or pass vacuously
against a stale library.

`tests/common/mod.rs` therefore:

1. builds the cdylib itself for the current feature set into a dedicated
   `--target-dir` (`target/xdiff-so-{default,probe}`), so the loaded `.so`
   always matches the test binary; and
2. asserts the loaded `.so` exports `probe_init` **iff** `shadow_probe` is
   enabled (`assert_so_matches_features`), turning any residual mismatch into a
   loud failure rather than a false pass.

### Harness validated by mutation testing

Passing tests only mean something if they can fail. Ten deliberate mutations
were injected into `src/lib.rs` and the suite re-run; see the mutation table in
`ERRORS.md`. All behaviour-changing mutations were caught (the only survivors
are provably *equivalent* mutants). `src/lib.rs` was restored and byte-compared
to the original afterwards.

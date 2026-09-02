# SYMBOLS.md — public symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-ZqncLi.so   | awk '$2 ~ /^[TtWwBbDdRr]$/ {print $3}' | sort -u
nm -D --defined-only translation/target/release/libmaxnmin_lib.so | awk '$2 ~ /^[TtWwBbDdRr]$/ {print $3}' | sort -u
```

The C library is built from exactly one translation unit (`c_src/src/lib.c`,
per `c_src/CMakeLists.txt`), so the whole C surface is that one file. No C
module is missing from the Rust translation.

## Symbol table

| # | C symbol | C type | exported by Rust `.so` | Rust definition |
|---|----------|--------|------------------------|-----------------|
| 1 | `add_node` | T (func) | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn add_node` |
| 2 | `find_node_by_id` | T (func) | yes | `#[unsafe(no_mangle)] pub extern "C" fn find_node_by_id` |
| 3 | `get_children_count` | T (func) | yes | `#[unsafe(no_mangle)] pub extern "C" fn get_children_count` |
| 4 | `calculate_subtree_sum` | T (func) | yes | `#[unsafe(no_mangle)] pub extern "C" fn calculate_subtree_sum` |
| 5 | `process_string` | T (func) | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn process_string` |
| 6 | `safe_double_to_int` | T (func) | yes | `#[unsafe(no_mangle)] pub extern "C" fn safe_double_to_int` |
| 7 | `maxnmin` | T (func) | yes | `#[unsafe(no_mangle)] pub extern "C" fn maxnmin` |

C defined symbols: 7. Rust defined symbols: 7.

`comm -23 c_syms r_syms` (present in C, missing from Rust) → **empty**.
`comm -13 c_syms r_syms` (extra in Rust) → **empty**.

## Non-exported C state (intentionally not symbols)

These are `static` in C, therefore not part of the ABI, and correctly have no
exported counterpart in Rust. They are nonetheless *observable* process-wide
state that the differential tests must keep in lock-step between the two
libraries (see `CONFIGS.md` rows 40–44).

| C declaration | Rust counterpart |
|---|---|
| `static Node node_storage[MAX_NODES];` | `static mut NODE_STORAGE: [Node; MAX_NODES]` |
| `static int node_count = 0;` | `static mut NODE_COUNT: c_int` |

## Undefined symbols in the Rust `.so`

All undefined imports are libc / `libgcc` unwinder / `ld.so` runtime symbols
(`malloc`, `memcpy`, `strlen`, `__cxa_finalize`, `_Unwind_*`, `dl_iterate_phdr`,
…). There are **0 missing or undefined non-libc symbols**.

## Verification status

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
- [x] Every C symbol exported by the Rust `.so` under the exact same name.

Automated by `translation/check_symbols.sh` (exit 0 == parity).

## Phase D completion gate

Enumerated mechanically: `Cargo.toml` declares **no** `[features]`
(`cargo metadata` → `features: {}`), so the complete configuration set is
`default` and `--no-default-features`, which are the same code. Both are run,
against both the `release` and the `debug` Rust `.so`, by
`translation/run_all_combos.sh` → **ALL COMBINATIONS PASSED**.

| gate | status |
|------|--------|
| `SYMBOLS.md`: 0 missing / 0 non-libc undefined symbols | 7/7 exported, verified per combination |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | 47/47 |
| Phase C: every `ERRORS.md` row has a passing error-path test | 39/39 |
| Holds under every feature combination × profile | 4/4 (`default`/`nodefault` × `release`/`debug`) |

Additional independent cross-check: `tests/zz_fuzz.rs` (`--ignored`) drove ~7.2M
randomized differential calls against the release `.so` and ~2.4M against the
debug `.so` across all seven entry points with 0 divergences.

One real divergence was found and fixed in the Rust: NaN propagation order in
`calculate_subtree_sum`'s accumulation (see the bottom of `ERRORS.md` and rows
45–47 of `CONFIGS.md`).

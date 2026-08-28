# Verification of the C → Rust translation

The Rust crate is verified **differentially**: every test loads BOTH the C `.so`
and the Rust `.so` with `libloading` and compares their results through the C ABI.
No Rust function is ever called directly, so the `#[no_mangle]`/`extern "C"`
export wrappers are under test too.

## Layout

| file | purpose |
|------|---------|
| `SYMBOLS.md` | Phase A — every `nm -D` symbol of the C `.so`, and its counterpart in the Rust `.so` |
| `ERRORS.md` | Phase A — error-surface table (E1–E10, G1–G10, L1–L3) |
| `CONFIGS.md` | Phase A — configuration-surface table (C1–C29) |
| `tests/common/mod.rs` | harness: dlopens both `.so`s, wraps all 8 exported symbols, seeded xorshift64\* RNG, process-wide lock for the mutable global |
| `tests/phase_b_valid.rs` | Phase B — valid-path rows C1–C20 |
| `tests/phase_b_pristine.rs` | Phase B — rows C21–C22, in their own process so the *as-linked* `matrix` initializer is observable |
| `tests/phase_bc_edge.rs` | rows C23–C28 / G8–G10 — size_t wrap-arounds, unprototyped-ABI call, 10 000-element growth chain, 20 000-step whole-API fuzz |
| `tests/phase_c_errors.rs` | Phase C — error rows E1–E10 and boundary rows G1–G7 |
| `tests/phase_c_leak.rs` | Phase C — rows L1–L3, allocator accounting via glibc `mallinfo2()` |
| `tests/phase_c_crash.rs` | Phase C — rows Z1–Z5, crash modes and side-effect ordering, compared in child processes |
| `tests/phase_d_symbols.rs` | Phase D — symbol parity (`nm -D` diff), symbol kinds, data-object sizes, no undefined non-libc imports |
| `tests/phase_d_alloc_traffic.rs` | Phase D — rows T1–T4, `matrixsum`'s call structure and allocator traffic |
| `check_features.sh` | Phase D — runs the whole suite for every feature combination × both cdylib profiles |
| `mutation_check.sh` | validates the suite itself: 31 injected bugs must all be caught |

## Running it

```sh
# 1. build the C shared library (required by every test)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. build the Rust cdylib — `cargo test` does NOT build it, because the crate is
#    cdylib-only, so this step is mandatory
cd translation && cargo build --release --offline

# 3. run the differential suite
cargo test --offline

# everything at once, across every feature combination and both cdylib profiles
./check_features.sh

# prove the suite actually catches bugs
./mutation_check.sh
```

`--offline` is used because the crates.io index is unreachable in this sandbox;
`libloading 0.8.9` is already in the local cargo cache and pinned in `Cargo.lock`.

The harness finds the libraries automatically (newest of
`target/{release,debug}/libmatrixsum_lib.so`, and `../c_src/build/lib*.so`, whose
name CMake derives from the parent directory). Override with the `C_SO` and
`RUST_SO` environment variables.

## Results

* **Symbol parity: exact.** `nm -D --defined-only` yields the same 8 symbols
  (`init_array`, `expand_array`, `add_element`, `free_array`, `process_flags`,
  `calculate_matrix_checksum`, `matrixsum`, `matrix`) with the same kinds, and the
  exported data object `matrix` is 0x30 bytes in both. The diff is empty. Nothing
  had to be added or translated — the whole C translation unit was already covered.
* **63 tests**, all passing, over `2` feature combinations × `2` cdylib profiles,
  and additionally against the C compiled at every `CMAKE_BUILD_TYPE`
  (default/`-O0`, `Release`, `RelWithDebInfo`, `MinSizeRel`, `Debug`) — 10
  C×Rust build combinations, all green.
* **Value-level behaviour matched from the start** and still does: for every input
  tested, the Rust returns exactly what the C returns, including the C's
  undefined/implementation-defined behaviour — signed-integer wrap-around in `sum`
  and `sum * 0x10`, `size_t` wrap-around in `capacity * sizeof(int)` and
  `capacity * 2 * sizeof(int)`, `malloc(0)`, `realloc(p, 0)`, and the unchecked
  doubling of an absurd capacity.
* **Four real divergences were found and fixed** in `src/lib.rs` / `Cargo.toml`.
  All four were outside the return-value surface, which is exactly why the
  happy-path and error-code tests missed them:

  | # | divergence | C behaviour | Rust behaviour before the fix | fix | test |
  |---|-----------|-------------|-------------------------------|-----|------|
  | 1 | side-effect ORDER in `add_element` | `arr->data[arr->size++] = value`: GCC commits `arr->size = old + 1` FIRST, then stores the element, so a faulting store leaves `size == 1` | stored the element first, leaving `size == 0` | reordered the two writes to match GCC | `z5` |
  | 2 | misaligned caller pointers (dev profile) | no alignment promise: plain unaligned `mov`, returns normally | rustc's `Assert(PointerAlignment)` → `panic_nounwind` → **`abort()`** | all struct/element accesses go through `read_unaligned`/`write_unaligned`, plus `[profile.dev] debug-assertions = false` | `z3`, `z4` |
  | 3 | `data == NULL` with `size < capacity` (dev profile) | dies with **`SIGSEGV`** | died with **`SIGABRT`** (`panic_null_pointer_dereference`, and later the `ptr::copy_nonoverlapping` precondition check) | same as #2 | `z2` |
  | 4 | release inliner elided an allocation | `matrixsum` calls its helpers through the PLT: `malloc(24)`, `malloc(8)`, `realloc`, `free`, `free` | LLVM inlined the whole chain and SROA'd the `DynamicArray` away: only `malloc(8)`, `realloc`, `free` — one fewer allocation-failure point that can return `-1`, and non-interposable helpers | `#[inline(never)]` on all seven exported functions | `t1`–`t4` |

  After the fix the Rust release `.so` carries `R_X86_64_GLOB_DAT` relocations for
  all six helpers, mirroring the C's `R_X86_64_JUMP_SLOT` PLT entries, and the
  dev-profile `.so` contains **zero** `panic_misaligned_pointer_dereference` /
  `panic_null_pointer_dereference` / `precondition_check` sites.

## Notes on the environment (not translation bugs)

Three findings came from the *test harness*, not the Rust code, and are worth
recording because they are easy to mistake for divergences:

1. **The exported `matrix` is mutable global state shared by the whole test
   process.** Tests running concurrently clobbered each other's `matrix`, which
   looked exactly like a `matrixsum` divergence. Fixed with a process-wide lock in
   `common::load()`.
2. **`RLIMIT_DATA` is a per-process budget shared by both `.so`s** (6 GiB here).
   Holding a 4 GiB allocation from the C library live while asking the Rust
   library for another 4 GiB made the second one fail, which looked like an
   `init_array` divergence. Fixed by probing large capacities sequentially
   (`probe_init`: init → snapshot → free).
3. **A test that resets global state to its own constant cannot detect a wrong
   initializer.** `common::load()` resets `matrix` to `DEFAULT_MATRIX`, which
   masked a mutated initializer in the Rust `.so`. Found by `mutation_check.sh`
   and fixed with `tests/phase_b_pristine.rs`.

## Suite self-validation (`mutation_check.sh`)

35 deliberate bugs are injected into a *copy* of the Rust crate (`c_src/` and
`translation/src/` are never modified), the copy is built in BOTH profiles, and
the suite is pointed at each cdylib via `RUST_SO`. Result: **33 killed, 2
survivors**, and both survivors are provably semantics-preserving:

* removing `!!` from `has_read` — `flags & 0b0001` is already in `{0, 1}`;
* removing `!!` from `valid1` — the value is only consumed as `if valid1 != 0`.

The script asserts that these two survive and that everything else dies, so a
future test that "kills" an equivalent mutant is flagged as over-asserting.

Four real blind spots were discovered this way and closed:

| blind spot | why the rest of the suite missed it | closed by |
|-----------|-------------------------------------|-----------|
| a wrong `matrix` initializer | the harness reset the global to its own constant before comparing | `phase_b_pristine.rs` |
| a missing `free()` in `free_array` | `free_array` returns `void`; nothing observable changes | `phase_c_leak.rs` (`mallinfo2` accounting) |
| crash mode / side-effect ordering | the process dies, so there is no return value to compare | `phase_c_crash.rs` (child processes + `MAP_SHARED` struct) |
| the release inliner eliding an allocation | allocator traffic stays balanced, so accounting cannot see it | `phase_d_alloc_traffic.rs` (`objdump` call structure) |

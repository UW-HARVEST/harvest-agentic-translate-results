# VERIFICATION.md — completion gate

Differential verification of the C→Rust translation of `c_src/src/lib.c`
(a merge sort over `spritebatch_sprite_t`, lifted from `cute_spritebatch`).

The C is the ground truth. Every behavioural quirk it has is reproduced, not
fixed — including the fact that `spritebatch_internal_sprite_less_than_or_equal`'s
second `if` is **dead code**, so `texture_id` can never influence the ordering.

## How to reproduce everything

```sh
./run_all_configs.sh     # symbol parity + Phases B/C in every configuration
./mutation_test.sh       # proves the suite rejects wrong translations
```

Individually:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../.. && cargo build          # cargo test does NOT build a cdylib-only lib
cargo test
```

Both `.so`s are loaded with `libloading` and driven **only** through their
exported `merge_sort` symbol, so the `#[no_mangle] extern "C"` wrapper is part
of what is under test. No Rust function is ever called directly.

## Artifacts

| file | contents |
|---|---|
| `SYMBOLS.md` | `nm -D` symbol parity (Phase A / D) |
| `ERRORS.md` | error-surface table, 15 rows (Phase A, gates Phase C) |
| `CONFIGS.md` | configuration-surface table, 30 rows (Phase A, gates Phase B) |
| `tests/common/mod.rs` | dlopen harness, byte-exact comparison, seeded RNG |
| `tests/phase_b_valid.rs` | 34 tests — valid-path rows C1..C30 + ABI + coverage |
| `tests/phase_c_errors.rs` | 15 tests — error/boundary rows E1..E15 |
| `run_all_configs.sh` | Phase D driver (enumerates feature combos mechanically) |
| `mutation_test.sh` | 20 mutants: 18 must be killed, 2 must survive |

## Completion gate

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing / 0 undefined non-libc symbols.**
      The C `.so` exports exactly one dynamic symbol, `merge_sort` (its three
      helpers are `static`). The Rust `.so` exports `merge_sort` with the exact
      same name. `comm -23` of the two sorted symbol lists is **empty**, for
      both the debug and the release artifact. No symbol is stubbed; no C module
      was left untranslated (`c_src/CMakeLists.txt` lists exactly one source
      file and all four of its functions have Rust counterparts).

- [x] **Phase B: every row in `CONFIGS.md` passes across randomized inputs.**
      31 rows (C1..C30, C30 split into a/b), 34 tests, **≈40,000 differential
      `merge_sort` call pairs** from a fixed seed. Every call compares **both**
      caller-visible buffers — the sorted result `a` *and* the scratch buffer
      `b`, which the C leaves holding intermediate merge state — byte-for-byte
      across all 16 bytes per element, tail padding included.

- [x] **Phase C: every row in `ERRORS.md` has a passing error-path
      differential test.** 15 rows. The C has *no* explicit error surface
      (0 error returns, 0 asserts, 0 null checks, 0 enums — grep census in
      `ERRORS.md`), so the rows are the implicit/degenerate/out-of-contract
      behaviours: `size` = 0 / 1 / one-past-buffer / negative / `INT_MIN` /
      `INT_MAX`, NULL source, NULL scratch, NULL + zero length, aliased and
      partially-overlapping buffers, and the signed/unsigned comparison
      extremes. Faulting rows run in **child processes** and assert the *same
      signal* (not merely "both failed"); the in-contract rows additionally
      compare memory byte-for-byte.
      Out-of-range enum values are explicitly discharged: the API declares no
      enum, and the only scalar parameter `int size` is swept over its
      boundaries (`0, ±1, INT_MIN, INT_MAX, n+1, 2^12, 2^20, 2^28`).

- [x] **All of the above hold under EVERY feature combination.** `Cargo.toml`
      declares no `[features]` and `c_src/CMakeLists.txt` declares no options or
      `#ifdef`s, so the powerset of build configurations is the single empty
      combination — enumerated mechanically by `run_all_configs.sh` rather than
      assumed. Verified for that combination against **three** Rust artifacts:
      `dev`, `release`, and a `-C debug-assertions=on -C overflow-checks=on`
      build (which proves no Rust UB/overflow check ever fires on in-contract
      input).

## Evidence the suite has teeth

`./mutation_test.sh` → **20/20 expectations met**.

18 deliberately wrong translations are all **killed**: comparator direction and
strictness (`<=`→`<`, `>=`, swapped operands, forced-false), reviving the dead
`texture_id` tiebreak, dropping the padding from the element copy, swapping the
source/scratch roles at all three call sites, the recursion base case, the merge
loop bound, the right-run-exhausted short circuit, the `i < split` guard, the
index increments, and two mutations of the `int`→`size_t` conversion in the
`memcpy` length.

2 **semantically equivalent** mutants correctly **survive**, which shows the
suite is not merely over-fitted to the exact source text:

* rewriting `(lo + hi) / 2` as `lo + (hi - lo) / 2` — identical for every
  non-negative `lo ≤ hi` reachable from `merge_sort`;
* disabling `lib.c:9` entirely — it is unreachable, so nothing observes it.
  This mutant is the positive proof of the documented dead-code quirk.

## Notable findings during verification

1. **`size = -1` does not crash.** `sizeof(...) * size` sign-extends to
   `0xFFFFFFFFFFFFFFF0`; glibc's `memmove` treats that as an overlap, copies
   backward from a wrapped address and the process *survives with exit code 0*
   (stable over 12 runs), while `size = -1000` and `INT_MIN` reliably
   `SIGSEGV`. The Rust reproduces each sub-case exactly because
   `(size as usize).wrapping_mul(16)` computes the same length as gcc's
   `cltq; shl $0x4`.
2. **Rust's debug `ub_checks` were masking the comparison.** In a stock debug
   build, `ptr::copy_nonoverlapping` aborts (`SIGABRT`) on a NULL/oversized
   copy, where the unchecked C faults (`SIGSEGV`) or survives. Since the C is
   compiled with no instrumentation at all, `[profile.dev]` sets
   `debug-assertions = false` / `overflow-checks = false` so the tested artifact
   is faithful to the C contract; `run_all_configs.sh` separately re-runs the
   valid-path phase with both checks forced **on** to prove they never fire on
   in-contract input. The translation was already correct — only the
   instrumentation differed.
3. **Padding is observable and does match.** gcc compiles `b[k] = a[i]` as two
   8-byte moves, so the 4 tail padding bytes propagate with the element; the
   Rust `copy_nonoverlapping(.., 1)` copies the same 16 bytes. Rows with random
   padding (C3, C7, C15, C22, C27, C29) pin this, and the `no_padding_copy`
   mutant confirms they would catch a field-wise copy.
4. **Robust across C toolchains.** The same Rust artifact matches the C built
   with gcc `-O0/-O1/-O2/-O3/-Os` and clang `-O2` (both phases), so the
   translation does not depend on a particular C codegen choice.

## Conclusion

The translation in `src/lib.rs` is **byte-for-byte equivalent** to
`c_src/src/lib.c` across every configuration, every `CONFIGS.md` row and every
`ERRORS.md` row. No divergence was found in the Rust source; no change to
`src/lib.rs` was required. Nothing under `c_src/` was modified.

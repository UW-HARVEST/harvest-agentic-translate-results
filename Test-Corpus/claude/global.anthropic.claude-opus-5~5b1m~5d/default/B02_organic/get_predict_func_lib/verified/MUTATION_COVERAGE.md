# MUTATION_COVERAGE.md — proof that the differential suite actually bites

Passing tests only mean something if they would *fail* on a wrong translation.
`../mutation_coverage.py` proves that mechanically: it applies one small,
behaviour-changing edit at a time to `src/lib.rs`, re-runs the whole suite under
**both** feature sets, and requires the suite to fail. A mutation that survives
is a coverage hole.

Run it with:

```
python3 mutation_coverage.py        # from the working directory
```

## Result

```
baseline: PASS
running 78 mutations
...
ALL 78 MUTATIONS KILLED — no coverage holes found
```

## What the 78 mutations cover

| group | count | what is perturbed |
|---|---|---|
| `BTAC1C2_PredictSample` arms `0..=15` + `default` | 14 | tap index 1 → 3 in every arm; `pred = 0` → `1` in the `default` arm |
| `BTAC1C2_PredictSample` shifts / divisors | 11 | every `>> n` → `>> n+1`, every `/ n` → `/ 2n` |
| `firfx` addressing | 2 | row index `pfcn - 12` → `0`; column `7` → `6` |
| `_Pfn0`..`_Pfn11` | 22 | tap index 1 → 3 in each; each one's shift or divisor perturbed |
| `s()` index-masking helper | 1 | mask `& 7` → `& 3` |
| `BTAC1C2_GetPredictFunc` | 25 | each of the 12 arms rerouted to the generic fn **and** to the neighbouring `_PfnN`; the `default` arm rerouted to `_Pfn0` |
| `get_predict_func` | 3 | `default` arm returns 1; the `pfcn == 11` arm dropped; initial `result` 0 → 1 |

## A real coverage hole this found (and how it was closed)

The first run had **one survivor**:

```
[53/56] *** SURVIVED *** selector: default -> Pfn0
```

`BTAC1C2_GetPredictFunc`'s `default:` arm is genuinely unobservable through the
public API: `get_predict_func`'s own `default:` arm never inspects the pointer it
got back, so returning `_Pfn0` instead of the generic `BTAC1C2_PredictSample`
changes nothing the wrapper reports. It was also unreachable through the original
`__difftest_predict` hook, which dispatches on its own `which` argument and never
calls the selector at all.

Two hooks were added on both sides (Rust `src/lib.rs` and the C shim
`difftest_c/shim.c`) to make it observable:

* `__difftest_selector(pfcn) -> int` — returns *which* function the selector
  chose (`0..=11` for `_PfnN`, `12` for the generic dispatcher, `-1` for an
  unrecognised pointer);
* `__difftest_call_selected(psamp, idx, pfcn, ridx) -> int` — calls whatever the
  selector returned, exercising selector + predictor as a **composed pipeline**
  rather than as isolated units.

New rows `C39`–`C41` (`CONFIGS.md`) and `E8b`–`E8d` (`ERRORS.md`) cover them, and
the mutation now dies.

## A harness bug this found

Before the mutation run, the harness located the Rust `.so` at
`target/<profile>/libget_predict_func_lib.so` if it existed. But `cargo test` does
**not** link integration tests against a `cdylib`-only crate, so it does not
necessarily rebuild that file — the harness was happily verifying a **stale**
`.so` left over from an earlier `cargo build`. The very first mutation run
exposed this: a deliberately broken `_Pfn10` passed every test.

`tests/common/mod.rs` now always runs `cargo build --lib` itself into a private
target directory (matching profile and features, no lock contention with the
outer `cargo test`), and additionally asserts the resulting `.so` is newer than
`src/lib.rs` and `Cargo.toml`. Re-running the same mutation afterwards killed it
in 6 tests.

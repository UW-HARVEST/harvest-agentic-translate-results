# CONFIGS.md — Phase A: configuration-surface table

## Build-time configuration axes

| source | axes found |
|--------|------------|
| `Cargo.toml` | **no `[features]` section at all** → exactly one feature combination: the empty set (`--no-default-features`, identical to the default build) |
| `c_src/CMakeLists.txt` | no `option()`, no `target_compile_definitions`, no `#ifdef` in `src/lib.c` or `include/lib.h` → exactly one C configuration |

So Phase D's "repeat for every feature combination" collapses to a single
combination, which is nonetheless run explicitly as
`cargo test --no-default-features` (see `run_all.sh`).

## Public API surface (full set of entry points)

`c_src/include/lib.h` declares exactly one function, and it *is* the
lowest-level entry point (there are no convenience wrappers to hide behind):

```c
int encode_quant(int uni, int step, int pred, int tgt, int tgt2, int lsbit);
```

## Runtime option / input-shape axes (grepped from the branches the C takes)

| axis | source lines | distinct states the C actually distinguishes |
|------|--------------|----------------------------------------------|
| **A — `lsbit` mode selector** | 12, 13, 20, 24 | `A0`: `lsbit == 0` (no LSB fixup at all) · `A4`: `lsbit == 4` (dither/"clear then re-OR from bits 1&2") · `AODD`: `lsbit != 0,4` and `lsbit & 1` (force bit 0 set) · `AEVEN`: `lsbit != 0,4` and `!(lsbit & 1)` (force bit 0 clear) |
| **B — `uni1` clamp** | 8-9 | `B_CLAMP`: `uni & 7 == 7`, so `uni+1` carries out of the 3-bit magnitude → `uni1 = uni` · `B_FREE`: otherwise, `uni1 = uni+1` |
| **C — `uni2` clamp** | 10-11 | `C_CLAMP`: `uni & 7 == 0`, so `uni-1` borrows out of the 3-bit magnitude → `uni2 = uni` · `C_FREE`: otherwise, `uni2 = uni-1` |
| **D — sign bit `uni & 8`** | 31-32, 37-38, 43-44 | `D_POS`: clear → `diff` kept positive · `D_NEG`: set → `diff` negated. (Note: bit 3 is provably identical for `uni`, `uni1`, `uni2` — the ±1 either stays inside bits 0-2 or is clamped away — so this axis has 2 states, not 8.) |
| **E — `uni` sign / high bits** | 17-19 (`uni >> 1`, `uni >> 2` are *arithmetic*), 30 (`uni & 7`) | `E_NONNEG` · `E_NEG` (sign-propagating shifts change the `A4` fixup) |
| **F — `step` shape** | 30, 36, 42 (`(2*(uni&7)+1) * step / 8`) | `F_ZERO` (0 → all three diffs 0, forces the three-way tie) · `F_TINY` (`1..=7`: `/8` truncates to 0 for small `uni&7`) · `F_SMALL_POS` · `F_SMALL_NEG` · `F_OVF_POS` (`step > INT_MAX/15` → multiply wraps) · `F_OVF_NEG` · `F_EXTREME` (`INT_MAX` / `INT_MIN`; the latter also makes `-diff` wrap) |
| **G — `pred` shape** | 33, 39, 45 | `G_ZERO` · `G_SMALL` · `G_EXTREME` (`pred + diff` wraps) |
| **H — `tgt` / `tgt2` relationship** | 34-56 | `H_EQ` (`tgt2 == tgt`) · `H_ZERO2` (`tgt2 == 0`) · `H_FAR` (`\|tgt2 - p\| >= 32` → the `>> 5` penalty is non-zero) · `H_NEAR` (`< 32` → penalty truncates to 0) · `H_EXTREME` (subtractions wrap) |
| **I — decision outcome** | 57-60 | `I_KEEP` (neither `d1` nor `d2` beats `d0`) · `I_UP` (`d1 < d0` only) · `I_DOWN` (`d2 < d0` only) · `I_BOTH` (both → the second `if` still tests `d0`, so `uni2` wins) · `I_TIE1` (`d1 == d0`, strict `<` keeps `uni`) · `I_TIE2` (`d2 == d0`) |

## Configuration table (one row per combination the C treats differently)

Every row is exercised with **many** randomized inputs from a fixed-seed
SplitMix64 generator (or exhaustively where the domain is small), comparing the
C `.so` and the Rust `.so` return values byte-for-byte.

| #  | entry point | configuration (options set + input shape) | [ ] |
|----|-------------|-------------------------------------------|-----|
| 1  | `encode_quant` | `A0` · degenerate baseline: `uni` exhaustive `0..=15`, `step=pred=tgt=tgt2=0` (`F_ZERO`,`G_ZERO`) | [x] |
| 2  | `encode_quant` | `A0` · `B_FREE`+`C_FREE` (`uni & 7` in `1..=6`) · `F_SMALL_POS` · random `pred/tgt/tgt2` | [x] |
| 3  | `encode_quant` | `A0` · `C_CLAMP` (`uni & 7 == 0`) · random `step/pred/tgt/tgt2` | [x] |
| 4  | `encode_quant` | `A0` · `B_CLAMP` (`uni & 7 == 7`) · random `step/pred/tgt/tgt2` | [x] |
| 5  | `encode_quant` | `A0` · `D_POS` (`uni & 8 == 0`) · random rest | [x] |
| 6  | `encode_quant` | `A0` · `D_NEG` (`uni & 8 == 8`) · random rest | [x] |
| 7  | `encode_quant` | `A4` · `uni` with bits 1 **and** 2 set (the `(uni>>1)&(uni>>2)&1` re-OR fires) · random rest | [x] |
| 8  | `encode_quant` | `A4` · `uni` with bits 1,2 **not** both set (re-OR contributes 0) · random rest | [x] |
| 9  | `encode_quant` | `A4` · `E_NEG` (negative `uni`, arithmetic `>>` feeds the re-OR) · random rest | [x] |
| 10 | `encode_quant` | `A4` · `B_CLAMP`/`C_CLAMP` interaction: `uni & 7 ∈ {0,7}` with the bit-0 rewrite applied *after* clamping · random rest | [x] |
| 11 | `encode_quant` | `AODD` with `lsbit == 1` · random `uni/step/pred/tgt/tgt2` | [x] |
| 12 | `encode_quant` | `AODD` with random odd `lsbit` (incl. `3,5,7,0x7fff_ffff`) · random rest | [x] |
| 13 | `encode_quant` | `AEVEN` with `lsbit == 2` · random rest | [x] |
| 14 | `encode_quant` | `AEVEN` with random even `lsbit ∉ {0,4}` (incl. `6,8,12,100`) · random rest | [x] |
| 15 | `encode_quant` | all four `A*` modes × `F_ZERO` (`step == 0` → `d0==d1==d2`, exercises `I_TIE1`+`I_TIE2` together) | [x] |
| 16 | `encode_quant` | all four `A*` modes × `F_TINY` (`step ∈ 1..=7`, `/8` truncation) × `uni` exhaustive `0..=15` | [x] |
| 17 | `encode_quant` | `A*` × `F_SMALL_NEG` (`step ∈ -1..=-1024`) · random rest | [x] |
| 18 | `encode_quant` | `A*` × `F_OVF_POS` (`step > INT_MAX/15`, signed multiply wraps) · random rest | [x] |
| 19 | `encode_quant` | `A*` × `F_OVF_NEG` (`step < INT_MIN/15`) · random rest | [x] |
| 20 | `encode_quant` | `A*` × `F_EXTREME` (`step ∈ {INT_MAX, INT_MIN, INT_MIN+1}`) × `uni` exhaustive `0..=15` (`INT_MIN` also wraps `-diff`) | [x] |
| 21 | `encode_quant` | `A*` × `G_EXTREME` (`pred` near `INT_MIN`/`INT_MAX` → `pred+diff` wraps) · random rest | [x] |
| 22 | `encode_quant` | `A*` × `H_EXTREME` (`tgt`,`tgt2` near `INT_MIN`/`INT_MAX` → `tgt-p` wraps, pseudo-abs on `INT_MIN`) | [x] |
| 23 | `encode_quant` | `A*` × `H_EQ` (`tgt2 == tgt`) · random rest | [x] |
| 24 | `encode_quant` | `A*` × `H_ZERO2` (`tgt2 == 0`, asymmetric penalty) · random rest | [x] |
| 25 | `encode_quant` | `A*` × `H_FAR` (`tgt2` far from `pred` so the `>>5` penalty dominates and can flip the winner) | [x] |
| 26 | `encode_quant` | `A*` × `H_NEAR` (`\|tgt2-p\| < 32` so the penalty truncates to 0) | [x] |
| 27 | `encode_quant` | `I_UP` forced: `tgt` placed on the `uni1` reconstruction level | [x] |
| 28 | `encode_quant` | `I_DOWN` forced: `tgt` placed on the `uni2` reconstruction level | [x] |
| 29 | `encode_quant` | `I_BOTH` forced: `tgt` equidistant-ish so both `d1<d0` and `d2<d0` (`uni2` must win) | [x] |
| 30 | `encode_quant` | `I_KEEP` forced: `tgt` exactly on the `uni` reconstruction level | [x] |
| 31 | `encode_quant` | realistic codec domain: `uni` exhaustive `0..=15` × `step` exhaustive `1..=256` × random `pred/tgt/tgt2` in `±32768` × all `A*` modes | [x] |
| 32 | `encode_quant` | unconstrained fuzz: all six arguments uniform over the full `i32` range, 200 000 vectors | [x] |
| 33 | `encode_quant` | structured fuzz: each of the six arguments independently drawn from a "corner value" pool (`0, ±1, ±7, ±8, ±15, 32, 255, INT_MIN, INT_MAX, …`), 200 000 vectors | [x] |
| 34 | `encode_quant` | full exhaustive small grid: `uni ∈ 0..=15` × `lsbit ∈ {0,1,2,3,4,5,6,8,12,-1,-2,-4}` × `step ∈ {0,1,7,8,9,-1,-8,255,-255}` × `(pred,tgt,tgt2)` from a 6-value pool | [x] |

## Verification results

All 34 rows above pass. Run everything with `./run_all.sh`, or manually:

```sh
cargo build --no-default-features      # MANDATORY: `cargo test` does not
                                       # rebuild a cdylib-only lib target
cargo test  --no-default-features --no-fail-fast
```

`tests/common/mod.rs` additionally refuses to run against a `.so` whose mtime
predates `src/`, so a stale artifact cannot produce a false pass. (This was a
real trap: the first run of the suite silently tested a stale `.so`.)

| dimension | result |
|-----------|--------|
| feature combinations | 1 (`--no-default-features`); no `[features]` section exists |
| Phase B tests (`tests/configs.rs`) | 34 passed, 0 failed |
| Phase C tests (`tests/errors.rs`) | 23 passed, 0 failed |
| differential calls executed | ~2.4 million per run |
| divergences found | **0** |
| changes needed in `src/lib.rs` | **none** — the translation was already exact |

### Cross-configuration robustness (beyond what is strictly required)

Because the C relies on wrap-around signed arithmetic (technically UB), the same
suite was re-run against alternative builds of both sides. All agree:

| build | result |
|-------|--------|
| Rust debug cdylib vs C `-O0` (default cmake) | 57/57 pass |
| Rust **release** cdylib (`panic = "abort"`) vs C `-O0` | 57/57 pass |
| Rust debug vs C `gcc -O1` | 57/57 pass |
| Rust debug vs C `gcc -O2` | 57/57 pass |
| Rust debug vs C `gcc -O3` | 57/57 pass |
| Rust debug vs C `gcc -Ofast` | 57/57 pass |

### Mutation testing (suite-sensitivity evidence)

Passing tests only mean something if they can fail. Each mutant below was
injected into `src/lib.rs`, rebuilt, and the suite re-run with
`--no-fail-fast`; every behaviour-changing mutant is caught.

| mutant | injected change | failing tests |
|--------|-----------------|---------------|
| M1  | secondary-target penalty `d3 >> 5` -> `>> 4` | 26 |
| M2  | "fix" the quirk: `d2 < d0` -> `d2 < d1` | 19 |
| M3  | truncating `/8` -> floor division (`div_euclid`) | 5 |
| M4  | `lsbit == 4` -> `lsbit & 4 != 0` | 6 |
| M5  | pseudo-abs `d0 >> 31` arithmetic -> logical shift | 32 |
| M6  | clamp mask `& ~7` -> `& ~3` | 48 |
| M7  | `d1 < d0` -> `d1 <= d0` (tie handling) | 27 |
| M7b | `d2 < d0` -> `d2 <= d0` (tie handling) | 53 |
| M8  | sign test `uni & 8` -> `uni & 16` | 30 |
| M9  | pseudo-abs -> `wrapping_abs()` (off-by-one for negatives) | 11 |
| M11 | `uni + 1` -> `uni + 2` | 52 |
| M12 | penalty computed from `tgt` instead of `tgt2` | 30 |
| M13 | odd-`lsbit` branch `\|= 1` -> `&= ~1` | 19 |
| M14 | drop the `+1` in `2*(uni&7)+1` | 31 |
| M15 | `uni - 1` -> `uni + 1` | 51 |
| M16 | delete the `lsbit == 4` re-OR of bit 0 | 35 |
| M17 | `lsbit != 0` -> `lsbit > 0` (negative `lsbit`) | 15 |
| M18 | even-`lsbit` branch `&= ~1` -> `\|= 1` | 39 |
| M10 | lsbit==4 branch shifts arithmetic -> logical | 0 — **equivalent mutant**: the results are masked with `& 1`, so only bits 1-2 of `uni` survive and the shift kind provably cannot matter |

Mutation score: **18 / 18** behaviour-changing mutants detected.

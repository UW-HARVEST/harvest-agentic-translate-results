# VERIFICATION.md — completion gate

Differential verification of `translation/` (Rust `cdylib`) against
`c_src/` (the C reference, the ground truth).

## How to reproduce

```sh
cd translation
./run_tests.sh          # builds both .so's, runs the whole suite
./check_features.sh     # repeats it for every feature combination
```

`cargo test` alone is also correct: the harness rebuilds the release `cdylib`
itself before `dlopen`ing it (see below).

Every test loads **both** shared objects with `libloading` and calls them only
through their exported C symbols. The Rust crate is never linked directly, so the
`#[no_mangle] extern "C"` wrappers and the struct ABI are themselves under test.
All comparisons are **bit-exact**: floats by `to_bits()`, structs by raw bytes
(no struct in this library has padding, which is asserted at compile time in
`tests/common/mod.rs`).

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` shows **31 exported symbols in the C `.so` and
      31 in the Rust `.so`, with an empty diff in both directions**, and **0
      undefined non-libc symbols** in the Rust `.so`. Re-verified at test time by
      `tests/symbols.rs`, so the document cannot go stale.
- [x] **Phase B** — all **87 rows** of `CONFIGS.md` pass across randomized inputs
      from a fixed seed (`0x5eed_1234_c0ffee01`, SplitMix64).
- [x] **Phase C** — all **69 rows** of `ERRORS.md` have a passing error-path
      differential test, including the 23 unchecked-dereference rows, which are
      compared by **termination signal in a child process** (all 23: SIGSEGV in
      both libraries) rather than by "both failed somehow".
- [x] **Phase D** — both feature configurations (`default` and
      `--no-default-features`; `Cargo.toml` declares no `[features]`, so those are
      the only two) pass the full suite *and* the symbol-parity diff.

```
tests/symbols.rs         3 tests    symbol parity + import audit
tests/leaf.rs           15 tests    CONFIGS rows 1..15
tests/shape_simplex.rs  15 tests    CONFIGS rows 16..41
tests/gjk.rs            18 tests    CONFIGS rows 42..82
tests/errors.rs         34 tests    ERRORS rows 1..68 (+ CONFIGS 83..87)
                        --------
                        85 tests, 0 failures
```

## Bugs found and fixed

The Rust translation was **behaviourally wrong on NaN inputs** for nine call
sites. Its doc comments had been written against an *optimised* disassembly, but
`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the reference library is
built with `C_FLAGS = -fPIC` and **no optimisation** (`-O0`). At `-O0` GCC keeps
every local in a stack slot and picks different SSE destination registers than it
does at `-O2`, and `ADDSS`/`MULSS`/`SUBSS`/`DIVSS` return the **destination**
operand's NaN in preference to the source's — so the operand order is observable
through a NaN's sign and payload.

Each fix below was re-derived from `objdump -d` of the actual reference `.so`
(GCC 11.5.0, x86-64) and is annotated with that instruction in `src/lib.rs`:

| function | was (from an `-O2` build) | is (verified against `-O0`) |
|----------|---------------------------|------------------------------|
| `c2Mulvs` | `mul_r(a.x, b)` | `mul_l(a.x, b)` — a memory operand can only be the `mulss` *source* |
| `c2Add` | `add_l` | `add_r` — GCC loaded `b` into the accumulator |
| `c2Dot` | `add_l(mul_l, mul_l)` | `add_r(mul_l(a.x,b.x), mul_r(a.y,b.y))` |
| `c2Det2` | `sub_l(mul_l, mul_l)` | `sub_l(mul_r(a.x,b.y), mul_r(a.y,b.x))` |
| `c2Mulrv` | `sub_l(mul_l,mul_l)`, `add_r(mul_l,mul_l)` | `sub_l(mul_r,mul_r)`, `add_l(mul_l,mul_r)` |
| `c2MulrvT` | `sub_l(mul_l(a.c,b.y), …)` for `y` | `add_l(mul_l(fneg(a.s),b.x), mul_r(a.c,b.y))` — `-O0` does **not** fold `-x+y` into a `subss`; it materialises `-a.s` with `xorps` and the `addss` destination is the *first* term |
| `c23` (`uBC+vBC`) | `add_r` | `add_l` |
| `c23` (`uABC+vABC+wABC`) | `add_r(add_l(…), wABC)` | `add_l(add_l(…), wABC)` |
| `c2Witness` (all `den*u`) | `mul_l(den, u)` | `mul_r(den, u)` — `den` is the memory operand, so `u` is the destination |
| `c2L` (`den*v1.u`) | `mul_l(den, u)` | `mul_r(den, u)` |

All ten were caught by `tests/leaf.rs` / `tests/shape_simplex.rs` once the tests
drove NaN payloads through both operands; reverting any one of them turns the
suite red (verified individually).

## Test-strength audit (mutation testing)

Passing tests only mean something if they *can* fail. **59 deliberate mutations**
were injected into `src/lib.rs` one at a time, rebuilt, and re-run against the
whole suite. 49 were caught; the 10 survivors are analysed below.

**Result: every mutation that is behaviourally observable was caught.** Each of
the 10 survivors was shown to be *provably equivalent* to the original — a
semantics-preserving rewrite, not a test gap. The reasoning is recorded in
`ERRORS.md` under "Provably unobservable sites". Examples:

* `c2Support`'s loop starting at `i = 0` instead of `i = 1` cannot change the
  result, because re-testing vertex 0 compares `dot > dmax` with `dot == dmax`.
* `c23`'s `div = uBC + vBC` cannot depend on operand order, because that arm is
  guarded by `uBC > 0 && vBC > 0`, so neither operand can be NaN.
* `den * u` in `c2Witness`/`c2L` is only observable for the **last** term of the
  `c2Add` chain, because `c2Add` is `add_r` and discards the left operand's NaN —
  and both operands are NaN precisely when `den` is, which is the only case where
  `mul_l` and `mul_r` differ. Mutating the last term *is* caught.
* `iter < 20` is unreachable (max observed: 4 iterations).

Three real gaps were found this way and closed:

1. `c2Len(c2Sub(a,b))` vs `c2Len(c2Sub(b,a))` — identical for every finite input,
   so it needed shape coordinates carrying **distinct** NaN payloads
   (`err_gjk_nan_shape_coords`).
2. `metric < -1.0e8f` vs `<=` — needed the *freshly computed* simplex metric to
   land exactly on the constant **and** `cache_was_read` to be observable
   (`err_gjk_cache_metric_threshold`; see below).
3. `min_metric < 2*max_metric` vs `<=` — needed exact equality with
   `metric < -1e8` (`err_gjk_cache_metric_double_boundary`).

Gaps 2 and 3 are worth spelling out because the naive test for them is vacuous.
GJK is *self-correcting*: for most shape pairs a warm start and a cold start
converge to the same witness points, so flipping `cache_was_read` is invisible. A
construction was needed that satisfies both requirements at once:

* `A` is a **circle** — one proxy vertex, so `iA = [0,0,0]` is always in range and
  no uninitialised proxy slot is ever read;
* `B` is `AABB{(bx,by),(bx+W,by+H)}` with `iB = [0,1,3]`, making the simplex
  differences `(W,0)` and `(0,H)` and hence `metric = W*H`;
* **every coordinate is an integer** below `2^24`, so all the subtractions are
  exact and `metric == W*H` *regardless of where the shapes sit*.

That decouples "hit the constant exactly" from "make the outcome observable", so
the position can be swept freely. `W=10000, H=-10000` gives exactly `-1.0e8f`;
2048 of 3000 sampled positions make `cache_was_read` observable.

## A trap worth recording

`cargo test` does **not** rebuild the `cdylib`, because the integration tests
never link it — they `dlopen` it. The suite therefore happily ran against a
**stale `.so`** and reported green while `src/lib.rs` contained an injected bug.
`tests/common/mod.rs::rust_so_path()` now rebuilds the release library itself
(once per test binary, guarded by `OnceLock`) before resolving the path, so
`cargo test` is self-correcting. `GJK_RUST_SO` / `GJK_NO_BUILD` override it, and
`run_tests.sh` builds both libraries up front.

## Deliberately not asserted

Four input classes are **undefined behaviour whose observable result depends on
the contents of uninitialised C stack**, so requiring the Rust to match them
would be requiring it to match a coin flip. They are enumerated in `ERRORS.md`
("Untestable rows") with the reason and the nearest *defined* neighbour that is
tested instead:

* `c2GJK` with `typeA`/`typeB` outside `{0,1,2}` — `c2MakeProxy` has no
  `default:`, so `c2Proxy pA;` (uninitialised, lib.c:371) keeps stack garbage and
  `pA.count`/`pA.verts` are then read. The same fallthrough *is* tested directly
  on `c2MakeProxy` with a caller-owned zeroed proxy, where it is deterministic.
* `cache->iA[i] >= pA.count` — reads proxy vertex slots `c2MakeProxy` never wrote.
  Every **in-range** index is tested for all three shape types.
* `cache->iA[i] >= 8` or `< 0` — indexes outside `c2Proxy.verts[8]`.
* `cache->count >= 5` — `verts + 4` writes past the end of `c2Simplex` and
  `cache->iB[4]` reads past the end of the 36-byte `c2GJKCache`. `count == 4`, the
  largest value that stays in bounds, **is** tested (`ERRORS.md` row 65), where
  `iA[3]` aliases `iB[0]` and `iB[3]` aliases `div`'s bits read as an `int`.

Nothing in `src/lib.rs` is stubbed, faked or `unimplemented!()`; all 31 symbols
are real translations of the corresponding C functions.

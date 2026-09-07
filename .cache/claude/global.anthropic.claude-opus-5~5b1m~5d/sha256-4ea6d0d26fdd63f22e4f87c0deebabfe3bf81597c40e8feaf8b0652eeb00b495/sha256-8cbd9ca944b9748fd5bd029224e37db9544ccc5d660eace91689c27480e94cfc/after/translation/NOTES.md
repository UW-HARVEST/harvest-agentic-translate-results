# Verification notes

## What was verified

`c_src` is a single translation unit (`src/lib.c`, 48 lines) exporting a single
function, `void hsl_to_rgb(float *dest, const float *src)`. The Rust crate is a
`cdylib` exporting the same symbol. Every test in `tests/` loads **both** shared
objects with `libloading` and calls them through the FFI boundary — the Rust
implementation is never called directly, so the `#[no_mangle] extern "C"` wrapper
is itself under test.

Comparison is always **bit-for-bit** on all three output words (`f32::to_bits`),
so `+0.0` vs `-0.0` and differing NaN sign/payload are treated as failures.

Artifacts: `SYMBOLS.md` (Phase A), `ERRORS.md` (Phase C gate), `CONFIGS.md`
(Phase B gate). Driver: `run_all.sh`.

## Result

| gate | status |
|------|--------|
| `SYMBOLS.md` — `nm -D` parity | **PASS** — C exports exactly `hsl_to_rgb`; the Rust `.so` exports it too. 0 missing, 0 extra. |
| Phase B — all 32 `CONFIGS.md` rows | **PASS** (27 tests) |
| Phase C — all `ERRORS.md` rows | **PASS** (16 tests) |
| Phase D — symbol parity + smoke | **PASS** (2 tests) |
| Binary/driver stdout comparison | **N/A** — `CMakeLists.txt` has no `add_executable` and `Cargo.toml` has no `[[bin]]`; the project builds no executable. |
| All feature combinations | **PASS** — the crate declares no `[features]`; `default`, `--no-default-features` and `--all-features` were each run in both the `debug` and `release` profiles (6 combinations). |
| Heavy fuzz | **PASS** — 5,000,000 random bit-pattern triples + 5,000,000 random NaN triples (`--ignored` tests in `phase_d_symbols.rs`). |

No divergence was found against the C library built as `c_src/CMakeLists.txt`
specifies. No changes to `src/lib.rs` were required; the only source edit was
adding `libloading` to `[dev-dependencies]`.

## Important finding: the C is not bit-stable across GCC optimisation levels

The C source's **NaN results** depend on the optimisation level, because
`addss`/`mulss` forward the *first* source operand's NaN in preference to the
second, and GCC picks a different operand order for `x + m` at `-O0` than at
`-O2`.

Concrete witness (found by `tests/phase_d_c_optlevel.rs`, which compares two
builds of the *same* C source against each other):

```
src  = [0xaef69b01, 0xe93f586f, 0xff981750]        (h<0, s normal, l = -NaN)
C -O0 = [0xffd81750, 0x7fd81750, 0x7fd81750]
C -O2 = [0xffd81750, 0x7fd81750, 0xffd81750]       <-- dest[2] differs
                                       ^^^^
```

`h < 0` selects the third arm (the one the doubled `h < 120.0f` test makes the
negative-hue arm), whose third output is `x + m`. Both `x` and `m` are NaN here:
`x` carries the positive payload `0x7fd81750` and `m` the negative one
`0xffd81750`. At `-O0` GCC emits `addss x, m` (so `x`'s NaN survives); at `-O2`
it emits the operands the other way round (so `m`'s NaN survives).

**Consequence:** no single implementation can be bit-identical to both builds.
The conformance target is therefore the library produced by the project's own
`CMakeLists.txt` with the documented command

```sh
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

which sets no `CMAKE_BUILD_TYPE` and so compiles at `-O0`. The Rust matches that
build exactly, on every one of the ~10.6 million inputs exercised. `run_all.sh`
still runs the whole suite against an `-O2` build, but reports the result as a
diagnostic rather than a failure, and prints the C-vs-C self-consistency verdict
so the reason is never mistaken for a translation bug.

For all **non-NaN** inputs the two C builds agree with each other and with the
Rust; the instability is confined to which NaN payload propagates.

## Behavioural quirks of the C that the Rust deliberately reproduces

1. **The doubled comparison on line 27.** The third arm tests
   `h < 120.0f && h < 180.0f` rather than `h >= 120.0f && h < 180.0f`. Two
   consequences, both preserved and both directly tested:
   - hues in `[120, 180)` reach no arm and fall through to the final `else`,
     yielding grey `m, m, m` (`row06_h4_unreachable_arm`);
   - all **negative** hues (including `-INF`) take that arm and get
     `m, c+m, x+m` (`row05_h3_negative_quirk_arm`, `err_h_negative_takes_quirk_branch`).
2. **`s == 0` short-circuits before anything else is computed**, so a NaN or
   infinite hue is ignored entirely and `l` is copied verbatim — NaN payload
   included. `-0.0f` compares equal to `0` and short-circuits too.
3. **NaN saturation does not short-circuit** (`NaN == 0` is false), so the full
   formula runs.
4. **No validation whatsoever.** `s > 1`, `s < 0`, `l` outside `[0, 1]`, and
   infinities are all accepted and propagate arithmetically. `ERRORS.md`
   records that the function has no error surface at all: it is `void`, has no
   status return, no sentinel, no `errno`, no `assert`, and no length parameter.
5. **`1.0f *` on line 17 is a genuine no-op** and is documented as such in
   `src/lib.rs` rather than emitted, because multiplying by `1.0f` can neither
   change a finite value nor alter a NaN that the preceding subtraction has
   already quietened.

## Deliberately untested (undefined behaviour in the C)

`hsl_to_rgb` dereferences `src[0]` and stores to `dest[0]` unconditionally, and
takes no length parameter. Passing a null pointer, or a buffer shorter than three
`float`s, is undefined behaviour in the C — it segfaults rather than returning an
error, so there is no error code for the Rust to match. `ERRORS.md` rows E16/E17
record this, `err_null_and_short_buffers_are_ub_not_tested` pins the reasoning,
and the calls are not made. The Rust carries the identical `unsafe` contract.

Aliasing, by contrast, **is** well-defined and **is** tested: the C reads all
three inputs into locals before performing any store, so `dest == src` and
partially overlapping buffers behave exactly like disjoint ones
(`row29`/`row30`/`row31`, `err_dest_aliases_src`, `err_partial_overlap`).

## How to reproduce

```sh
# everything: both C builds, 6 profile x feature combos, symbol parity, all tests
bash translation/run_all.sh

# or by hand (note: cargo test does NOT build cdylib artifacts, so build first)
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . && cd ../..
cd translation && cargo build --release && cargo test --release
cargo test --release -- --ignored          # the two 5M-case fuzz runs
```

`--offline` is required in a sandbox without crates.io access; `run_all.sh`
passes it by default and it can be overridden with `OFFLINE= bash run_all.sh`.

Environment overrides understood by the harness:

| variable | effect |
|----------|--------|
| `HSL_C_SO` | path to the C `.so` to test against (default: first `c_src/build/lib*.so`) |
| `HSL_RUST_SO` | path to the Rust `.so` (default: the cdylib of the profile the test binary was built in) |
| `HSL_C_SO_ALT` | when set, `phase_d_c_optlevel.rs` compares this C build against `HSL_C_SO` instead of skipping |

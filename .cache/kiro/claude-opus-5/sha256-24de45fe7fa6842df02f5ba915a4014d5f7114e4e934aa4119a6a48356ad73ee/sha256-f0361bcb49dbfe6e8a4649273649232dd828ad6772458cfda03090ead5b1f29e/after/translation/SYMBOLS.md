# SYMBOLS.md — Phase A / Phase D symbol parity

Derived mechanically, not from assumptions:

```sh
CSO=c_src/build/libharvest-work-XIHzQf.so
RSO=translation/target/release/libcontrast_ratio_lib.so
nm -D --defined-only "$CSO" | awk '$2 ~ /^[TtWwDdBbRr]$/ {print $3}' | sort -u > /tmp/c_syms.txt
nm -D --defined-only "$RSO" | awk '$2 ~ /^[TtWwDdBbRr]$/ {print $3}' | sort -u > /tmp/r_syms.txt
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt     # symbols missing from Rust
```

## Public symbols exported by the C `.so`

| # | symbol | C `nm -D` | Rust `nm -D` | status |
|---|--------|-----------|--------------|--------|
| 1 | `contrast_ratio` | `T` | `T` | present |

`comm -23` output is **EMPTY** → 0 symbols missing from the Rust `.so`.

## C symbols deliberately NOT exported

`c_src/src/lib.c` declares two helpers `static`, so they have internal linkage
and appear in neither `.so`'s dynamic symbol table. They are correctly private
(non-`pub`, non-`no_mangle`) in the Rust translation as well:

| C symbol | linkage | Rust counterpart | exported? |
|----------|---------|------------------|-----------|
| `cbLuminance` | `static` (internal) | `fn cbLuminance` | no — correct |
| `cbContrastRatio` | `static` (internal) | `fn cbContrastRatio` | no — correct |

No C source file was left untranslated: the library is a single translation
unit (`src/lib.c`, 29 lines) plus one header (`include/lib.h`, 7 lines), and
every function in it has a Rust counterpart. No symbol is stubbed or
`unimplemented!()`.

## Undefined symbols in the Rust `.so`

`nm -D -u` on the Rust `.so` lists only libc / glibc / unwinder imports
(`pow@GLIBC_2.29`, `malloc`, `memcpy`, `_Unwind_*`, `__cxa_finalize`, …).
**0 undefined non-libc symbols.**

Notable: Rust's `f64::powf` lowers to the *same* `pow@GLIBC_2.29` that the C
object imports, so the transfer function is bit-identical by construction
rather than by luck.

## Build configurations covered

`translation/Cargo.toml` has **no `[features]` section**, so the only feature
configuration that exists is the default (empty) one. There are no `[[bin]]`
targets, no `src/main.rs`, and no `src/bin/`; `c_src/CMakeLists.txt` contains
no `add_executable`. Therefore there is no driver binary to diff, and the
Phase D "every feature combination" requirement is satisfied by the single
default configuration (verified below with `--no-default-features` as well).

## Completion gate (re-verified, `./verify_all.sh`)

| gate | result |
|---|---|
| `nm -D`: 0 missing symbols, 0 undefined non-libc symbols in Rust | PASS (all 4 configs) |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | PASS 27/27 rows, 28 tests |
| Binary/driver stdout diff | N/A — no `[[bin]]`, no `src/main.rs`, no `add_executable` |
| Phase C: every `ERRORS.md` row has a passing differential test | PASS 15/15 rows |
| Holds under every feature combination | PASS — `{default, --no-default-features}` × `{debug, release}` |

Harness integrity is itself asserted by `tests/phase_a_harness.rs` (4 tests): the
two `.so`s are distinct files, `dlsym` resolves `contrast_ratio` to two distinct
addresses, comparison is bit-exact (a 1-ULP difference is not tolerated), NaN is
compared by bits rather than `==`, and the Rust artifact is not stale.

Rows 7 and 8 sweep the **entire 2^24 color domain** (16 777 216 cases each,
stride 1) in both operand positions. Because the result depends on each operand
only through its `float` luminance, these two rows exhaust every reachable
luminance value on both sides of the division.

### Note on `0.0f / 0.0f`

The C's unguarded `High / Low` for black-vs-black executes a hardware `divss`,
producing the *negative* quiet NaN `0xFFC00000`. A compile-time-folded
`0.0 / 0.0` in Rust instead yields `0x7FC00000`. The translated
`contrast_ratio` is unaffected (its operands come from a runtime call, so the
division is not folded) and matches the C bit-for-bit — asserted by
`row16_black_vs_black_is_nan` and `err03_zero_over_zero_is_nan`. Worth knowing
if the function is ever made `const` or `#[inline]`-visible to callers, which
would let a caller const-fold it into the wrong NaN.

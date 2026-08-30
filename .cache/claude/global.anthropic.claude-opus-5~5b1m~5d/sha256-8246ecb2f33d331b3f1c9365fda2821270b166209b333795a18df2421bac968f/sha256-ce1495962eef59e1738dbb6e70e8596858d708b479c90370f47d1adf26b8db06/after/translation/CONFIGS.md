# CONFIGS.md — Phase A configuration-surface table

The mirror of `ERRORS.md`, for VALID inputs. Axes derived mechanically from the
C source and the public header, not from a guess about what "matters".

## Axis enumeration

### Axis 1 — runtime options / modes / flags

Grepped the public header and the source for anything the caller can toggle:

| candidate | found |
|---|---|
| public functions in `include/driver.h` | exactly **1**: `void driver(int x);` |
| global/`extern` variables, setters, init/config functions | **0** |
| environment variables read (`getenv`) | **0** |
| `#ifdef`-selected behaviour | **0** (only the `DRIVER_H_` include guard) |
| struct/context parameters carrying options | **0** |
| function pointers / callbacks | **0** |

**Result: the option axis is empty.** The library is stateless — there is no
init, no context object, no flag, no mode. `driver` is simultaneously the
highest-level convenience wrapper *and* the lowest-level entry point, so the
Phase B instruction to "exercise the LOW-LEVEL entry points directly, not only
the convenience wrappers" is satisfied by calling `driver` itself: there is no
lower level to reach and no composed pipeline to hide a bug in.

### Axis 2 — input shapes the code distinguishes

The single parameter is a by-value `int` (32-bit, two's complement on this
target). The source contains no branches, but the *observable* — the bytes
`printf("%d\n", y)` writes — is value-dependent, and the arithmetic
`y = 2*x + 300` has wrap discontinuities. Those give the real shape axes:

- **sign of the result `y`**: positive / zero / negative (the `-` sign byte)
- **decimal width of `y`**: 1 … 10 digits, plus the 11-char `-2147483648`
- **overflow class**: no overflow / `2*x` wraps / `y += 300` wraps
- **extremal values**: `INT_MIN`, `INT_MAX`, and the ±2^30 wrap thresholds
- **cardinality**: zero calls / one call / many consecutive calls (a stateless
  function must produce no cross-call drift; a stateful mistranslation would)

### Axis 3 — feature combinations

`translation/Cargo.toml` has **no `[features]` table** and no optional
dependencies:

```
$ grep -n 'feature' translation/Cargo.toml
(no output)
```

Therefore the feature powerset is the single element `{default}` = `{}`. There
is exactly **one** configuration to test; `--no-default-features` is equivalent
to the default build. This is verified explicitly rather than assumed (see
`check_feature_combos.sh`).

## CONFIGURATION-SURFACE TABLE

Cross-product of the axes above, pruned to combinations the C actually
distinguishes. Every row is a differential test that calls **both** `.so`s via
`libloading` and compares stdout byte-for-byte. Rows marked "randomized" draw
many inputs from a fixed-seed PRNG (seed `0x243F6A8885A308D3`) rather than one
hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `driver` | `x = 0` — identity input, result `300` | [x] |
| C2 | `driver` | small positive `x` in `[1, 1000]`, exhaustive — no overflow, positive result | [x] |
| C3 | `driver` | small negative `x` in `[-1000, -1]`, exhaustive — spans the `y>0 / y==0 / y<0` transition | [x] |
| C4 | `driver` | `x` in `[-150 - 8, -150 + 8]` — exact zero-crossing of `y`, sign-byte boundary | [x] |
| C5 | `driver` | result width 1 digit (`y` in `[0,9]`, i.e. `x` in `[-150,-146]`) | [x] |
| C6 | `driver` | result width 2…10 digits, positive — every `printf("%d")` field-width transition | [x] |
| C7 | `driver` | result width 2…10 digits + sign, negative — every negative field-width transition | [x] |
| C8 | `driver` | `x = INT_MAX` (`2*x` wraps to `-2`, `y = 298`) | [x] |
| C9 | `driver` | `x = INT_MIN` (`2*x` wraps to `0`, `y = 300`) | [x] |
| C10 | `driver` | `x` in `[INT_MAX-32, INT_MAX]` — upper wrap region, randomized + exhaustive | [x] |
| C11 | `driver` | `x` in `[INT_MIN, INT_MIN+32]` — lower wrap region, randomized + exhaustive | [x] |
| C12 | `driver` | `x` near `+2^30` (`0x40000000 ± 32`) — the `2*x` overflow threshold | [x] |
| C13 | `driver` | `x` near `-2^30` (`0xC0000000 ± 32`) — the `2*x` overflow threshold from below | [x] |
| C14 | `driver` | `x = 0x3FFFFFFF ± small` — `2*x` just below `INT_MAX` so `y += 300` is what wraps (add-overflow, distinct from C12's mul-overflow) | [x] |
| C15 | `driver` | `x` such that `y == INT_MAX` / `y == INT_MIN` exactly (extremal printable results) | [x] |
| C16 | `driver` | uniformly random `x` over the **full** `i32` range, 20000 draws, fixed seed | [x] |
| C17 | `driver` | random `x` restricted to powers of two and their neighbours `±(2^k), ±(2^k ± 1)` for k=0..31 | [x] |
| C18 | `driver` | many consecutive calls (5000) interleaved C/Rust in one captured stream — proves statelessness and no cross-call drift | [x] |
| C19 | `driver` | zero calls — capture harness self-check, both produce empty output (guards against the harness reporting false matches) | [x] |
| C20 | `driver` | one call in isolation with a freshly `dlopen`ed handle each time — proves no load-time/one-shot initialisation difference | [x] |

## Gate status

- [x] All 20 rows pass across randomized inputs.
- [x] Only one feature combination exists (`{default}`); all rows pass under it,
      and under `--no-default-features` (equivalent build).

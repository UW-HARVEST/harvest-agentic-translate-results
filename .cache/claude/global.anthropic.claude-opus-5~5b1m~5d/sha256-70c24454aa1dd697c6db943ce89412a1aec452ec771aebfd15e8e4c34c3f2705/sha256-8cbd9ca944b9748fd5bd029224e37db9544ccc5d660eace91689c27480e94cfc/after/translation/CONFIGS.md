# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the branches the C actually takes.

## Axes the C code distinguishes

Public entry points (`c_src/include/staticalias.h`), lowest level first:

* `int *static_alias(int *outer)` — the **low-level** entry point; owns the
  `static int inner = 1;` and the single `if (*outer >= inner)`.
* `void driver(int initial_value, int iterations)` — the **composed wrapper**;
  loops `iterations` times over `static_alias`, feeding each call's *returned*
  pointer back in as the next call's argument, and `printf("%d\n", *running_sum)`
  after each call.

There are **no runtime options, modes or flags** (no setters, no globals in the
header, no `#ifdef` in the source outside the header guard, no `switch`), so the
option axis is empty. The axes the code does branch on are:

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| **A** branch | `*outer >= inner` (then: mutate `inner`, return `&inner`) / `*outer < inner` (else: mutate `*outer`, return `outer`) | `staticalias.c:30` |
| **B** alias shape | `outer` → caller-owned `int` / `outer` == `&inner` (the static's own address, obtainable only from a previous return) | `staticalias.c:32,35` |
| **C** static state | fresh library load (`inner == 1`) / `inner` already advanced by earlier calls (persistent across calls **and** across entry points) | `staticalias.c:29` |
| **D** value shape | `INT_MIN`, negative, `0`, `1` (== the initial `inner`), `== inner` exactly, small positive, large positive, `INT_MAX` | operands of `>=`, `+=` |
| **E** call count | 1 / 2 / many (state accumulates; the branch *flips* once `*outer` catches up to `inner`) | caller-driven |
| **F** `iterations` | `0`, `1`, `2`, small, large | `staticalias.c:45` |
| **G** returned identity | `&inner` (same address every time) vs the caller's own pointer | `staticalias.c:32,35` |
| **H** entry-point interleaving | `static_alias` only / `driver` only / `static_alias` then `driver` / `driver` then `static_alias` / interleaved | shared `inner` |

Build configurations: `Cargo.toml` has no `[features]`, so there is exactly one.

## Rows

Each row is checked off only after **many randomized inputs** (fixed seed,
`SplitMix64`) pass for it, comparing the C `.so` and the Rust `.so`. For
`static_alias` every call compares three observables: the **returned pointer's
identity class** (is it the argument, or the library's static?), the **`int` at
the returned pointer**, and **`*outer` after the call**. For `driver` the whole
**stdout byte stream** is compared (fd-1 redirection + `fflush`).

Every row that needs a virgin `inner` gets a **private copy of both `.so` files**
so `dlopen` produces a fresh data segment.

| # | entry point(s) | configuration (options set + input shape) | ✅ |
|---|----------------|-------------------------------------------|----|
| 1 | `static_alias` | fresh state, single call, `*outer` randomized over the full `i32` range (axes A both, D all, E=1) | [x] |
| 2 | `static_alias` | fresh state, single call, `*outer == 1` — exact `== inner` boundary, then-branch (A=then, D=`==inner`) | [x] |
| 3 | `static_alias` | fresh state, single call, `*outer == 0` — one below `inner`, else-branch (A=else, D=0) | [x] |
| 4 | `static_alias` | fresh state, single call, `*outer` random **negative** ⇒ always else-branch, returns own pointer (A=else, B=caller, G=arg) | [x] |
| 5 | `static_alias` | fresh state, single call, `*outer` random **large positive** ⇒ then-branch, returns static (A=then, G=static) | [x] |
| 6 | `static_alias` | fresh state, single call, `*outer` ∈ {`INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `2`, `INT_MAX-1`, `INT_MAX`} exhaustively (D extremes) | [x] |
| 7 | `static_alias` | fresh state, **2 calls**, 2nd call re-passes the *returned* pointer (B: may become `&inner`; the self-alias shape) (E=2, B both) | [x] |
| 8 | `static_alias` | fresh state, **many (64) calls**, always re-feeding the returned pointer — reproduces `driver`'s chain by hand at the low level (C advanced, E=many, G alternating) | [x] |
| 9 | `static_alias` | fresh state, **many (64) calls**, always passing a *fresh caller-owned* `int` with a new random value each call, so `inner` advances while `*outer` does not (C advanced, B=caller) | [x] |
| 10 | `static_alias` | fresh state, many calls alternating between the returned pointer and a fresh random caller-owned `int` (B alternating, H n/a) | [x] |
| 11 | `static_alias` | state deliberately advanced first (one big positive call ⇒ large `inner`), then random `*outer` ⇒ else-branch dominates (C advanced, A=else) | [x] |
| 12 | `static_alias` | **returned-pointer identity stability**: `&inner` returned by call *n* equals `&inner` returned by call *m*, and differs from any caller pointer (G) | [x] |
| 13 | `driver` | fresh state, `iterations == 0`, random `initial_value` ⇒ empty stdout (F=0) | [x] |
| 14 | `driver` | fresh state, `iterations == 1`, random `initial_value` (F=1, A both depending on value) | [x] |
| 15 | `driver` | fresh state, `iterations == 2`, random `initial_value` — first call may take else-branch, second then-branch (F=2, the branch flip) | [x] |
| 16 | `driver` | fresh state, random `initial_value` **≥ 1** (then-branch first ⇒ `running_sum` becomes `&inner` and stays there, doubling), `iterations` ∈ 1..24 (A=then, G=static, overflow reached) | [x] |
| 17 | `driver` | fresh state, random `initial_value` **< 1** (else-branch first ⇒ `running_sum` stays on the stack slot and climbs by `inner` until it catches up, then flips) (A=else→then) | [x] |
| 18 | `driver` | fresh state, `initial_value` = small negative with `iterations` large enough to cross the flip point several times (F=large, E=many) | [x] |
| 19 | `driver` | fresh state, `initial_value` ∈ {`INT_MIN`, `-1`, `0`, `1`, `INT_MAX`} × `iterations` ∈ {1,2,3,8,40} — cross-product of the extremes (D extremes × F) | [x] |
| 20 | mixed: `static_alias` → `driver` | fresh state, N random `static_alias` calls advance `inner`, *then* `driver` runs on that non-virgin state (H, C advanced) | [x] |
| 21 | mixed: `driver` → `static_alias` | fresh state, `driver` advances `inner`, then random `static_alias` calls observe the carried-over state (H, C advanced) | [x] |
| 22 | mixed: interleaved | fresh state, randomized interleaving of `static_alias` calls and `driver` calls, comparing pointer class, values **and** stdout at every step (H full) | [x] |
| 23 | `driver` | fresh state, `iterations` large (4096) ⇒ long stdout stream, checks buffering/flush parity of the `printf` path (F=large) | [x] |
| 24 | both | **state persistence within one load**: two consecutive `driver` calls on the same handle — the 2nd starts from the `inner` the 1st left (C) | [x] |

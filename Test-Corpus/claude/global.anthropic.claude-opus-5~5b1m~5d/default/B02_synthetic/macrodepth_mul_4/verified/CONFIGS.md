# CONFIGS.md — configuration-surface table (Phase B)

## Derivation

Axes were extracted mechanically from the C source, not guessed:

```sh
grep -nE '#ifn?def|#define|switch|case |if *\(|OP_FN|STEP_|INIT_|REP[0-7]|REPEAT|CHOOSE_REP' \
     c_src/src/mdmacros.h c_src/src/mdcore.c c_src/src/mdmain.c
grep -n 'set(' c_src/CMakeLists.txt      # -> OP (default add), REPEAT (default 5)
```

### Axis 1 — build-time `OP` (CMake `-DOP=`, Cargo feature `add`/`sub`/`mul`)

`OP` is token-pasted into four *independent* macro families, so it toggles four
distinct pieces of state:

| macro | `add` | `sub` | `mul` |
|-------|-------|-------|-------|
| `OP_FN(OP)`   (`CAT(op_, op)`) | calls `op_add` | calls `op_sub` | calls `op_mul` |
| `STEP_OP`     (`CAT(STEP_, op)`) | `acc += i` | `acc -= i` | `acc *= (i+1)` |
| `INIT_FOR(OP)`(`CAT(INIT_, op)`) | `0` | `0` | **`1`** |
| `STR(OP)` → `G_OP_NAME`, `OP_FN(OP)` → `G_OP` | `"add"`, `&op_add` | `"sub"`, `&op_sub` | `"mul"`, `&op_mul` |

`OP` **unset** is a 4th distinct configuration (`#ifndef OP → add`), reached in
Rust with `--no-default-features` and no OP feature.

### Axis 2 — build-time `REPEAT` (CMake `-DREPEAT=`, Cargo feature `0`..`7`)

`REPEAT` selects `CHOOSE_REP(REPEAT)` → `REP<REPEAT>`, the *statically unrolled*
step chain used by `helper_call` and by `main`. Distinct branches: `REP0`
(expands to **nothing** — accumulator stays at `INIT`), `REP1` … `REP7`.
`REPEAT` **unset** is a 9th configuration (`#ifndef REPEAT → 5`).
`REPEAT` also feeds `use_generated(REPEAT)` from `main`, where `REPEAT == 7`
crosses `DISPATCH_REP`'s `case 6` boundary into `default:` — an interaction
between the two axes that only shows up at `REPEAT=7`.

### Axis 3 — public entry points (from `mdmacros.h`'s `extern` declarations)

Lowest level first: `op_add`, `op_sub`, `op_mul` (leaf ops) → `G_OP` /
`G_OP_NAME` (globals initialised by macro expansion at load time) →
`helper_ptr` (indirect call) → `helper_call` (op + unrolled `RUN_LOOP`) →
`use_generated` (the `static`, macro-generated `accum_<OP>` behind
`DISPATCH_REP`) → `main` (composes all five plus `atoi`).
Every one of these is exercised **directly** through `dlsym`, not only through
the `main`/one-shot path.

### Axis 4 — input shape

* `int a, b`: `0`, `±1`, small randoms, large randoms, `INT_MIN`, `INT_MAX`,
  `INT_MIN+1`, `INT_MAX-1`, and sign combinations (`+/+`, `+/-`, `-/+`, `-/-`).
* `int n` for `use_generated`: `0` (empty unroll), `1` (single step), `2..5`
  (many), `6` (last `case`), `7` (first `default`), `>7`, negatives.
* `main` argv shape: `argc<3`, `argc==3`, `argc>3`; decimal / signed /
  whitespace-padded / non-numeric / overflowing numeric text.
* Observable outputs compared byte-for-byte: the returned `int` **and** the
  exact bytes each function `printf`s to stdout.

### Build configurations (cross product of axes 1 × 2 — all 36 are checked)

`../run_all.sh` builds a C `.so` + C executable for each and runs the whole
Phase B/C suite against the matching Rust feature combination.

| | REPEAT unset | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|--|--|--|--|--|--|--|--|--|--|
| **OP unset** | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] |
| **add** | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] |
| **sub** | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] |
| **mul** | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] |

## Configuration-surface table

Each row is run under **every one of the 36 build configurations above**, with
many randomized inputs (fixed seed `0x5DEECE66D`, SplitMix64 PRNG, 512
iterations per randomized row) unless the row is exhaustive by construction.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `op_add` | leaf op, independent of `OP`/`REPEAT`; `a,b` = exhaustive small grid `[-4..4]²` (81 pairs) | [x] |
| 2 | `op_add` | `a,b` = 512 randomized full-range `int`s (all four sign combinations) | [x] |
| 3 | `op_add` | `a,b` ∈ boundary set {`0`,`1`,`-1`,`INT_MIN`,`INT_MIN+1`,`INT_MAX`,`INT_MAX-1`}² (49 pairs, incl. overflow) | [x] |
| 4 | `op_sub` | exhaustive small grid `[-4..4]²` | [x] |
| 5 | `op_sub` | 512 randomized full-range `int`s | [x] |
| 6 | `op_sub` | boundary-set cross product (49 pairs, incl. `INT_MIN - 1`) | [x] |
| 7 | `op_mul` | exhaustive small grid `[-4..4]²` | [x] |
| 8 | `op_mul` | 512 randomized full-range `int`s | [x] |
| 9 | `op_mul` | boundary-set cross product (49 pairs, incl. `INT_MIN * -1`, `INT_MAX * INT_MAX`) | [x] |
| 10 | `G_OP_NAME` | read the exported `const char *` and compare the NUL-terminated bytes; must be `"add"`/`"sub"`/`"mul"` per the `OP` axis | [x] |
| 11 | `G_OP` | read the exported function pointer; assert it is identical to that `.so`'s own `op_<OP>` symbol address (checks `OP_FN(OP)` expanded to the right identifier in both) | [x] |
| 12 | `G_OP` | invoke through the pointer: exhaustive small grid `[-4..4]²` | [x] |
| 13 | `G_OP` | invoke through the pointer: 512 randomized full-range `int`s + boundary set | [x] |
| 14 | `helper_ptr` | indirect call via a local `int(*fp)(int,int)`; exhaustive small grid `[-4..4]²`; compares return value **and** the `helper.ptr=%d\n` stdout bytes | [x] |
| 15 | `helper_ptr` | 512 randomized full-range `int`s; return value + stdout bytes | [x] |
| 16 | `helper_ptr` | boundary set (49 pairs), incl. overflowing ops; return value + stdout bytes | [x] |
| 17 | `helper_call` | `OP_FN` result + `RUN_LOOP(OP, acc, REPEAT)` unrolled accumulator; exhaustive small grid `[-4..4]²`; compares return **and** `helper.call=%d helper.acc=%d\n` bytes (the `acc` half is the `REPEAT` axis, incl. `REPEAT=0` where `REP0` is empty) | [x] |
| 18 | `helper_call` | 512 randomized full-range `int`s; return + stdout bytes | [x] |
| 19 | `helper_call` | boundary set (49 pairs) — `r + acc` overflows for `INT_MAX`-ish `r`; return + stdout bytes | [x] |
| 20 | `use_generated` | `DISPATCH_REP` in-range: exhaustive `n ∈ {0,1,2,3,4,5,6}` (every `case`), incl. `n=0` empty `REP0`; return + `gen.acc=%d\n` bytes | [x] |
| 21 | `use_generated` | `n` = `REPEAT` itself (the value `main` passes) — the axis-1×axis-2 interaction, and the `REPEAT=7 → default:` case | [x] |
| 22 | `use_generated` | `n` ∈ {`7`,`8`,`9`,`100`,`-1`,`-2`,`INT_MIN`,`INT_MAX`} → `default:` arm; return + stdout bytes | [x] |
| 23 | `use_generated` | 512 randomized full-range `int` `n` (mostly `default:`, biased to include the `0..8` window); return + stdout bytes | [x] |
| 24 | ordered composition `helper_call` → `helper_ptr` → `use_generated` → `G_OP` on the *same* loaded handle | replicates `main`'s call sequence directly on the low-level API, capturing **all** stdout in order, so interleaving/buffering of the composed pipeline is compared too; 128 randomized `(a,b)` | [x] |
| 25 | repeated invocation | each entry point called 64× in a row with different inputs on one handle — checks there is no hidden per-call state (C has none: `acc` is a local, `G_OP`/`G_OP_NAME` are never written) | [x] |
| 26 | `main` (`driver` executable) | `argc == 3`, small decimal args; compares stdout bytes, stderr bytes and exit status | [x] |
| 27 | `main` (`driver` executable) | `argc == 3`, 128 randomized full-range decimal args (incl. negatives / `INT_MIN` / `INT_MAX`), so `summary=` overflow-wrapping is compared | [x] |
| 28 | `main` (`driver` executable) | `argc == 3` with `atoi`-edge argument text: `""`, `"abc"`, `"12abc"`, `" 7 "`, `"+5"`, `"-0"`, `"0x10"`, `"007"`, `"2147483648"`, `"-2147483649"`, `"99999999999999999999"` | [x] |
| 29 | `main` (`driver` executable) | `argc > 3` (extra args ignored) and `argc < 3` (usage + status 2) | [x] |
| 30 | symbol surface | `nm -D` C-vs-Rust parity re-checked in this configuration | [x] |

## Status

All 30 rows pass, across randomized inputs, under **all 36 build configurations**
(`../run_all.sh` → 36/36 PASS; 53 tests per configuration = 1908 test executions).

Per-configuration test counts: `tests/configs.rs` 26, `tests/errors.rs` 20,
`tests/valid_main.rs` 4, `tests/symbols.rs` 3.

### Divergences found by this phase

* Row 11/14 + `ERRORS.md` row 13 exposed that `helper_ptr` in Rust read the
  global `G_OP` where the C token-pastes `op_<OP>` directly — fixed.
* The same row exposed that Rust's `G_OP`/`G_OP_NAME` were emitted read-only
  (`.data.rel.ro`) while C's are writable (`.data`) — fixed with `static mut`.

### Negative control

Running the Rust `mul,7` build against the C `sub/3` `.so` produces 34 failing
assertions, confirming the rows actually discriminate between configurations.

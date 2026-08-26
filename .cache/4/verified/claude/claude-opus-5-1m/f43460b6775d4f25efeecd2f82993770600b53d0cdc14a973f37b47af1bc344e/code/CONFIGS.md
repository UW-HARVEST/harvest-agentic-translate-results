# CONFIGS.md — configuration-surface table (Phase B)

## Build-time configuration

`Cargo.toml` declares `[features] default = []` and **no other features**;
`c_src/CMakeLists.txt` has no `option()`, no `add_definitions`, no
`target_compile_definitions` and `lib.c`/`main.c` contain **no `#ifdef`** other
than header include guards. The complete set of valid feature combinations is
therefore:

| # | feature combination | cargo invocation |
|---|---------------------|------------------|
| F1 | *(none — `default` is empty)* | `cargo check/test --no-default-features` |
| F2 | `default` (identical to F1, the `default` list is empty) | `cargo check/test` |

Both are verified by `./check_features.sh`.

## Runtime configuration axes (derived from the C branches)

| axis | values the C actually distinguishes | source |
|---|---|---|
| `operation` | `0`, `1`, `2`, `3`, anything else | `lib.c:56` `switch (operation)` |
| `param` | for `operation == 1` it is `logic_op`: `0` AND, `1` OR, `2` XOR, `3` NAND, anything else. **Ignored** for operations 0, 2, 3. | `lib.c:77`, `lib.c:171` |
| `length` | `0`; `1`, `2` (below the `< 3` gate); `3`; `4..10`; `11..31`; `32`; `> 32` (clamped); `1023` (`MAX_INPUT_SIZE - 1`) | `lib.c:52,59,70,83`, `lib.c:360,365,370`, `main.c:34` |
| byte values | `'y'`, `'Y'` → true; `'n'`, `'N'` → false; every other byte → false (incl. `0x00`, `0x80..0xFF`) | `lib.c:108-114` |
| content shape | all-true, all-false, exactly-one-true, exactly-one-false, strictly alternating, runs of >= 3 true, runs of >= 4 equal, mixed | `lib.c:257-308`, `lib.c:339-357` |
| buffer mutation | `operation == 3` rewrites `[0, length)` in place through a `bool *` alias | `lib.c:322-326` |
| entry point | `process_decisions` (the only exported library symbol) and the `driver` executable (`main`: 3 stdin lines → 1 stdout line) | `nm -D`, `main.c` |

## Configuration rows

Each row is exercised against **both** `.so`s with many randomized inputs
(fixed seed, deterministic xorshift PRNG in `tests/differential.rs`), and the
returned `int` **plus the full post-call buffer** are compared byte-for-byte.

### Operation 0 — `apply_permissions` (lowest-level path: read/write/execute tree)

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| C1 | `process_decisions` | `op=0`, `length=3`, exhaustive over all 8 `y`/`n` triples | [x] |
| C2 | `process_decisions` | `op=0`, `length=3`, exhaustive over all `4^3 = 64` combinations of `{'y','Y','n','N'}` (case handling) | [x] |
| C3 | `process_decisions` | `op=0`, `length=3`, random bytes `0x00..0xFF` in all three positions (invalid → false) | [x] |
| C4 | `process_decisions` | `op=0`, `length` in `3..=64` with random content — trailing bytes beyond index 2 must be ignored | [x] |
| C5 | `process_decisions` | `op=0`, `param` swept over `{-2,-1,0,1,2,3,4,INT_MIN,INT_MAX}` — must be ignored | [x] |

### Operation 1 — `evaluate_conditions` (all four logic operators)

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| C6 | `process_decisions` | `op=1, param=0` (AND), exhaustive over all 8 `y`/`n` triples (hits `100`, `50/51/52`, `10/11/12`, `0`) | [x] |
| C7 | `process_decisions` | `op=1, param=1` (OR), exhaustive over all 8 triples (hits `100+count`, `0`) | [x] |
| C8 | `process_decisions` | `op=1, param=2` (XOR), exhaustive over all 8 triples (hits `1/2/3/7`, `90`, `0`) | [x] |
| C9 | `process_decisions` | `op=1, param=3` (NAND), exhaustive over all 8 triples (hits `200`, `150/151/152`, `100`, `0`) | [x] |
| C10 | `process_decisions` | `op=1`, all four `param` values × all `4^3` case combinations of `{'y','Y','n','N'}` | [x] |
| C11 | `process_decisions` | `op=1`, all four `param` values × random arbitrary bytes | [x] |
| C12 | `process_decisions` | `op=1`, `length` in `3..=64`, random content — bytes past index 2 ignored | [x] |

### Operation 2 — `configure_flags` (bitmask + pattern rules)

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| C13 | `process_decisions` | `op=2`, `length=1`, both `"y"` and `"n"` (`count == 1` boundary: all-false → `0`, all-true → `1001`) | [x] |
| C14 | `process_decisions` | `op=2`, all-false input for every `length` in `1..=40` (`special_count == 0`) | [x] |
| C15 | `process_decisions` | `op=2`, all-true input for every `length` in `1..=40` (`special_count == count` → `1000+count`, incl. clamp at 32) | [x] |
| C16 | `process_decisions` | `op=2`, exactly one true at every index `i` for every `length` in `1..=40` (`100+i`) | [x] |
| C17 | `process_decisions` | `op=2`, exactly one false at every index `i` for every `length` in `1..=40` (`200+i`) | [x] |
| C18 | `process_decisions` | `op=2`, strictly alternating starting `y` and starting `n`, every `length` in `1..=40` (`500+special_count`) | [x] |
| C19 | `process_decisions` | `op=2`, inputs engineered to have a maximal true-run of exactly `k` for `k` in `1..=10` (`300+max_consecutive` / fallthrough `special_count`) | [x] |
| C20 | `process_decisions` | `op=2`, `length` exactly `31`, `32`, `33` (clamp boundary), exhaustive-ish random content | [x] |
| C21 | `process_decisions` | `op=2`, `length` in `33..=64` — result must equal the same call on the 32-byte prefix | [x] |
| C22 | `process_decisions` | `op=2`, `length=1023` (`MAX_INPUT_SIZE-1`), random content | [x] |
| C23 | `process_decisions` | `op=2`, exhaustive over all `2^n` `y`/`n` patterns for `n` in `1..=12` (complete coverage of the rule interaction) | [x] |
| C24 | `process_decisions` | `op=2`, random arbitrary bytes (`0x00..0xFF`) at random lengths `1..=64` | [x] |
| C25 | `process_decisions` | `op=2`, `param` swept over `{-1,0,1,2,3,99}` — must be ignored | [x] |

### Operation 3 — `validate_sequence` (rules + length tiers + in-place rewrite)

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| C26 | `process_decisions` | `op=3`, `length=1`, all four of `{'y','Y','n','N'}` (`transitions == 0` → `1`, or `-10`) | [x] |
| C27 | `process_decisions` | `op=3`, exhaustive over all `2^n` `y`/`n` patterns for `n` in `1..=14` (covers rules 1-3 and the `len<=3`, `4..10`, `>=11` tiers together) | [x] |
| C28 | `process_decisions` | `op=3`, `length` exactly `3` and `4` (tier boundary `len <= 3` vs `len <= 10`), exhaustive patterns | [x] |
| C29 | `process_decisions` | `op=3`, `length` exactly `10` and `11` (tier boundary `len <= 10` vs long), random valid-ish patterns | [x] |
| C30 | `process_decisions` | `op=3`, medium tier `4..=10`: patterns tuned to `transitions < len/3`, `transitions > len/2`, and in between (`20`/`30`/`25`) | [x] |
| C31 | `process_decisions` | `op=3`, long tier `11..=64`: patterns tuned to `transitions < 3`, `transitions > len-3`, and in between (`40`/`50`/`45`). `40` turns out to be **unreachable** in the C — see the notes below — and both implementations are asserted never to return it. | [x] |
| C32 | `process_decisions` | `op=3`, case-mixed input using `{'y','Y','n','N'}` at random lengths `1..=40` | [x] |
| C33 | `process_decisions` | `op=3`, random arbitrary bytes (`0x00..0xFF`) at random lengths `1..=64` | [x] |
| C34 | `process_decisions` | `op=3`, `length=1023`, random content, plus `length` = 1024 / 4096 / 10000 (well beyond what `main` can supply) to stress the long-tier `size_t` arithmetic | [x] |
| C35 | `process_decisions` | `op=3`, **post-call buffer compared byte-for-byte** (the `bool *` aliasing rewrite) for every shape above, including early-return paths `-10`/`-11`/`-12` where the rewrite still happened | [x] |
| C36 | `process_decisions` | `op=3`, `param` swept over `{-1,0,1,2,3,99}` — must be ignored | [x] |

### Cross-cutting

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| C37 | `process_decisions` | Full random fuzz: `operation` in `-4..=8`, `param` in `-4..=8`, `length` in `0..=80`, uniformly random bytes — 200 000 cases | [x] |
| C38 | `process_decisions` | Repeated calls on the same buffer (statefulness check: `op=3` rewrites, then `op=0/1/2/3` re-run on the rewritten bytes — the `0x00`/`0x01` bytes now parse as false) | [x] |
| C39 | `driver` executable | End-to-end stdin → stdout/stderr/exit-code equality for every `operation`/`param`/decision-string shape above (C `c_src/build/driver` vs Rust `target/debug/driver`) | [x] |
| C40 | `driver` executable | `atoi` shapes for the operation/param lines: empty, whitespace, `+`/`-` signs, leading zeros, trailing garbage (`3abc`), pure garbage, overflow (`99999999999999999999`, `-99999999999999999999`) | [x] |
| C41 | `driver` executable | Decision line shapes: with/without trailing `\n`, empty line, embedded `\0`, CRLF, exactly 1023 bytes, longer than 1023 bytes (fgets split) | [x] |
| C42 | `process_decisions` | `length` FAR larger than the readable buffer, for the operations whose access pattern is bounded independently of `length`: ops 0/1 with only 3 readable bytes and ops 2 with only 32, at `length` = 3, 4, 32, 33, 1024, 65536, 2^40, `usize::MAX/2`, `usize::MAX-1`, `usize::MAX`; plus unknown operations, which never dereference at all. Well-defined in C; breaks a naive `from_raw_parts(ptr, length)`. | [x] |
| C43 | `driver` executable | **stdout is a pipe whose read end is closed** — a C `main` inherits the default `SIGPIPE` disposition and dies with signal 13. Raw wait status (code *and* signal) compared, for operations 0/1/2/3 and an invalid one. | [x] |
| C44 | `driver` executable | **stderr is a pipe whose read end is closed** and stdin is at EOF, so `main` takes each `fprintf(stderr, ...)` error path. Raw wait status compared. | [x] |
| C45 | `driver` executable | stdout and/or stderr **closed** (`1>&-`, `2>&-`, both) rather than a broken pipe: `printf`/`fprintf` fail with `EBADF`, which C ignores. Exit code compared. | [x] |

## Notes discovered while verifying

* **`validate_sequence` `return 40` is dead code.** In the long tier
  (`len >= 11`) the branch `if (transitions < 3) return 40;` (lib.c:372) is
  unreachable: rule 3 has already rejected any run longer than 3 equal values,
  so a surviving sequence of length `L` consists of at least `ceil(L/3)` runs and
  therefore has at least `ceil(L/3) - 1 >= ceil(11/3) - 1 == 3` transitions.
  Rows C27 and C31 assert that *neither* implementation ever produces `40`,
  exhaustively for `len` 11..=14 and over 16 000 randomized long inputs.
* **`apply_permissions`' `return 0` fallthrough is dead for `read && write`.**
  `permission_value` is provably `6` inside that branch, so the inner
  `if (permission_value == 6)` always fires and control never reaches lib.c:162
  via that path. Row `err_e9_*` pins the answer at `56`.
* **`configure_flags`' `flags` bitmask is computed but never read** by the C, so
  it cannot be observed; the Rust keeps the computation (including its
  `i < count && i < 32` shift guard) for fidelity.
* **Operation 3 mutates the caller's buffer.** This was a genuine divergence in
  the original translation (it built a private `Vec<bool>` instead of aliasing
  the caller's bytes) and is now fixed; row C35 and the whole-buffer comparison
  inside `assert_same` guard it.

## Profile / build-configuration coverage

| build | how it is verified |
|---|---|
| `dev` (debug, overflow checks **on**) | `cargo test --offline` (all four test binaries) |
| `release` (optimised, overflow checks **off**, `panic = "abort"`) | `cargo build --release`, then the debug test harness is pointed at the release artifacts via `DRIVER_RUST_SO=target/release/libdriver.so DRIVER_RUST_BIN=target/release/driver cargo test`. `cargo test --release` itself is impossible because the crate's `[profile.release] panic = "abort"` is incompatible with the unwinding test harness. |

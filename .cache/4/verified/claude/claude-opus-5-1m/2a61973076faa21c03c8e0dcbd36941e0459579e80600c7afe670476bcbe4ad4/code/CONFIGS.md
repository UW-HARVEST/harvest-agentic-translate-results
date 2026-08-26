# CONFIGS.md — configuration surface table (Phase B)

## Build-time axes

| axis | values | note |
|------|--------|------|
| cargo features | **none exist** (`Cargo.toml` has no `[features]`) → single combination | `cargo check/test --no-default-features` == default; both verified |
| CMake options | **none** (`add_executable` only, no `option()`, no `CMAKE_BUILD_TYPE` in the file) | reference build is unoptimised; the `.so` is built with the same flags + `-fPIC -shared` |
| `#ifdef` in C | **none** (only include guards + 3 function-like macros) | no conditional code paths |

So all rows below are exercised in the one and only build configuration; the
`scripts/run_all.sh` loop iterates the (single-element) feature-combination set
and additionally re-runs everything against the **debug** cdylib, where Rust's
integer-overflow checks are active (any non-wrapping arithmetic would panic).

## Runtime axes the C actually branches on

* `impl_id` (`run_engine` → `classify` / `process_stream`): `0` → a.c, `1` → b.c,
  **anything else** → lib.c `target` (`2`, `-1`, `3`, `INT_MIN`, `INT_MAX`, …).
* opcode (`switch (op)`): `0..=10` plus the `default` arm.
* per-opcode operand shapes: missing operand, negative operand, boundary
  operand (`k == n - ip`, `m == stack.len`, `times <= 0`).
* stack shape: empty / 1 element / many; `iv_peek` default substitution.
* `IntVec` capacity state: `cap == 0` (first alloc = 8), `len < cap`,
  `len == cap` (doubling), `need <= cap`, `need > cap`, overflowing `need`.
* `Program` ip state: `ip < n`, `ip == n`, `ip > n`, `n == 0`.
* value domain of the `target` functions: `code < 0`, and `code % 10` ∈
  {0, 1..3, 4..6, 7, 8..9} for lib.c; `((code>>2) ^ state_a) & 7` ∈ 0..7 for a.c;
  `(code ^ mask) % 8` ∈ 0..7 for b.c.
* **persistent state**: a.c `state_a` and b.c `flipflop` — pristine vs. warmed,
  and the parity of the number of internal `target` calls made so far.
* stream length: 0 / 1 / 2 / many / long enough to overflow `int` accumulators.
* `vm_print` shapes: empty vs non-empty stack (default `-777`), empty vs long
  trace, negative / huge trace values (`t & 25`), label contents, `steps` value.
* CLI: argv shapes × stdin shapes (see the last section).

## Rows

Every row is exercised with **many pseudo-random inputs from a fixed seed**
(`common::Rng`, xorshift64*), not a single hand-picked value, and asserts
byte/field equality of every observable (return value, `*out` values, `len`,
`cap`, `data == NULL`, full stack/trace contents, `steps`, and the exact bytes
written by `vm_print`).

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|-------------------------------------------|------|---|
| 1 | `target` | `code < 0`, incl. `INT_MIN` and random negatives | `cfg_target_all_branches` | [x] |
| 2 | `target` | `code >= 0`, `code % 10 == 0` | `cfg_target_all_branches` | [x] |
| 3 | `target` | `code % 10 ∈ 1..=3` | `cfg_target_all_branches` | [x] |
| 4 | `target` | `code % 10 ∈ 4..=6` | `cfg_target_all_branches` | [x] |
| 5 | `target` | `code % 10 == 7` | `cfg_target_all_branches` | [x] |
| 6 | `target` | `code % 10 ∈ 8..=9`, incl. `INT_MAX` | `cfg_target_all_branches` | [x] |
| 7 | `call_a_once` | first call on a pristine module (`state_a == 0`) | `cfg_call_a_once_stateful` | [x] |
| 8 | `call_a_once` | 2 000 chained calls with random `x` — `state_a` accumulates, every internal `target` call order matters | `cfg_call_a_once_stateful` | [x] |
| 9 | `call_a_once` | negative `x` (a.c `target` early-return path, `state_a` *not* updated) | `cfg_call_a_once_boundaries` | [x] |
| 10 | `call_a_once` | `x ∈ {INT_MIN, INT_MAX, ±2^k}` — `code<<1` overflow, `(x^0x55)+7` overflow | `cfg_call_a_once_boundaries` | [x] |
| 11 | `process_a_stream` | `n == 0` with `xs == NULL` | `err_zero_len`, `err_null_ptr_zero_len` | [x] |
| 12 | `process_a_stream` | `n == 1`, values chosen so the inner loop hits `continue` (even `t`), `break` (`t == 5`) and fall-through | `cfg_process_a_stream` | [x] |
| 13 | `process_a_stream` | `n ∈ 2..=64` random values (mixed sign, boundaries) | `cfg_process_a_stream` | [x] |
| 14 | `process_a_stream` | `n == 4096` (long stream, `size_t` accumulator grows past `INT_MAX`) | `cfg_process_a_stream` | [x] |
| 15 | `process_a_stream` | interleaved with `call_a_once` so both share `state_a` | `cfg_a_shared_state` | [x] |
| 16 | `call_b_once` | first call on a pristine module (`flipflop == 0`) | `cfg_call_b_once_stateful` | [x] |
| 17 | `call_b_once` | 2 000 chained calls with random `x` (4 internal `target` calls each → `flipflop` parity) | `cfg_call_b_once_stateful` | [x] |
| 18 | `call_b_once` | negative `x` and negative intermediates (`c ^ x < 0`) | `cfg_call_b_once_boundaries` | [x] |
| 19 | `call_b_once` | `x ∈ {INT_MIN, INT_MAX}` — `(x+9)^0x2222-17` overflow | `cfg_call_b_once_boundaries` | [x] |
| 20 | `process_b_stream` | `n == 0` with `xs == NULL` | `err_zero_len`, `err_null_ptr_zero_len` | [x] |
| 21 | `process_b_stream` | `n == 1`, values hitting `t == 6` (break), `t == 3` (continue) and the `acc*3^t` path | `cfg_process_b_stream` | [x] |
| 22 | `process_b_stream` | `n ∈ 2..=64` random values | `cfg_process_b_stream` | [x] |
| 23 | `process_b_stream` | `n == 1024` (`acc * 3` overflows `int` repeatedly) | `cfg_process_b_stream` | [x] |
| 24 | `process_b_stream` | interleaved with `call_b_once` so both share `flipflop` | `cfg_b_shared_state` | [x] |
| 25 | `iv_init`/`iv_push`/`iv_peek` | growth sequence: `cap` 0 → 8 → 16 → 32 … over 300 pushes, checking `len`/`cap`/contents after every push | `cfg_iv_growth_and_reserve` | [x] |
| 26 | `iv_reserve` | `need == 0`, `need < cap`, `need == cap`, `need == cap+1`, `need == 8/9/17`, on empty and populated vectors | `cfg_iv_growth_and_reserve` | [x] |
| 27 | `iv_reserve` + `iv_push` | reserve first, then push up to the reserved capacity (no re-alloc) | `cfg_iv_growth_and_reserve` | [x] |
| 27b | `iv_reserve` | **caller-chosen `cap`** (1,2,3,5,6,7,9,10,13,17,20,33,100,1000 — i.e. not a power of two, which the API itself can never produce) × `len ∈ {0,1,cap/2,cap}` × `need ∈ {0,1,cap-1,cap,cap+1,cap+2,2cap,2cap+1,3cap+1,8cap+3}`: the doubling loop starts at *that* `cap`, so the resulting `cap` is only observable this way | `cfg_iv_arbitrary_cap` | [x] |
| 27c | `iv_push` | same caller-chosen `cap`s, pushing past `len == cap` so the implied `iv_reserve(cap*2)` grows from a non-power-of-two capacity | `cfg_iv_arbitrary_cap` | [x] |
| 28 | `iv_pop`/`iv_peek` | pop to empty with `out != NULL` and `out == NULL`, then peek with several defaults | `cfg_iv_pop_peek` | [x] |
| 29 | `iv_free` + `iv_init` + `iv_push` | free a populated vector, re-init, re-push (`data == NULL`, `cap == 0` again) | `cfg_iv_free_reuse`, `err_double_free_and_reinit` | [x] |
| 30 | all `iv_*` | 5 000 random ops (push/pop/peek/reserve/free/init) driven by a seeded PRNG, comparing the full struct after every op | `cfg_iv_random_op_sequence` | [x] |
| 31 | `prog_init`/`prog_fetch` | `n == 0`, `n == 1`, `n == 37`: fetch to exhaustion and past it, checking `ip`, `*out`, return value each step | `cfg_prog_fetch_sequences` | [x] |
| 32 | `prog_fetch` | caller-mangled `ip`: `ip == n`, `ip == n+1`, `ip == SIZE_MAX` | `err_prog_fetch` | [x] |
| 33 | `vm_init`/`vm_trace`/`vm_free` | 500 traces (trace vector growth), then `vm_free` (both vectors NULL, `steps == 0`) | `cfg_vm_trace_and_free` | [x] |
| 34 | `vm_print` | empty stack → default `-777`; non-empty stack; `steps ∈ {0, 1, INT_MIN, INT_MAX}` | `cfg_vm_print_labels` | [x] |
| 35 | `vm_print` | trace values `0..=14` (the values the engine produces) and random/negative/huge values (`t & 25` alphabet) incl. `INT_MIN`, plus a 4 000-entry trace | `cfg_vm_print_trace_alphabet` | [x] |
| 36 | `vm_print` | labels: `""`, `"A:"`, `"EXT:"`, 300-byte label, label containing `%d`/`%s` (it is an argument, not the format) | `cfg_vm_print_labels` | [x] |
| 37 | `run_engine` | opcode `0` (push) — valid immediate, incl. `INT_MIN`/`INT_MAX`; `impl_id ∈ {0,1,2}` | `cfg_engine_opcodes_per_impl` | [x] |
| 38 | `run_engine` | opcode `1` (add) — 2 operands, incl. wrap-around overflow; all impls | `cfg_engine_opcodes_per_impl` | [x] |
| 39 | `run_engine` | opcode `2` (mul) — incl. overflow; all impls | `cfg_engine_opcodes_per_impl` | [x] |
| 40 | `run_engine` | opcode `3` (dup) — empty stack (peek default `0`) and non-empty; all impls | `cfg_engine_opcodes_per_impl` | [x] |
| 41 | `run_engine` | opcode `4` (drop) — non-empty; all impls | `cfg_engine_opcodes_per_impl` | [x] |
| 42 | `run_engine` | opcode `5` (classify+trace switch) — buckets `0`,`1`,`2`,`3/4` (fall-through) and `default`; all impls, empty and non-empty stack | `cfg_engine_opcodes_per_impl`, `cfg_engine_classify_buckets` | [x] |
| 43 | `run_engine` | opcode `6` — `cond == 0`; `cond != 0` with `k == 0`, `k` mid-program, `k == n-ip` (exact boundary) | `cfg_engine_op6_jumps` | [x] |
| 44 | `run_engine` | opcode `7` — `times ∈ {0, negative, 1, 2, 5}` × inner opcode that succeeds (`3`, `5`, `8`, `10`) or fails (`0`, `1`, `6`, `9`, `11`) | `cfg_engine_op7_repeat` | [x] |
| 45 | `run_engine` | opcode `7` nested (inner window is itself a `7`), and `7` right before end-of-program | `cfg_engine_op7_repeat` | [x] |
| 46 | `run_engine` | opcode `8` (classify, trace 13) — all impls, empty and populated stack | `cfg_engine_opcodes_per_impl` | [x] |
| 47 | `run_engine` | opcode `9` — `m == 0`, `m == 1`, `m == stack.len` (second pop round fully fails), `m == len/2` (second round partially succeeds → mixed `tmp[]`), all impls | `cfg_engine_op9_stream_double_pop` | [x] |
| 48 | `run_engine` | opcode `10` (halt) mid-program: remaining words ignored | `cfg_engine_opcodes_per_impl` | [x] |
| 49 | `run_engine` | 1 200 random programs (len 1..24) × `impl_id ∈ {0,1,2}` | `cfg_engine_random_programs` | [x] |
| 50 | `run_engine` | 120 long random programs (len up to 120) × `impl_id ∈ {0,1,2}` | `cfg_engine_long_programs` | [x] |
| 51 | `run_engine` | **VM reuse**: several `run_engine` calls on the *same* VM (stack/trace/steps accumulate) | `cfg_engine_vm_reuse` | [x] |
| 51b | `run_engine` | VM whose **stack the caller pre-built** with an arbitrary `cap` (1,2,3,5,9,10,17) and `len` (0,1,cap), then 20 random programs per shape, per impl — the engine's pushes grow that buffer | `cfg_engine_caller_supplied_vm` | [x] |
| 52 | `run_engine` | `impl_id ∉ {0,1}`: `2`, `3`, `-1`, `7`, `INT_MIN`, `INT_MAX` — all must behave identically (lib.c `target` path) | `cfg_engine_impl_id_variants`, `err_impl_id_out_of_range` | [x] |
| 53 | `run_engine` | **state carried across runs**: repeated runs of the same program on one library instance (`state_a`/`flipflop` evolve) | `cfg_engine_state_across_runs` | [x] |
| 54 | `run_engine` + `vm_print` | full pipeline exactly as `main` does it: run impl 0/1/2 then print each VM, on random programs | `cfg_engine_pipeline_like_main` | [x] |
| 55 | `run_engine` | program built only from *raw random ints* (no opcode bias) — mostly `default → 99`, exercises the unknown-opcode path early | `cfg_engine_random_garbage` | [x] |
| 56 | `main` (exe) | argv: none, `--help` (first/middle/last), `--stdin`, junk args, numeric edge cases (empty string, `" 12"`, `"12abc"`, `0x10`, `+-5`, `LONG_MAX+1`, `INT_MAX+1`), non-UTF-8 arg | `cli_fixed_cases`, `scripts/cli_diff.sh` | [x] |
| 57 | `main` (exe) | stdin: empty, `\n` only, multi-line, tabs/CR, no trailing newline, junk tokens, overflowing numbers, embedded NUL, line > 4096 bytes (fgets split), token split across two `fgets` chunks | `cli_stdin_shapes`, `scripts/cli_diff.sh` | [x] |
| 58 | `main` (exe) | 400 random programs on argv, and 100 random programs on stdin | `cli_random_programs` | [x] |
| 59 | `main` (via `.so`) | the exported `main` symbol called through `dlopen`/`dlsym` by a C loader: `argc == 0/argv == NULL`, `--help`, junk arg, empty arg, real programs | `err_main_symbol_via_dlopen` | [x] |
| 60 | `run_engine` | caller pre-set `steps` at/near `INT_MAX` (and negative) → `steps++` wrap, for every impl | `cfg_engine_steps_overflow` | [x] |
| 61 | both `.so` | `nm -D` symbol parity (19/19) and ABI layout of `IntVec`/`Program`/`VM` | `symbol_parity`, `abi_layout` | [x] |
| 62 | harness | private `.so` copies really do get pristine `static` state | `fresh_state_is_independent` | [x] |

## Mutation testing (evidence that the rows above have teeth)

Twelve deliberate single-line bugs were injected into `src/lib.rs`, rebuilt and
run through the whole suite (`cargo test --no-fail-fast`), then reverted:

| # | injected bug | failing tests |
|---|--------------|---------------|
| 1 | lib.c `target`: `m <= 3` → `m < 3` | 11 differential + 2 CLI |
| 2 | engine op 9: only ONE pop round instead of the C's two | 12 differential + 3 CLI |
| 3 | `iv_reserve`: start the doubling at 8 instead of `v->cap` | 2 differential (`cfg_iv_arbitrary_cap`, `cfg_engine_caller_supplied_vm`) |
| 4 | `process_a_stream`: drop the (always taken) `INT_MIN` clamp | 12 differential + 3 CLI |
| 5 | engine op 7: `vm_trace(vm, 12)` → `11` | 11 differential + 2 CLI |
| 6 | `vm_print`: `t & 25` → `t & 26` | 11 |
| 7 | b.c `target`: toggle `flipflop` *after* the `code < 0` check | 13 |
| 8 | engine op 5: bucket `3` no longer falls through to `4` | 11 |
| 9 | engine op 6: `>` → `>=` in the jump range check | 10 |
| 10 | engine: `vm->steps++` moved after the `switch` (error paths) | 26 |
| 11 | `classify` impl 1: drop the `+1` of `MAC_CALL` | 16 |
| 12 | `main`: accept arguments with trailing garbage (`*e == 0` check removed) | 2 CLI |

Mutation 3 initially **survived** — through the public API `cap` is always 0 or
`8·2^k`, so the doubling start value is invisible.  Rows 27b/27c/51b (which hand
the library a caller-allocated vector with e.g. `cap == 10`) were added for it,
and they now kill it.  The only mutants that are provably *not* killable are
semantically equivalent ones (e.g. `iv_push`'s `iv_reserve(cap*2)` →
`iv_reserve(cap+1)`, which always yields the same capacity).

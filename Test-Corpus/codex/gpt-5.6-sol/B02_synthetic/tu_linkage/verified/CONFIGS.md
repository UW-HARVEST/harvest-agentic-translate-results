# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` entries and `c_src/CMakeLists.txt` has no
options, compile definitions, or conditional source selection. The complete
set of valid feature combinations is therefore:

| # | Cargo invocation | C configuration |
|---|------------------|-----------------|
| 1 | `--no-default-features` (empty feature set) | default |

## Runtime and Input Configurations

Rows come from public headers, externally linked C definitions, and branch
conditions in those definitions. Backend IDs are integer modes: `0` selects A,
`1` selects B, and every other integer selects the external target.

| # | entry point(s) | configuration (options set + input shape) | passed |
|---|----------------|-------------------------------------------|--------|
| 1 | `target` | `code < 0` | [x] |
| 2 | `target` | nonnegative `code % 10 == 0` | [x] |
| 3 | `target` | nonnegative `code % 10` in `1..=3` | [x] |
| 4 | `target` | nonnegative `code % 10` in `4..=6` | [x] |
| 5 | `target` | nonnegative `code % 10 == 7` | [x] |
| 6 | `target` | nonnegative `code % 10` in `8..=9` | [x] |
| 7 | `call_a_once` | negative input; state-preserving negative branch in A target | [x] |
| 8 | `call_a_once` | nonnegative input; A state mutation and indirect/macro calls | [x] |
| 9 | `call_a_once` | repeated calls; persistent `state_a` affects later results | [x] |
| 10 | `process_a_stream` | null pointer with `n == 0` | [x] |
| 11 | `process_a_stream` | one element | [x] |
| 12 | `process_a_stream` | many elements exercising even-result `continue`, odd-result xor, and result-5 `break` | [x] |
| 13 | `process_a_stream` | accumulated value crosses the `0x7fffffffLL` clamp boundary if representable | [x] |
| 14 | `call_b_once` | negative input; flip-flop-dependent negative branch | [x] |
| 15 | `call_b_once` | nonnegative input; indirect/macro calls | [x] |
| 16 | `call_b_once` | repeated calls; persistent `flipflop` affects later results | [x] |
| 17 | `process_b_stream` | null pointer with `n == 0` | [x] |
| 18 | `process_b_stream` | one element | [x] |
| 19 | `process_b_stream` | many elements exercising result-6 `break`, result-3 `continue`, and update branch | [x] |
| 20 | `iv_init` | initialize a nonzero/dirty `IntVec` | [x] |
| 21 | `iv_free` | free an empty initialized vector | [x] |
| 22 | `iv_free` | free an allocated vector and reset all fields | [x] |
| 23 | `iv_reserve` | `need <= cap`, no allocation | [x] |
| 24 | `iv_reserve` | empty vector grows to minimum capacity 8 | [x] |
| 25 | `iv_reserve` | allocated vector doubles repeatedly to satisfy `need` | [x] |
| 26 | `iv_push` | push with spare capacity | [x] |
| 27 | `iv_push` | push into empty vector, causing capacity 8 allocation | [x] |
| 28 | `iv_push` | push at nonzero full capacity, causing doubling | [x] |
| 29 | `iv_pop` | nonempty vector and nonnull output | [x] |
| 30 | `iv_pop` | nonempty vector and null output; length still decrements | [x] |
| 31 | `iv_peek` | empty vector returns caller-provided default | [x] |
| 32 | `iv_peek` | nonempty vector returns top element | [x] |
| 33 | `prog_init` | null code with `n == 0`; fields initialized and `ip == 0` | [x] |
| 34 | `prog_init` | nonempty code; fields initialized and `ip == 0` | [x] |
| 35 | `prog_fetch` | available element is returned and `ip` increments | [x] |
| 36 | `vm_init` | initialize a nonzero/dirty `VM` | [x] |
| 37 | `vm_free` | free an empty initialized VM | [x] |
| 38 | `vm_free` | free populated stack and trace; reset vectors and steps | [x] |
| 39 | `vm_trace` | append to empty trace (allocation path) | [x] |
| 40 | `vm_trace` | append with spare capacity and across growth boundary | [x] |
| 41 | `vm_print` | empty stack/trace uses top default `-777` | [x] |
| 42 | `vm_print` | populated stack/trace; trace character index uses `entry & 25` | [x] |
| 43 | `run_engine` | empty/null program, any backend; EOF success | [x] |
| 44 | `run_engine` | opcode 0 push immediate, then EOF | [x] |
| 45 | `run_engine` | opcode 1 add with two stack values | [x] |
| 46 | `run_engine` | opcode 2 multiply with two stack values | [x] |
| 47 | `run_engine` | opcode 3 duplicates default 0 on empty stack | [x] |
| 48 | `run_engine` | opcode 3 duplicates an existing top value | [x] |
| 49 | `run_engine` | opcode 4 pops a populated stack | [x] |
| 50 | `run_engine` | opcode 5, backend 0, reachable bucket trace classes from `call_a_once` | [x] |
| 51 | `run_engine` | opcode 5, backend 1, reachable trace classes `3/4` and default from shifted/XOR `call_b_once` results | [x] |
| 52 | `run_engine` | opcode 5, external backend IDs (including negative and `2`), reachable bucket trace classes | [x] |
| 53 | `run_engine` | opcode 6 with zero condition; jump count is ignored | [x] |
| 54 | `run_engine` | opcode 6 with nonzero condition and `k == 0` | [x] |
| 55 | `run_engine` | opcode 6 with nonzero condition and in-range positive `k`; skipped opcodes are not run | [x] |
| 56 | `run_engine` | opcode 7 with `times <= 0`; following one-word instruction is skipped | [x] |
| 57 | `run_engine` | opcode 7 with one/many repeats of a successful one-word instruction | [x] |
| 58 | `run_engine` | opcode 7 inner instruction returns nonzero; trace 12 and break, outer call continues | [x] |
| 59 | `run_engine` | opcode 8 classify, backend 0 | [x] |
| 60 | `run_engine` | opcode 8 classify, backend 1 | [x] |
| 61 | `run_engine` | opcode 8 classify, external backend IDs | [x] |
| 62 | `run_engine` | opcode 9 with `m == 0`, backend 0/1/external | [x] |
| 63 | `run_engine` | opcode 9 with `m > 0` and stack length exactly `m`; second pop loop fails and retained temporary values are processed | [x] |
| 64 | `run_engine` | opcode 9 with `m > 0` and `m < stack.len < 2*m`; second pop loop partially overwrites temporaries | [x] |
| 65 | `run_engine` | opcode 9 with `m > 0` and `stack.len >= 2*m`, backend 0 | [x] |
| 66 | `run_engine` | opcode 9 with `m > 0` and `stack.len >= 2*m`, backend 1 | [x] |
| 67 | `run_engine` | opcode 9 with `m > 0` and `stack.len >= 2*m`, external backend | [x] |
| 68 | `run_engine` | opcode 10 halts successfully and ignores trailing words | [x] |
| 69 | `run_engine` | multiple opcodes end to end; stack, trace, steps, and backend state interact | [x] |
| 70 | `main` | `--help`; print usage, ignore trailing argv, and return success | [x] |
| 71 | `main` | argv bytecodes: valid, invalid/skipped, empty, and `strtol` overflow tokens; run all three backends and print results | [x] |
| 72 | `main` | `--stdin` with space/tab/CR/LF token shapes, accepted and skipped tokens, and multiple lines; run all three backends | [x] |
| 73 | `main` | `--stdin` at the 4095-byte `fgets` payload boundary and with embedded NUL truncation | [x] |

Row 13 uses `134,217,728` repetitions of value 5. From fresh A-backend state,
those inputs contribute an alternating `+17/+15`, making that even length cross
`INT_MAX` and execute the clamp.

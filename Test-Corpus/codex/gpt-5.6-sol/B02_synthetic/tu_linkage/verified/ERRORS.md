# Error Surface

Rows are distinct rejection branches mechanically identified in `util.c`,
`engine.c`, and `main.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `iv_reserve` | Growth loop still has `nc < need` when `nc > SIZE_MAX/2` | `false`; vector unchanged | [x] |
| 2 | `iv_reserve` | `realloc(v->data, nc*sizeof(int))` returns null | `false`; vector unchanged | [x] |
| 3 | `iv_push` | `len == cap` and delegated `iv_reserve` returns `false` | `false`; no element appended | [x] |
| 4 | `iv_pop` | `v->len == 0` | `false`; `out` is not written | [x] |
| 5 | `prog_fetch` | `p->ip >= p->n` | `false`; `out` is not written | [x] |
| 6 | `run_engine` | Opcode 0 has no following immediate | return `1` | [x] |
| 7 | `run_engine` | Opcode 1 cannot pop its first operand (`stack.len == 0`) | return `2` | [x] |
| 8 | `run_engine` | Opcode 1 pops one operand but cannot pop the second (`stack.len == 1`) | return `2`; first operand remains popped | [x] |
| 9 | `run_engine` | Opcode 2 cannot pop its first operand (`stack.len == 0`) | return `3` | [x] |
| 10 | `run_engine` | Opcode 2 pops one operand but cannot pop the second (`stack.len == 1`) | return `3`; first operand remains popped | [x] |
| 11 | `run_engine` | Opcode 4 executes with an empty stack | return `4` | [x] |
| 12 | `run_engine` | Opcode 6 has no following jump count | return `5` | [x] |
| 13 | `run_engine` | Opcode 6 has a jump count but cannot pop a condition | return `6` | [x] |
| 14 | `run_engine` | Opcode 6 has nonzero condition and `(size_t)k > p.n-p.ip` (including negative `k`) | return `7` | [x] |
| 15 | `run_engine` | Opcode 7 has no following repeat count | return `8` | [x] |
| 16 | `run_engine` | Opcode 7 consumes its repeat count and then `p.ip >= p.n` | return `9` | [x] |
| 17 | `run_engine` | Opcode 9 has no following stream length | return `10` | [x] |
| 18 | `run_engine` | Opcode 9 receives `m < 0` | return `11` | [x] |
| 19 | `run_engine` | Opcode 9 receives `(size_t)m > vm->stack.len` | return `11` | [x] |
| 20 | `run_engine` | Fetched opcode is outside `0..=10` | return `99` | [x] |
| 21 | `main` | No argv bytecodes are accepted and `--stdin` is absent or yields no accepted bytecodes (`code.len == 0`) | print `no program` to stderr and return `2` | [x] |

## Generic FFI Boundaries

The C functions do not reject null object pointers, null array pointers with a
nonzero length, or null `FILE *` values; they dereference them and have
undefined behavior (normally a process signal). Null array pointers with zero
length are valid for `process_a_stream`, `process_b_stream`, and `run_engine`.
`iv_pop` explicitly accepts a null `out` pointer when the vector is nonempty.
`prog_init` accepts a null `code` pointer, but it is usable safely only while no
fetch dereferences it.

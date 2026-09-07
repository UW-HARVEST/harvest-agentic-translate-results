# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection site in `c_src/src/lib.c`:

```
grep -n 'return|NULL|assert|Error|> 0|>= 0|< 4|> 4|?' c_src/src/lib.c
```

There are **no** `assert`s, no error enums, no `RETURN_ERROR` macros and no
`errno` use in this library. Every rejection is either a null-pointer guard, a
range check on `opcode`, or a `count` sign check. `malloc` failure is the only
allocation error path.

Rows are checked off (`[x]`) only when the differential test in
`tests/phase_c_errors.rs` passes for both the C and the Rust `.so`.
"expected C result" covers BOTH the return value AND the bytes written to
stdout, which the tests compare byte-for-byte.

| # | function | trigger (the exact invalid input/condition) | expected C result | ok |
|---|----------|---------------------------------------------|-------------------|----|
| 1 | `get_operation` | `opcode == -1` (fails `opcode >= 0`, `lib.c:76`) | returns `NULL`; no stdout | [x] |
| 2 | `get_operation` | `opcode == INT_MIN` (fails `opcode >= 0`) | returns `NULL`; no stdout | [x] |
| 3 | `get_operation` | `opcode == 4` (fails `opcode < 4`, one past valid range) | returns `NULL`; no stdout | [x] |
| 4 | `get_operation` | `opcode == INT_MAX` (fails `opcode < 4`) | returns `NULL`; no stdout | [x] |
| 5 | `get_operation` | `opcode` = arbitrary out-of-range enum-ish ints (`5,6,7,0x7FFF,-2,-100,0xDEAD`) crossing the FFI as `int` | returns `NULL`; no stdout | [x] |
| 6 | `execute_operation` | `func == NULL` (`lib.c:84`), `op_name` = `"XOR"` | prints `Error: Operation function pointer is NULL for XOR\n`; returns `0` | [x] |
| 7 | `execute_operation` | `func == NULL`, `op_name == NULL` (glibc `%s` prints `(null)`) | prints `Error: Operation function pointer is NULL for (null)\n`; returns `0` | [x] |
| 8 | `execute_operation` | `func == NULL`, `op_name == ""` (empty string) | prints `Error: Operation function pointer is NULL for \n`; returns `0` | [x] |
| 9 | `execute_operation` | `func == NULL` reached via `get_operation(bad)` (composed path) | prints the NULL error; returns `0` | [x] |
| 10 | `compute_checksum` | `values == NULL`, `count == 4` (fails `values != NULL`, `lib.c:102`) | skips body, returns `0` (`0 & MASK_LOWER`); no stdout | [x] |
| 11 | `compute_checksum` | `values == NULL`, `count == 0` (both sub-conditions fail) | returns `0`; no stdout | [x] |
| 12 | `compute_checksum` | `values == NULL`, `count == -1` | returns `0`; no stdout | [x] |
| 13 | `compute_checksum` | `values != NULL`, `count == 0` (fails `count > 0`) | returns `0`; no stdout | [x] |
| 14 | `compute_checksum` | `values != NULL`, `count == -1` (one step below valid range) | returns `0`; no stdout | [x] |
| 15 | `compute_checksum` | `values != NULL`, `count == INT_MIN` (extreme negative) | returns `0`; no stdout | [x] |
| 16 | `init_state` | `state == NULL` (`lib.c:117`) | prints `Error: state pointer is NULL in init_state\n`; returns void, writes nothing | [x] |
| 17 | `apply_operation` | `state == NULL`, `func` non-NULL (`lib.c:130`) | prints `Error: state pointer is NULL in apply_operation\n`; no mutation | [x] |
| 18 | `apply_operation` | `state == NULL` **and** `func == NULL` (state check wins, ordering matters) | prints only `Error: state pointer is NULL in apply_operation\n` | [x] |
| 19 | `apply_operation` | `state` valid, `func == NULL` (`lib.c:135`) | prints `Error: operation function pointer is NULL in apply_operation\n`; `accumulator`/`operation_count` unchanged | [x] |
| 20 | `checkshift` | `malloc(sizeof(ComputeState))` returns `NULL` (`lib.c:150`) | prints the two banner lines then `Error: Failed to allocate memory for state\n`; returns `-1` | [x] (see note) |

## Note on row 20

`malloc(sizeof(ComputeState))` (12 bytes) cannot be made to fail
deterministically from outside the library — both implementations call the
process allocator. The row is therefore covered structurally: the test asserts
both `.so` images contain the byte string `Error: Failed to allocate memory for
state` and every other diagnostic/format string.

**This row found a real divergence.** In the original Rust the release build's
LLVM recognised the `malloc`/`free` pair as removable, deleted the allocation
**and with it the `state == NULL` branch**, so no `malloc` call and no
allocation-failure message existed in `libcheckshift_lib.so`. Had `malloc`
failed, C would print the message and `return -1` while Rust would have carried
on and returned a computed value. Fixed by wrapping the allocation in
`core::hint::black_box`, which keeps the pointer opaque to the optimizer;
`objdump -d --disassemble=checkshift` now shows the `malloc` and `free` calls
and the message is present in both images.

Note when reading `.rodata`: both gcc and rustc rewrite a newline-terminated
`printf` with no conversions into `puts`, which strips the trailing newline from
the stored string. The test searches for each message both with and without it.

## Generic FFI-boundary boundaries additionally covered (`tests/phase_c_errors.rs`)

* Null pointers for every pointer parameter (`values`, `state`, `op_name`,
  `func`) individually and in combination.
* Zero and oversized lengths for `count`: `0`, `-1`, `INT_MIN`, `1..=4`,
  `5`, `1000`, `INT_MAX` (the `count > 4 ? 4 : count` clamp).
* Values one step past the documented valid `opcode` range: `-1` and `4`.
* Out-of-range "enum" values for `opcode` passed as raw `int` across FFI —
  C enums/opcodes accept any `int`, so `INT_MIN`, `-2`, `5`, `0xDEAD`,
  `INT_MAX` are real inputs both sides must reject identically.
* Function pointers *from the other library* passed into `execute_operation`
  and `apply_operation` (C op into Rust entry point and vice versa).

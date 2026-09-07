# Configuration surface

Mechanically derived from every `if` and `switch` in `c_src/src/lib.c`, plus
the input shapes of all 12 exported entry points. Arithmetic randomization is
restricted to defined C signed-integer operations; C overflow cases are not C
semantics and therefore are not valid configurations.

For `perform_computation_with_history`, the four history shapes are:

- **null**: `*history == NULL`, causing allocation and count reset;
- **partial**: allocated history with count `0`;
- **last**: allocated history with count `9`;
- **full**: allocated history with count `10` or greater, causing no write.

For `mathop`, each operation-pair row includes both `is_valid` branches
(`param1 % 128` inside and outside `'1'..='5'`) and repeated randomized calls
that observe history counts `2, 4, 6, 8, 10, 10...`. The `default` switch class
is randomized over all reachable out-of-range values (`-3..=0`) so the
priority differences are also covered.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `is_valid_operation` | character is NUL | [x] |
| 2 | `is_valid_operation` | non-NUL character below `'1'` (including negative signed `char`) | [x] |
| 3 | `is_valid_operation` | character in `'1'..='5'`, including both boundaries | [x] |
| 4 | `is_valid_operation` | character above `'5'` | [x] |
| 5 | `get_operation_priority` | every valid enum value `1..=5` and randomized integer operands | [x] |
| 6 | `add_operation` | randomized negative/zero/positive operands; ignored third argument varied | [x] |
| 7 | `multiply_operation` | randomized safe negative/zero/positive operands; ignored third argument varied | [x] |
| 8 | `subtract_operation` | randomized negative/zero/positive operands; ignored third argument varied | [x] |
| 9 | `divide_operation` | nonzero divisor; all sign combinations and exact/non-exact quotients | [x] |
| 10 | `modulo_operation` | nonzero divisor; all sign combinations and zero/nonzero remainders | [x] |
| 11 | `select_operation`, returned callback | `OP_ADD` | [x] |
| 12 | `select_operation`, returned callback | `OP_MULTIPLY` | [x] |
| 13 | `select_operation`, returned callback | `OP_SUBTRACT` | [x] |
| 14 | `select_operation`, returned callback | `OP_DIVIDE` | [x] |
| 15 | `select_operation`, returned callback | `OP_MODULO` | [x] |
| 16 | `select_operation`, returned callback | out-of-range enum takes `default` and returns add callback | [x] |
| 17 | `get_computation_timestamp` | no arguments; current `time_t` shifted right by 29 | [x] |
| 18 | `allocate_results` | count `1`; allocation is zero-initialized | [x] |
| 19 | `allocate_results` | count `2..=10`; allocation is zero-initialized | [x] |
| 20 | `allocate_results` | count above history capacity (`11..=64`); allocation remains zero-initialized | [x] |
| 21 | `perform_computation_with_history` | add × null history | [x] |
| 22 | `perform_computation_with_history` | add × partial history | [x] |
| 23 | `perform_computation_with_history` | add × last-slot history | [x] |
| 24 | `perform_computation_with_history` | add × full history | [x] |
| 25 | `perform_computation_with_history` | multiply × null history | [x] |
| 26 | `perform_computation_with_history` | multiply × partial history | [x] |
| 27 | `perform_computation_with_history` | multiply × last-slot history | [x] |
| 28 | `perform_computation_with_history` | multiply × full history | [x] |
| 29 | `perform_computation_with_history` | subtract × null history | [x] |
| 30 | `perform_computation_with_history` | subtract × partial history | [x] |
| 31 | `perform_computation_with_history` | subtract × last-slot history | [x] |
| 32 | `perform_computation_with_history` | subtract × full history | [x] |
| 33 | `perform_computation_with_history` | divide × null history | [x] |
| 34 | `perform_computation_with_history` | divide × partial history | [x] |
| 35 | `perform_computation_with_history` | divide × last-slot history | [x] |
| 36 | `perform_computation_with_history` | divide × full history | [x] |
| 37 | `perform_computation_with_history` | modulo × null history | [x] |
| 38 | `perform_computation_with_history` | modulo × partial history | [x] |
| 39 | `perform_computation_with_history` | modulo × last-slot history | [x] |
| 40 | `perform_computation_with_history` | modulo × full history | [x] |
| 41 | `perform_computation_with_history` | default/add fallback × null history | [x] |
| 42 | `perform_computation_with_history` | default/add fallback × partial history | [x] |
| 43 | `perform_computation_with_history` | default/add fallback × last-slot history | [x] |
| 44 | `perform_computation_with_history` | default/add fallback × full history | [x] |
| 45 | `mathop` | first add × second add; both validation branches; empty→full history | [x] |
| 46 | `mathop` | first add × second multiply; both validation branches; empty→full history | [x] |
| 47 | `mathop` | first add × second subtract; both validation branches; empty→full history | [x] |
| 48 | `mathop` | first add × second divide; both validation branches; empty→full history | [x] |
| 49 | `mathop` | first add × second modulo; both validation branches; empty→full history | [x] |
| 50 | `mathop` | first add × second default/add fallback; both validation branches; empty→full history | [x] |
| 51 | `mathop` | first multiply × second add; both validation branches; empty→full history | [x] |
| 52 | `mathop` | first multiply × second multiply; both validation branches; empty→full history | [x] |
| 53 | `mathop` | first multiply × second subtract; both validation branches; empty→full history | [x] |
| 54 | `mathop` | first multiply × second divide; both validation branches; empty→full history | [x] |
| 55 | `mathop` | first multiply × second modulo; both validation branches; empty→full history | [x] |
| 56 | `mathop` | first multiply × second default/add fallback; both validation branches; empty→full history | [x] |
| 57 | `mathop` | first subtract × second add; both validation branches; empty→full history | [x] |
| 58 | `mathop` | first subtract × second multiply; both validation branches; empty→full history | [x] |
| 59 | `mathop` | first subtract × second subtract; both validation branches; empty→full history | [x] |
| 60 | `mathop` | first subtract × second divide; both validation branches; empty→full history | [x] |
| 61 | `mathop` | first subtract × second modulo; both validation branches; empty→full history | [x] |
| 62 | `mathop` | first subtract × second default/add fallback; both validation branches; empty→full history | [x] |
| 63 | `mathop` | first divide × second add; both validation branches; empty→full history | [x] |
| 64 | `mathop` | first divide × second multiply; both validation branches; empty→full history | [x] |
| 65 | `mathop` | first divide × second subtract; both validation branches; empty→full history | [x] |
| 66 | `mathop` | first divide × second divide; both validation branches; empty→full history | [x] |
| 67 | `mathop` | first divide × second modulo; both validation branches; empty→full history | [x] |
| 68 | `mathop` | first divide × second default/add fallback; both validation branches; empty→full history | [x] |
| 69 | `mathop` | first modulo × second add; both validation branches; empty→full history | [x] |
| 70 | `mathop` | first modulo × second multiply; both validation branches; empty→full history | [x] |
| 71 | `mathop` | first modulo × second subtract; both validation branches; empty→full history | [x] |
| 72 | `mathop` | first modulo × second divide; both validation branches; empty→full history | [x] |
| 73 | `mathop` | first modulo × second modulo; both validation branches; empty→full history | [x] |
| 74 | `mathop` | first modulo × second default/add fallback; both validation branches; empty→full history | [x] |
| 75 | `mathop` | first default/add fallback × second add; both validation branches; empty→full history | [x] |
| 76 | `mathop` | first default/add fallback × second multiply; both validation branches; empty→full history | [x] |
| 77 | `mathop` | first default/add fallback × second subtract; both validation branches; empty→full history | [x] |
| 78 | `mathop` | first default/add fallback × second divide; both validation branches; empty→full history | [x] |
| 79 | `mathop` | first default/add fallback × second modulo; both validation branches; empty→full history | [x] |
| 80 | `mathop` | first default/add fallback × second default/add fallback; both validation branches; empty→full history | [x] |

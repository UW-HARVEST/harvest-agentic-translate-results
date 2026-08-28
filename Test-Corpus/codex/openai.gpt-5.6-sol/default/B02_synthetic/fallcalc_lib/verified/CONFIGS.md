# Configuration Surface

The rows below come from the public dynamic symbols and every input-dependent
`if`, loop bound, `switch`, and remainder-derived mode in
`c_src/src/lib.c`. "Randomized" means many fixed-seed values from the stated
class are exercised.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| C01 | `safe_double_to_int` | `d` is NaN (random payload/sign forms) | [x] |
| C02 | `safe_double_to_int` | `d` is positive infinity | [x] |
| C03 | `safe_double_to_int` | `d` is negative infinity | [x] |
| C04 | `safe_double_to_int` | finite `d >= INT_MAX`, including the exact boundary | [x] |
| C05 | `safe_double_to_int` | finite `d <= INT_MIN`, including the exact boundary | [x] |
| C06 | `safe_double_to_int` | finite in-range integral values (zero, positive, negative) | [x] |
| C07 | `safe_double_to_int` | finite in-range fractional values (positive and negative truncation) | [x] |
| C08 | `process_array_reverse` | `count < 0`; loop is skipped and `end` is not read | [x] |
| C09 | `process_array_reverse` | `count == 0`; loop is skipped and `end` is not read | [x] |
| C10 | `process_array_reverse` | `count == 1`; `end` points at the sole element | [x] |
| C11 | `process_array_reverse` | `count > 1`; `end` points at the final element | [x] |
| C12 | `switch_fallthrough_calculator` | `operation == 0`; cases 0, 1, and 2 execute | [x] |
| C13 | `switch_fallthrough_calculator` | `operation == 1`; cases 1 and 2 execute | [x] |
| C14 | `switch_fallthrough_calculator` | `operation == 2`; case 2 executes | [x] |
| C15 | `switch_fallthrough_calculator` | `operation == 3`; cases 3 and 4 execute | [x] |
| C16 | `switch_fallthrough_calculator` | `operation == 4`; case 4 executes | [x] |
| C17 | `switch_fallthrough_calculator` | `operation < 0` or `operation > 4`; default executes | [x] |
| C18 | `allocate_and_compute` | `size < 0`; converted allocation size is rejected by `malloc` | [x] |
| C19 | `allocate_and_compute` | `size == 0`; both loops are skipped | [x] |
| C20 | `allocate_and_compute` | `size == 1`; one zero-valued point, any multiplier class | [x] |
| C21 | `allocate_and_compute` | `size > 1`, finite zero multiplier | [x] |
| C22 | `allocate_and_compute` | `size > 1`, finite positive multiplier | [x] |
| C23 | `allocate_and_compute` | `size > 1`, finite negative multiplier | [x] |
| C24 | `allocate_and_compute` | `size > 1`, positive infinity multiplier | [x] |
| C25 | `allocate_and_compute` | `size > 1`, negative infinity multiplier | [x] |
| C26 | `allocate_and_compute` | `size > 1`, NaN multiplier | [x] |
| C27 | `allocate_and_compute` | `size > 1`, finite multiplier producing sum at/above `INT_MAX` | [x] |
| C28 | `allocate_and_compute` | `size > 1`, finite multiplier producing sum at/below `INT_MIN` | [x] |
| C29 | `foreach_sum` | `count < 0`; loop is skipped and `array` is not read | [x] |
| C30 | `foreach_sum` | `count == 0`; loop is skipped and `array` is not read | [x] |
| C31 | `foreach_sum` | `count == 1`; one element is read | [x] |
| C32 | `foreach_sum` | `count > 1`; elements are traversed from first to last | [x] |
| C33 | `fallcalc` and all callees | `param3 < 0 && param3 % 5 != 0` (switch default), `param4 % 10 <= -2` (allocation fails) | [x] |
| C34 | `fallcalc` and all callees | `param3 < 0 && param3 % 5 != 0` (switch default), `param4 % 10 == -1` (size 0) | [x] |
| C35 | `fallcalc` and all callees | `param3 < 0 && param3 % 5 != 0` (switch default), `param4 % 10 == 0` (size 1) | [x] |
| C36 | `fallcalc` and all callees | `param3 < 0 && param3 % 5 != 0` (switch default), `param4 % 10 > 0` (size 2..10) | [x] |
| C37 | `fallcalc` and all callees | `param3 <= 128 && param3 % 5 == 0` (operation 0, flag clear), `param4 % 10 <= -2` (allocation fails) | [x] |
| C38 | `fallcalc` and all callees | `param3 <= 128 && param3 % 5 == 0` (operation 0, flag clear), `param4 % 10 == -1` (size 0) | [x] |
| C39 | `fallcalc` and all callees | `param3 <= 128 && param3 % 5 == 0` (operation 0, flag clear), `param4 % 10 == 0` (size 1) | [x] |
| C40 | `fallcalc` and all callees | `param3 <= 128 && param3 % 5 == 0` (operation 0, flag clear), `param4 % 10 > 0` (size 2..10) | [x] |
| C41 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 1` (operation 1, flag clear), `param4 % 10 <= -2` | [x] |
| C42 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 1` (operation 1, flag clear), `param4 % 10 == -1` | [x] |
| C43 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 1` (operation 1, flag clear), `param4 % 10 == 0` | [x] |
| C44 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 1` (operation 1, flag clear), `param4 % 10 > 0` | [x] |
| C45 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 2` (operation 2, flag clear), `param4 % 10 <= -2` | [x] |
| C46 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 2` (operation 2, flag clear), `param4 % 10 == -1` | [x] |
| C47 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 2` (operation 2, flag clear), `param4 % 10 == 0` | [x] |
| C48 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 2` (operation 2, flag clear), `param4 % 10 > 0` | [x] |
| C49 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 3` (operation 3, flag clear), `param4 % 10 <= -2` | [x] |
| C50 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 3` (operation 3, flag clear), `param4 % 10 == -1` | [x] |
| C51 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 3` (operation 3, flag clear), `param4 % 10 == 0` | [x] |
| C52 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 3` (operation 3, flag clear), `param4 % 10 > 0` | [x] |
| C53 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 4` (operation 4, flag clear), `param4 % 10 <= -2` | [x] |
| C54 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 4` (operation 4, flag clear), `param4 % 10 == -1` | [x] |
| C55 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 4` (operation 4, flag clear), `param4 % 10 == 0` | [x] |
| C56 | `fallcalc` and all callees | `0 <= param3 <= 128 && param3 % 5 == 4` (operation 4, flag clear), `param4 % 10 > 0` | [x] |
| C57 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 0` (operation 0, flag set), `param4 % 10 <= -2` | [x] |
| C58 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 0` (operation 0, flag set), `param4 % 10 == -1` | [x] |
| C59 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 0` (operation 0, flag set), `param4 % 10 == 0` | [x] |
| C60 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 0` (operation 0, flag set), `param4 % 10 > 0` | [x] |
| C61 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 1` (operation 1, flag set), `param4 % 10 <= -2` | [x] |
| C62 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 1` (operation 1, flag set), `param4 % 10 == -1` | [x] |
| C63 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 1` (operation 1, flag set), `param4 % 10 == 0` | [x] |
| C64 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 1` (operation 1, flag set), `param4 % 10 > 0` | [x] |
| C65 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 2` (operation 2, flag set), `param4 % 10 <= -2` | [x] |
| C66 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 2` (operation 2, flag set), `param4 % 10 == -1` | [x] |
| C67 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 2` (operation 2, flag set), `param4 % 10 == 0` | [x] |
| C68 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 2` (operation 2, flag set), `param4 % 10 > 0` | [x] |
| C69 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 3` (operation 3, flag set), `param4 % 10 <= -2` | [x] |
| C70 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 3` (operation 3, flag set), `param4 % 10 == -1` | [x] |
| C71 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 3` (operation 3, flag set), `param4 % 10 == 0` | [x] |
| C72 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 3` (operation 3, flag set), `param4 % 10 > 0` | [x] |
| C73 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 4` (operation 4, flag set), `param4 % 10 <= -2` | [x] |
| C74 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 4` (operation 4, flag set), `param4 % 10 == -1` | [x] |
| C75 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 4` (operation 4, flag set), `param4 % 10 == 0` | [x] |
| C76 | `fallcalc` and all callees | `param3 > 128 && param3 % 5 == 4` (operation 4, flag set), `param4 % 10 > 0` | [x] |

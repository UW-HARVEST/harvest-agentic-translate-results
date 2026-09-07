# Configuration surface

There are no Cargo features, C preprocessor feature flags, runtime option
setters, byte-order modes, element-type modes, or executable targets. The
configuration axes come directly from the C `if`, `switch`, and loop
conditions.

For the `fallcalc` matrix:

- `O0F`..`O4F`: `param3 % 5` is 0..4 and `param3 <= 0200`.
- `O0T`..`O4T`: `param3 % 5` is 0..4 and `param3 > 0200`.
- `ODF`: `param3 % 5` is negative (switch default); the flag is necessarily false.
- `C-`, `CI`, `C+`: `floating_calc` reaches the low clamp, interior conversion,
  or high clamp path in `safe_double_to_int`.
- `A-`, `A0`, `A1`, `AN`: `param4 % 10 + 1` is negative, zero, one, or 2..10.

## Direct low-level entry points

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 1 | `safe_double_to_int` | NaN payloads | [x] |
| 2 | `safe_double_to_int` | positive infinity | [x] |
| 3 | `safe_double_to_int` | negative infinity | [x] |
| 4 | `safe_double_to_int` | finite values at/above `INT_MAX` | [x] |
| 5 | `safe_double_to_int` | finite values at/below `INT_MIN` | [x] |
| 6 | `safe_double_to_int` | finite values strictly inside integer range, including signs, zero, and fractions | [x] |
| 7 | `process_array_reverse` | `count < 0`; no elements read | [x] |
| 8 | `process_array_reverse` | `count == 0`; no elements read | [x] |
| 9 | `process_array_reverse` | `count == 1`; `end` points at the sole element | [x] |
| 10 | `process_array_reverse` | `count > 1`; `end` points at the last element | [x] |
| 11 | `switch_fallthrough_calculator` | operation `0` (`*010`, then cases 1 and 2) | [x] |
| 12 | `switch_fallthrough_calculator` | operation `1` (then case 2) | [x] |
| 13 | `switch_fallthrough_calculator` | operation `2` | [x] |
| 14 | `switch_fallthrough_calculator` | operation `3` (then case 4) | [x] |
| 15 | `switch_fallthrough_calculator` | operation `4` | [x] |
| 16 | `switch_fallthrough_calculator` | operation outside `0..=4` (default) | [x] |
| 17 | `allocate_and_compute` | `size == 0` | [x] |
| 18 | `allocate_and_compute` | `size == 1`; sum remains zero | [x] |
| 19 | `allocate_and_compute` | `size > 1`, finite positive multiplier, finite in-range sum | [x] |
| 20 | `allocate_and_compute` | `size > 1`, zero multiplier | [x] |
| 21 | `allocate_and_compute` | `size > 1`, finite negative multiplier, finite in-range sum | [x] |
| 22 | `allocate_and_compute` | `size > 1`, NaN multiplier | [x] |
| 23 | `allocate_and_compute` | `size > 1`, positive-infinity multiplier | [x] |
| 24 | `allocate_and_compute` | `size > 1`, negative-infinity multiplier | [x] |
| 25 | `allocate_and_compute` | `size > 1`, finite multiplier producing sum `>= INT_MAX` | [x] |
| 26 | `allocate_and_compute` | `size > 1`, finite multiplier producing sum `<= INT_MIN` | [x] |
| 27 | `foreach_sum` | `count < 0`; no elements read | [x] |
| 28 | `foreach_sum` | `count == 0`; no elements read | [x] |
| 29 | `foreach_sum` | `count == 1` | [x] |
| 30 | `foreach_sum` | `count > 1` | [x] |

## Composed `fallcalc` entry point

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 31 | `fallcalc` | `O0F × C- × A-` | [x] |
| 32 | `fallcalc` | `O0F × C- × A0` | [x] |
| 33 | `fallcalc` | `O0F × C- × A1` | [x] |
| 34 | `fallcalc` | `O0F × C- × AN` | [x] |
| 35 | `fallcalc` | `O0F × CI × A-` | [x] |
| 36 | `fallcalc` | `O0F × CI × A0` | [x] |
| 37 | `fallcalc` | `O0F × CI × A1` | [x] |
| 38 | `fallcalc` | `O0F × CI × AN` | [x] |
| 39 | `fallcalc` | `O0F × C+ × A-` | [x] |
| 40 | `fallcalc` | `O0F × C+ × A0` | [x] |
| 41 | `fallcalc` | `O0F × C+ × A1` | [x] |
| 42 | `fallcalc` | `O0F × C+ × AN` | [x] |
| 43 | `fallcalc` | `O1F × C- × A-` | [x] |
| 44 | `fallcalc` | `O1F × C- × A0` | [x] |
| 45 | `fallcalc` | `O1F × C- × A1` | [x] |
| 46 | `fallcalc` | `O1F × C- × AN` | [x] |
| 47 | `fallcalc` | `O1F × CI × A-` | [x] |
| 48 | `fallcalc` | `O1F × CI × A0` | [x] |
| 49 | `fallcalc` | `O1F × CI × A1` | [x] |
| 50 | `fallcalc` | `O1F × CI × AN` | [x] |
| 51 | `fallcalc` | `O1F × C+ × A-` | [x] |
| 52 | `fallcalc` | `O1F × C+ × A0` | [x] |
| 53 | `fallcalc` | `O1F × C+ × A1` | [x] |
| 54 | `fallcalc` | `O1F × C+ × AN` | [x] |
| 55 | `fallcalc` | `O2F × C- × A-` | [x] |
| 56 | `fallcalc` | `O2F × C- × A0` | [x] |
| 57 | `fallcalc` | `O2F × C- × A1` | [x] |
| 58 | `fallcalc` | `O2F × C- × AN` | [x] |
| 59 | `fallcalc` | `O2F × CI × A-` | [x] |
| 60 | `fallcalc` | `O2F × CI × A0` | [x] |
| 61 | `fallcalc` | `O2F × CI × A1` | [x] |
| 62 | `fallcalc` | `O2F × CI × AN` | [x] |
| 63 | `fallcalc` | `O2F × C+ × A-` | [x] |
| 64 | `fallcalc` | `O2F × C+ × A0` | [x] |
| 65 | `fallcalc` | `O2F × C+ × A1` | [x] |
| 66 | `fallcalc` | `O2F × C+ × AN` | [x] |
| 67 | `fallcalc` | `O3F × C- × A-` | [x] |
| 68 | `fallcalc` | `O3F × C- × A0` | [x] |
| 69 | `fallcalc` | `O3F × C- × A1` | [x] |
| 70 | `fallcalc` | `O3F × C- × AN` | [x] |
| 71 | `fallcalc` | `O3F × CI × A-` | [x] |
| 72 | `fallcalc` | `O3F × CI × A0` | [x] |
| 73 | `fallcalc` | `O3F × CI × A1` | [x] |
| 74 | `fallcalc` | `O3F × CI × AN` | [x] |
| 75 | `fallcalc` | `O3F × C+ × A-` | [x] |
| 76 | `fallcalc` | `O3F × C+ × A0` | [x] |
| 77 | `fallcalc` | `O3F × C+ × A1` | [x] |
| 78 | `fallcalc` | `O3F × C+ × AN` | [x] |
| 79 | `fallcalc` | `O4F × C- × A-` | [x] |
| 80 | `fallcalc` | `O4F × C- × A0` | [x] |
| 81 | `fallcalc` | `O4F × C- × A1` | [x] |
| 82 | `fallcalc` | `O4F × C- × AN` | [x] |
| 83 | `fallcalc` | `O4F × CI × A-` | [x] |
| 84 | `fallcalc` | `O4F × CI × A0` | [x] |
| 85 | `fallcalc` | `O4F × CI × A1` | [x] |
| 86 | `fallcalc` | `O4F × CI × AN` | [x] |
| 87 | `fallcalc` | `O4F × C+ × A-` | [x] |
| 88 | `fallcalc` | `O4F × C+ × A0` | [x] |
| 89 | `fallcalc` | `O4F × C+ × A1` | [x] |
| 90 | `fallcalc` | `O4F × C+ × AN` | [x] |
| 91 | `fallcalc` | `O0T × C- × A-` | [x] |
| 92 | `fallcalc` | `O0T × C- × A0` | [x] |
| 93 | `fallcalc` | `O0T × C- × A1` | [x] |
| 94 | `fallcalc` | `O0T × C- × AN` | [x] |
| 95 | `fallcalc` | `O0T × CI × A-` | [x] |
| 96 | `fallcalc` | `O0T × CI × A0` | [x] |
| 97 | `fallcalc` | `O0T × CI × A1` | [x] |
| 98 | `fallcalc` | `O0T × CI × AN` | [x] |
| 99 | `fallcalc` | `O0T × C+ × A-` | [x] |
| 100 | `fallcalc` | `O0T × C+ × A0` | [x] |
| 101 | `fallcalc` | `O0T × C+ × A1` | [x] |
| 102 | `fallcalc` | `O0T × C+ × AN` | [x] |
| 103 | `fallcalc` | `O1T × C- × A-` | [x] |
| 104 | `fallcalc` | `O1T × C- × A0` | [x] |
| 105 | `fallcalc` | `O1T × C- × A1` | [x] |
| 106 | `fallcalc` | `O1T × C- × AN` | [x] |
| 107 | `fallcalc` | `O1T × CI × A-` | [x] |
| 108 | `fallcalc` | `O1T × CI × A0` | [x] |
| 109 | `fallcalc` | `O1T × CI × A1` | [x] |
| 110 | `fallcalc` | `O1T × CI × AN` | [x] |
| 111 | `fallcalc` | `O1T × C+ × A-` | [x] |
| 112 | `fallcalc` | `O1T × C+ × A0` | [x] |
| 113 | `fallcalc` | `O1T × C+ × A1` | [x] |
| 114 | `fallcalc` | `O1T × C+ × AN` | [x] |
| 115 | `fallcalc` | `O2T × C- × A-` | [x] |
| 116 | `fallcalc` | `O2T × C- × A0` | [x] |
| 117 | `fallcalc` | `O2T × C- × A1` | [x] |
| 118 | `fallcalc` | `O2T × C- × AN` | [x] |
| 119 | `fallcalc` | `O2T × CI × A-` | [x] |
| 120 | `fallcalc` | `O2T × CI × A0` | [x] |
| 121 | `fallcalc` | `O2T × CI × A1` | [x] |
| 122 | `fallcalc` | `O2T × CI × AN` | [x] |
| 123 | `fallcalc` | `O2T × C+ × A-` | [x] |
| 124 | `fallcalc` | `O2T × C+ × A0` | [x] |
| 125 | `fallcalc` | `O2T × C+ × A1` | [x] |
| 126 | `fallcalc` | `O2T × C+ × AN` | [x] |
| 127 | `fallcalc` | `O3T × C- × A-` | [x] |
| 128 | `fallcalc` | `O3T × C- × A0` | [x] |
| 129 | `fallcalc` | `O3T × C- × A1` | [x] |
| 130 | `fallcalc` | `O3T × C- × AN` | [x] |
| 131 | `fallcalc` | `O3T × CI × A-` | [x] |
| 132 | `fallcalc` | `O3T × CI × A0` | [x] |
| 133 | `fallcalc` | `O3T × CI × A1` | [x] |
| 134 | `fallcalc` | `O3T × CI × AN` | [x] |
| 135 | `fallcalc` | `O3T × C+ × A-` | [x] |
| 136 | `fallcalc` | `O3T × C+ × A0` | [x] |
| 137 | `fallcalc` | `O3T × C+ × A1` | [x] |
| 138 | `fallcalc` | `O3T × C+ × AN` | [x] |
| 139 | `fallcalc` | `O4T × C- × A-` | [x] |
| 140 | `fallcalc` | `O4T × C- × A0` | [x] |
| 141 | `fallcalc` | `O4T × C- × A1` | [x] |
| 142 | `fallcalc` | `O4T × C- × AN` | [x] |
| 143 | `fallcalc` | `O4T × CI × A-` | [x] |
| 144 | `fallcalc` | `O4T × CI × A0` | [x] |
| 145 | `fallcalc` | `O4T × CI × A1` | [x] |
| 146 | `fallcalc` | `O4T × CI × AN` | [x] |
| 147 | `fallcalc` | `O4T × C+ × A-` | [x] |
| 148 | `fallcalc` | `O4T × C+ × A0` | [x] |
| 149 | `fallcalc` | `O4T × C+ × A1` | [x] |
| 150 | `fallcalc` | `O4T × C+ × AN` | [x] |
| 151 | `fallcalc` | `ODF × C- × A-` | [x] |
| 152 | `fallcalc` | `ODF × C- × A0` | [x] |
| 153 | `fallcalc` | `ODF × C- × A1` | [x] |
| 154 | `fallcalc` | `ODF × C- × AN` | [x] |
| 155 | `fallcalc` | `ODF × CI × A-` | [x] |
| 156 | `fallcalc` | `ODF × CI × A0` | [x] |
| 157 | `fallcalc` | `ODF × CI × A1` | [x] |
| 158 | `fallcalc` | `ODF × CI × AN` | [x] |
| 159 | `fallcalc` | `ODF × C+ × A-` | [x] |
| 160 | `fallcalc` | `ODF × C+ × A0` | [x] |
| 161 | `fallcalc` | `ODF × C+ × A1` | [x] |
| 162 | `fallcalc` | `ODF × C+ × AN` | [x] |

# Configuration Surface

The public surface contains only `encode_quant`. Its branch-distinct
configuration axes are:

- `lsbit == 0`: leave candidate low bits unchanged.
- `lsbit == 4`: clear bit 0, then synthesize it from bits 1 and 2.
- `lsbit` odd: force bit 0.
- `lsbit` even, nonzero, and not 4: clear bit 0.
- `uni & 7 == 0`: the decrement candidate is clamped.
- `uni & 7 == 7`: the increment candidate is clamped.
- `uni & 8`: selects the sign of the quantized difference.

Crossing the four `lsbit` classes with every low nibble of `uni` captures the
full pruned cross-product of these branches, including the bit patterns used
by the `lsbit == 4` synthesis. For every row, tests randomize the high 28 bits
of `uni` and all 32 bits of `step`, `pred`, `tgt`, and `tgt2`; they also use
multiple representative values from the row's `lsbit` equivalence class.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C001 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0x0` | [x] |
| C002 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0x1` | [x] |
| C003 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0x2` | [x] |
| C004 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0x3` | [x] |
| C005 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0x4` | [x] |
| C006 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0x5` | [x] |
| C007 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0x6` | [x] |
| C008 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0x7` | [x] |
| C009 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0x8` | [x] |
| C010 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0x9` | [x] |
| C011 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0xa` | [x] |
| C012 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0xb` | [x] |
| C013 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0xc` | [x] |
| C014 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0xd` | [x] |
| C015 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0xe` | [x] |
| C016 | `encode_quant` | `lsbit == 0`; `uni & 0xf == 0xf` | [x] |
| C017 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0x0` | [x] |
| C018 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0x1` | [x] |
| C019 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0x2` | [x] |
| C020 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0x3` | [x] |
| C021 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0x4` | [x] |
| C022 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0x5` | [x] |
| C023 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0x6` | [x] |
| C024 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0x7` | [x] |
| C025 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0x8` | [x] |
| C026 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0x9` | [x] |
| C027 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0xa` | [x] |
| C028 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0xb` | [x] |
| C029 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0xc` | [x] |
| C030 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0xd` | [x] |
| C031 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0xe` | [x] |
| C032 | `encode_quant` | `lsbit == 4`; `uni & 0xf == 0xf` | [x] |
| C033 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0x0` | [x] |
| C034 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0x1` | [x] |
| C035 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0x2` | [x] |
| C036 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0x3` | [x] |
| C037 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0x4` | [x] |
| C038 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0x5` | [x] |
| C039 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0x6` | [x] |
| C040 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0x7` | [x] |
| C041 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0x8` | [x] |
| C042 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0x9` | [x] |
| C043 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0xa` | [x] |
| C044 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0xb` | [x] |
| C045 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0xc` | [x] |
| C046 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0xd` | [x] |
| C047 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0xe` | [x] |
| C048 | `encode_quant` | odd `lsbit`; `uni & 0xf == 0xf` | [x] |
| C049 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0x0` | [x] |
| C050 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0x1` | [x] |
| C051 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0x2` | [x] |
| C052 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0x3` | [x] |
| C053 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0x4` | [x] |
| C054 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0x5` | [x] |
| C055 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0x6` | [x] |
| C056 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0x7` | [x] |
| C057 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0x8` | [x] |
| C058 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0x9` | [x] |
| C059 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0xa` | [x] |
| C060 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0xb` | [x] |
| C061 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0xc` | [x] |
| C062 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0xd` | [x] |
| C063 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0xe` | [x] |
| C064 | `encode_quant` | even nonzero `lsbit != 4`; `uni & 0xf == 0xf` | [x] |


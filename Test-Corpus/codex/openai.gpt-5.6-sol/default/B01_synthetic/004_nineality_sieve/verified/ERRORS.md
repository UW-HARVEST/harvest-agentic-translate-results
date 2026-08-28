# Differential Verification

## Mismatches

No behavioral mismatches were found between the C and Rust executables.

## Harness Corrections

- An unrestricted `INT_MAX` case would emit over two billion lines after the
  signed wrap, so the case uses an identical 4096-byte file-size limit for both
  executables.
- Cargo can inherit an ignored `SIGXFSZ`. The subprocess launcher resets that
  signal to its default disposition before applying the file-size limit.

## Covered Input Classes

- Missing, extra, and multiple invalid arguments exercise the `argc` error and
  confirm that argument count is checked before parsing.
- Empty, non-numeric, and whitespace-only arguments exercise the `strtol`
  no-conversion error.
- `9` exercises immediate loop termination; `7` exercises loop continuation.
- Numeric prefixes and leading whitespace/sign/zeroes exercise the exact
  partial-parse behavior of `strtol`.
- `-9` exercises C's signed remainder behavior.
- `2147483639` exercises the largest `int` that terminates before overflow.
- `2147483647` exercises the maximum `int` and signed wrap. Both programs run
  under the same 4096-byte output file limit because an unrestricted run would
  emit over two billion lines after wrapping.
- `4294967295` exercises `long`-to-`int` narrowing.
- Values beyond `LONG_MAX` and `LONG_MIN` exercise `strtol` saturation.

Every case compares stdout bytes, stderr bytes, and process exit status.

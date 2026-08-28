#!/usr/bin/env bash
# argv parsing (`atoi`) parity: the C driver vs the Rust driver, config add/5.
set -u
cd "$(dirname "$0")/translation"
timeout 600 cargo build --release --no-default-features --features add,5 >/dev/null 2>&1 || {
  echo "build failed"; exit 1; }
C=/tmp/cref/add_5/driver
R=target/release/driver

ARGS=('0' '-0' '+0' '1' '-1' '+1' '007' '-007' '  42' $'\t42' $'\n42' $'\v9' $'\f9' $'\r9'
      '2147483647' '2147483648' '-2147483648' '-2147483649' '4294967296' '4294967297'
      '9223372036854775807' '9223372036854775808' '-9223372036854775808'
      '-9223372036854775809' '99999999999999999999' '-99999999999999999999'
      '0x10' '010' '1e3' '12x' 'x12' '' ' ' '-' '+' '--3' '+-3' '- 3' 'abc' '3.9' '-3.9'
      '1_000' '  -12abc' '000000000000000000005' '2147483647999999999999999999'
      '18446744073709551616' '-18446744073709551617' '  +  7' '9223372036854775806')

fail=0
for a in "${ARGS[@]}"; do
  for b in '3' '-3' '0' '2147483647'; do
    cout=$("$C" "$a" "$b" 2>&1); cst=$?
    rout=$("$R" "$a" "$b" 2>&1); rst=$?
    cout=${cout//$C/PROG}; rout=${rout//$R/PROG}
    if [[ "$cout" != "$rout" || "$cst" != "$rst" ]]; then
      printf 'ATOI MISMATCH a=%q b=%q (exit c=%s r=%s)\n' "$a" "$b" "$cst" "$rst"
      diff <(printf '%s\n' "$cout") <(printf '%s\n' "$rout") | sed 's/^/    /'
      fail=1
    fi
  done
done

# Random digit/sign/space soup.
for _ in $(seq 400); do
  a=$(head -c 24 /dev/urandom | tr -dc '0-9+\- \t' | head -c 12)
  b=$(head -c 24 /dev/urandom | tr -dc '0-9+\-' | head -c 8)
  cout=$("$C" "$a" "$b" 2>&1); cst=$?
  rout=$("$R" "$a" "$b" 2>&1); rst=$?
  cout=${cout//$C/PROG}; rout=${rout//$R/PROG}
  if [[ "$cout" != "$rout" || "$cst" != "$rst" ]]; then
    printf 'ATOI FUZZ MISMATCH a=%q b=%q\n' "$a" "$b"
    diff <(printf '%s\n' "$cout") <(printf '%s\n' "$rout") | sed 's/^/    /'
    fail=1
  fi
done

[[ $fail -eq 0 ]] && echo "=== PASS (atoi parity) ==="
exit $fail

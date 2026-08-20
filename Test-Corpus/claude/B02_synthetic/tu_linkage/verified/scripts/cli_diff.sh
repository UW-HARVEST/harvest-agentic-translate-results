#!/usr/bin/env bash
# Differential test driver for the *executable* (c_src/src/main.c vs src/main.rs
# + the `main` in src/lib.rs).  Compares stdout, stderr and exit status of the C
# program and the Rust program for the same argv / stdin.
#
# usage: cli_diff.sh                 # run the built-in case list
#        cli_diff.sh -v              # verbose (print every case)
set -u
HERE="$(cd "$(dirname "$0")/.." && pwd)"
CBIN="$HERE/c_src/build/driver"
RBIN="$HERE/target/release/driver"
TMP="${TMPDIR:-/tmp}/cli_diff.$$"
mkdir -p "$TMP"
VERBOSE=0
[ "${1:-}" = "-v" ] && VERBOSE=1

fail=0
total=0

# `main` prints argv[0] in --help output, so both programs must be invoked
# through the identical argv[0] string ("./driver") from their own directory.
mkdir -p "$TMP/c" "$TMP/r"
cp -f "$CBIN" "$TMP/c/driver"
cp -f "$RBIN" "$TMP/r/driver"

# run_case <stdin-file-or-empty> <args...>
run_case() {
  local stdin_file="$1"; shift
  total=$((total + 1))
  [ -z "$stdin_file" ] && stdin_file=/dev/null
  (cd "$TMP/c" && timeout 20 ./driver "$@") <"$stdin_file" >"$TMP/c.out" 2>"$TMP/c.err"; local crc=$?
  (cd "$TMP/r" && timeout 20 ./driver "$@") <"$stdin_file" >"$TMP/r.out" 2>"$TMP/r.err"; local rrc=$?
  # also compare the merged stream to catch stdout/stderr interleaving diffs
  (cd "$TMP/c" && timeout 20 ./driver "$@") <"$stdin_file" >"$TMP/c.both" 2>&1
  (cd "$TMP/r" && timeout 20 ./driver "$@") <"$stdin_file" >"$TMP/r.both" 2>&1
  local ok=1
  cmp -s "$TMP/c.out" "$TMP/r.out" || ok=0
  cmp -s "$TMP/c.err" "$TMP/r.err" || ok=0
  cmp -s "$TMP/c.both" "$TMP/r.both" || ok=0
  [ "$crc" = "$rrc" ] || ok=0
  if [ "$ok" = 0 ]; then
    fail=$((fail + 1))
    echo "FAIL argv=[$*] stdin=${stdin_file:-none} rc: C=$crc R=$rrc"
    diff <(cat "$TMP/c.out") <(cat "$TMP/r.out") | head -10 | sed 's/^/  out /'
    diff <(cat "$TMP/c.err") <(cat "$TMP/r.err") | head -10 | sed 's/^/  err /'
    diff <(cat "$TMP/c.both") <(cat "$TMP/r.both") | head -10 | sed 's/^/  both /'
  elif [ "$VERBOSE" = 1 ]; then
    echo "ok   argv=[$*] rc=$crc  out=$(head -c 120 "$TMP/c.out" | tr '\n' '|')"
  fi
}

# ---------------------------------------------------------------- fixed cases
run_case ""                       # no args at all -> "no program", rc 2
run_case "" --help
run_case "" --help 1 2 3
run_case "" 1 2 --help 3
run_case "" --stdin              # stdin closed/empty -> no program
run_case "" ""                   # empty arg: strtol -> no conversion, endptr==nptr, *e=='\0' -> pushes 0
run_case "" " "                  # blank arg -> skipped
run_case "" "  12  "             # trailing space -> *e != 0 -> skipped
run_case "" " 12"                # leading space is consumed by strtol -> pushed
run_case "" "abc"
run_case "" "12abc"
run_case "" "0x10"
run_case "" "+-5"
run_case "" "-0"
run_case "" "+7"
run_case "" "007"
run_case "" "99999999999999999999" 3     # LONG_MAX -> (int)-1
run_case "" "-99999999999999999999" 3    # LONG_MIN -> (int)0
run_case "" "2147483648" 3               # > INT_MAX -> truncated
run_case "" "-2147483649" 3
run_case "" "4294967296" 3                # truncates to 0
run_case "" $'\t12'                       # tab is whitespace for strtol
run_case "" $'\xff\xfe'                    # non-UTF8 argument
run_case "" "--Stdin" "--STDIN" "--help=x"
run_case "" 10                            # opcode 10 -> immediate return 0
run_case "" 11                            # unknown opcode -> 99
run_case "" -1                            # negative opcode -> 99
run_case "" 0                             # push with missing immediate -> rc 1
run_case "" 0 42
run_case "" 1                             # add on empty stack -> rc 2
run_case "" 0 1 1
run_case "" 0 1 0 2 1
run_case "" 2                             # mul on empty -> rc 3
run_case "" 0 3 0 4 2
run_case "" 3                             # dup on empty stack -> peek default 0
run_case "" 4                             # drop on empty -> rc 4
run_case "" 0 9 4
run_case "" 5                             # classify with empty stack (peek 0)
run_case "" 0 7 5
run_case "" 0 -3 5
run_case "" 8
run_case "" 0 12345 8 8 8
run_case "" 6                             # cond jump, missing k -> rc 5
run_case "" 6 1                            # cond jump, empty stack -> rc 6
run_case "" 0 1 6 99                       # jump too far -> rc 7
run_case "" 0 1 6 -1 0 5                   # negative k -> (size_t) huge -> rc 7
run_case "" 0 0 6 2 0 7 0 8                # cond false -> fall through
run_case "" 0 1 6 0 0 8                    # k == 0
run_case "" 0 1 6 2 0 7 0 8                # exact jump
run_case "" 0 1 6 4 0 7 0 8                # k == n - ip boundary
run_case "" 7                              # repeat, missing times -> rc 8
run_case "" 7 3                             # repeat, nothing to repeat -> rc 9
run_case "" 7 0 3                           # times == 0
run_case "" 7 -5 3                          # negative times
run_case "" 7 4 3                           # repeat dup 4 times
run_case "" 0 6 7 3 5                       # repeat classify
run_case "" 7 3 0                            # inner push: immediate missing -> rc 1 -> trace 12
run_case "" 7 2 10                           # inner return
run_case "" 7 2 11                           # inner unknown opcode
run_case "" 7 2 7                            # nested repeat inside 1-instruction window
run_case "" 9                                # stream, missing m -> rc 10
run_case "" 9 -1                              # negative m -> rc 11
run_case "" 9 1                                # m > stack len -> rc 11
run_case "" 9 0                                # m == 0
run_case "" 0 5 9 1                            # m == 1, double pop
run_case "" 0 5 0 6 9 2
run_case "" 0 1 0 2 0 3 0 4 9 4
run_case "" 0 1 0 2 0 3 0 4 9 2                # second pop round partially succeeds
run_case "" 0 -7 0 -8 9 2                      # negative stream values
run_case "" 0 2147483647 0 -2147483648 9 2     # extreme stream values
run_case "" 0 100 3 3 3 3 9 3 9 2 5 8 1 2 10

# --------------------------------------------------------------- stdin cases
printf '' > "$TMP/in.empty"
printf '\n' > "$TMP/in.nl"
printf '0 5 1 10\n' > "$TMP/in.simple"
printf '0 5\n1\n10\n' > "$TMP/in.multiline"
printf '0\t5\r\n3 3 1 1\n' > "$TMP/in.tabs"
printf '  0   5   9 2  ' > "$TMP/in.nonl"
printf 'abc 5 12x 7\n' > "$TMP/in.junk"
printf '99999999999999999999 -99999999999999999999 5\n' > "$TMP/in.overflow"
printf '0 5' > "$TMP/in.noeol"
printf '\0 0 5 1\n' > "$TMP/in.nul"
printf '0 5 \0 1 1\n0 6 1\n' > "$TMP/in.nulmid"
# a line longer than the 4096 byte fgets buffer, splitting a token across reads
{ for i in $(seq 1 1200); do printf '3 '; done; printf '1\n'; } > "$TMP/in.long"
{ printf '0 '; for i in $(seq 1 4090); do printf '1'; done; printf ' 3 3\n'; } > "$TMP/in.splittoken"
for f in empty nl simple multiline tabs nonl junk overflow noeol nul nulmid long splittoken; do
  run_case "$TMP/in.$f" --stdin
  run_case "$TMP/in.$f" --stdin 0 77
  run_case "$TMP/in.$f" 0 77          # stdin ignored without --stdin
done

# ------------------------------------------------------- randomised programs
# Deterministic pseudo random opcode soup, fixed seed.
for seed in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
  prog=$(awk -v s="$seed" 'BEGIN{srand(s);n=3+int(rand()*14);for(i=0;i<n;i++){r=rand();
      if(r<0.75) printf "%d ", int(rand()*12);
      else if (r<0.9) printf "%d ", int(rand()*2000)-1000;
      else printf "%d ", int(rand()*4294967296)-2147483648;}}')
  # shellcheck disable=SC2086
  run_case "" $prog
done

echo "cli_diff: $((total - fail))/$total cases matched"
rm -rf "$TMP"
[ "$fail" = 0 ] || exit 1

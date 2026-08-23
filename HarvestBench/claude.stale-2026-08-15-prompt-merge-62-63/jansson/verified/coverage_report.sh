#!/usr/bin/env bash
# Mechanically reports which CONFIGS.md / ERRORS.md rows are referenced by a
# `row<N>` label in the test suite, and which are not. Rows in ERRORS.md marked
# UB / INT / DEAD are excluded from the "must be covered" set, with the reason
# recorded in ERRORS.md itself.
set -uo pipefail
cd "$(dirname "$0")"
GREP=/usr/bin/grep

# All rowN tokens appearing in test labels, e.g. diff("row150/...") or "rows30-46/".
collect_labels() {
  $GREP -rhoE 'row[s]?[0-9]+(-[0-9]+)?' tests/ 2>/dev/null \
    | sed -E 's/^rows?//' \
    | while IFS= read -r tok; do
        case "$tok" in
          *-*) a=${tok%%-*}; b=${tok##*-}
               if [ "$a" -le "$b" ] 2>/dev/null; then seq "$a" "$b"; fi ;;
          *)   echo "$tok" ;;
        esac
      done | sort -n -u
}

LABELS=$(collect_labels)
echo "$LABELS" > /tmp/labels.txt
echo "Distinct row numbers referenced in tests/: $(wc -l < /tmp/labels.txt)"

report() { # $1=file  $2=total  $3=name
  local file=$1 total=$2 name=$3
  local missing=() n=0
  for ((i = 1; i <= total; i++)); do
    if ! $GREP -qx "$i" /tmp/labels.txt; then missing+=("$i"); else n=$((n+1)); fi
  done
  printf '\n%s: %d/%d rows referenced by a test label\n' "$name" "$n" "$total"
  if [ ${#missing[@]} -gt 0 ]; then
    printf '  not referenced by number: %s\n' "$(echo "${missing[@]}" | tr ' ' ',')"
  fi
}

CONF_ROWS=$($GREP -cE '^\| [0-9]+ \|' CONFIGS.md)
ERR_ROWS=$($GREP -cE '^\| [0-9]+ \|' ERRORS.md)

echo
echo "CONFIGS.md rows: $CONF_ROWS"
echo "ERRORS.md  rows: $ERR_ROWS"
$GREP -oE '\| (TEST|OOM|UB|INT|DEAD) \|$' ERRORS.md | sort | uniq -c | sed 's/^/  /'

echo
echo "Rows excluded from the must-cover set (documented in ERRORS.md):"
$GREP -E '^\| [0-9]+ \|.*\| (UB|INT|DEAD) \|$' ERRORS.md \
  | sed -E 's/^\| ([0-9]+) \| ([^|]*)\|.*\| (UB|INT|DEAD) \|$/  row \1 (\3): \2/' | head -30

report CONFIGS.md "$CONF_ROWS" "CONFIGS.md"
report ERRORS.md  "$ERR_ROWS"  "ERRORS.md"

echo
echo "NOTE: row-number labels are a lower bound on coverage — many tests cover"
echo "rows via randomized property sweeps and shared closures without naming"
echo "every row number. Treat unreferenced rows as 'needs manual confirmation',"
echo "not automatically 'untested'."

#!/bin/bash
# Regenerates SYMBOLS.md: the full nm -D surface of the C shared libraries for
# every configuration, checked against the Rust cdylib for the same config.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT" || exit 1
T="${TMPDIR:-/tmp}"

cat SYMBOLS.md.head > translation/SYMBOLS.md

# --- per-config parity summary ------------------------------------------------
echo '## Per-configuration parity (all 48 CMake configurations)' >> translation/SYMBOLS.md
echo >> translation/SYMBOLS.md
echo '| config (backend-thash-secpar) | # C symbols | missing from Rust `.so` |' >> translation/SYMBOLS.md
echo '|---|---|---|' >> translation/SYMBOLS.md
missing_total=0
for b in haraka sha2 shake blake; do
 for t in robust simple; do
  for s in 128s 128f 192s 192f 256s 256f; do
    combo="$b-$t-$s"
    nm -D --defined-only "cbuild/$combo/libsphincs_core_det.so" "cbuild/$combo/lib$b.so" \
      | awk 'NF>=3{print $3}' | sort -u > "$T/sym_c.txt"
    nm -D --defined-only "rbuild/$combo/libsphincs_core_det.so" \
      | awk 'NF>=3{print $3}' | sort -u > "$T/sym_r.txt"
    n=$(wc -l < "$T/sym_c.txt")
    miss=$(comm -23 "$T/sym_c.txt" "$T/sym_r.txt" | tr '\n' ' ')
    [ -z "${miss// /}" ] && miss='*(none)*' || missing_total=$((missing_total+1))
    echo "| \`$combo\` | $n | $miss |" >> translation/SYMBOLS.md
  done
 done
done
echo >> translation/SYMBOLS.md
echo "**Configurations with missing symbols: $missing_total / 48**" >> translation/SYMBOLS.md
echo >> translation/SYMBOLS.md

# --- full symbol listing, per backend ----------------------------------------
for b in haraka sha2 shake blake; do
  echo "## Backend \`$b\` — every C-exported symbol" >> translation/SYMBOLS.md
  echo >> translation/SYMBOLS.md
  echo '| symbol | nm type | C translation unit (best-effort grep) | in Rust `.so` |' >> translation/SYMBOLS.md
  echo '|---|---|---|---|' >> translation/SYMBOLS.md
  # union of symbols over all thash/secpar for this backend, with the type seen
  for combo_dir in cbuild/$b-simple-128f cbuild/$b-robust-128f cbuild/$b-simple-256f cbuild/$b-robust-256f; do
    nm -D --defined-only "$combo_dir/libsphincs_core_det.so" "$combo_dir/lib$b.so" 2>/dev/null \
      | awk 'NF>=3{print $3" "$2}'
  done | sort -u -k1,1 > "$T/sym_all.txt"
  nm -D --defined-only "rbuild/$b-simple-128f/libsphincs_core_det.so" \
    "rbuild/$b-robust-128f/libsphincs_core_det.so" \
    "rbuild/$b-simple-256f/libsphincs_core_det.so" \
    "rbuild/$b-robust-256f/libsphincs_core_det.so" \
    | awk 'NF>=3{print $3}' | sort -u > "$T/sym_rall.txt"
  while read -r sym ty; do
    tu=$(grep -rlE "(^|[^A-Za-z0-9_])${sym#SPX_}[[:space:]]*\(" c_src/app/src c_src/lib/$b/src 2>/dev/null | head -1)
    [ -z "$tu" ] && tu=$(grep -rlE "(^|[^A-Za-z0-9_])${sym}[[:space:]]*[\(\[=]" c_src/app/src c_src/lib/$b/src 2>/dev/null | head -1)
    [ -z "$tu" ] && tu="(macro-generated / data)"
    tu=${tu#c_src/}
    if grep -qx "$sym" "$T/sym_rall.txt"; then mark='yes'; else mark='**MISSING**'; fi
    echo "| \`$sym\` | $ty | \`$tu\` | $mark |" >> translation/SYMBOLS.md
  done < "$T/sym_all.txt"
  echo >> translation/SYMBOLS.md
done
echo "wrote translation/SYMBOLS.md ($(wc -l < translation/SYMBOLS.md) lines); configs with missing symbols: $missing_total"

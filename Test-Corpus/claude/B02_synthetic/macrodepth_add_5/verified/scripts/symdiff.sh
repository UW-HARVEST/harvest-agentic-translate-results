#!/bin/bash
# Compare `nm -D` between the C .so and the Rust .so for every configuration.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT" || exit 1
fail=0
for d in artifacts/*/; do
  cfg="$(basename "$d")"
  nm -D --defined-only "$d/libcdriver.so" | awk '{print $2, $3}' | sort > "$TMPDIR/c.$cfg"
  nm -D --defined-only "$d/libdriver.so"  | awk '{print $2, $3}' | sort > "$TMPDIR/r.$cfg"
  # names only
  cut -d' ' -f2 "$TMPDIR/c.$cfg" | sort > "$TMPDIR/cn.$cfg"
  cut -d' ' -f2 "$TMPDIR/r.$cfg" | sort > "$TMPDIR/rn.$cfg"
  missing="$(comm -23 "$TMPDIR/cn.$cfg" "$TMPDIR/rn.$cfg" | tr '\n' ' ')"
  # undefined non-libc symbols in the Rust .so
  undef="$(nm -D --undefined-only "$d/libdriver.so" | awk '$1=="U"{print $2}' \
            | grep -v -E '@GLIBC|@GCC|^_ITM_|^__gmon_start__|^__cxa_finalize' | tr '\n' ' ')"
  # type/binding comparison
  typediff="$(diff <(sort "$TMPDIR/c.$cfg") <(sort "$TMPDIR/r.$cfg") >/dev/null && echo same || echo DIFFER)"
  if [ -n "$missing" ] || [ -n "$undef" ] || [ "$typediff" != same ]; then
    echo "FAIL $cfg missing=[$missing] undef=[$undef] types=$typediff"
    [ "$typediff" != same ] && diff <(sort "$TMPDIR/c.$cfg") <(sort "$TMPDIR/r.$cfg")
    fail=1
  else
    echo "OK   $cfg  ($(wc -l < "$TMPDIR/cn.$cfg") symbols, exact name+type match, 0 undefined non-libc)"
  fi
done
echo "---"; [ $fail -eq 0 ] && echo "SYMBOL PARITY OK FOR ALL CONFIGS" || echo "SYMBOL PARITY FAILURES"
exit $fail

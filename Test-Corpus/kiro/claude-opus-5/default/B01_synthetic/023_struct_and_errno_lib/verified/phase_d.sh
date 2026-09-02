#!/usr/bin/env bash
# Phase D driver: run the whole differential suite under every feature
# combination and under both profiles, then re-check symbol parity.
set -u
cd "$(dirname "$0")"

echo "### Feature combinations declared in Cargo.toml"
FEATS=$(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
if not m:
    print("", end="")
else:
    names = [ln.split('=')[0].strip() for ln in m.group(1).splitlines()
             if '=' in ln and not ln.strip().startswith('#')]
    print(" ".join(n for n in names if n != 'default'), end="")
PY
)
if [ -z "$FEATS" ]; then
  echo "  (none declared -> the only combination is the empty/default one)"
  COMBOS=("")
else
  echo "  features: $FEATS"
  # power set
  mapfile -t COMBOS < <(python3 - "$FEATS" <<'PY'
import itertools, sys
f = sys.argv[1].split()
for r in range(len(f) + 1):
    for c in itertools.combinations(f, r):
        print(",".join(c))
PY
)
fi

FAIL=0

echo
echo "### nm -D symbol parity"
C_SO=../c_src/build/libdriver.so
for PROFILE in debug release; do
  if [ "$PROFILE" = release ]; then cargo build -q --release; else cargo build -q; fi
  R_SO=target/$PROFILE/libdriver.so
  MISSING=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u))
  if [ -z "$MISSING" ]; then
    echo "  $PROFILE: OK (C exports: $(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u | tr '\n' ' '))"
  else
    echo "  $PROFILE: MISSING -> $MISSING"; FAIL=1
  fi
done

echo
echo "### cargo check / test per combination"
for COMBO in "${COMBOS[@]}"; do
  LABEL=${COMBO:-<default/none>}
  ARGS=(--no-default-features)
  [ -n "$COMBO" ] && ARGS+=(--features "$COMBO")

  if ! cargo check -q "${ARGS[@]}" >/tmp/pd.log 2>&1; then
    echo "  check  $LABEL: FAIL"; tail -5 /tmp/pd.log; FAIL=1; continue
  fi

  export DRIVER_TEST_FEATURES="$COMBO"
  for PROFILE in dev release; do
    PARGS=("${ARGS[@]}")
    [ "$PROFILE" = release ] && PARGS+=(--release)
    if timeout 900 cargo test -q "${PARGS[@]}" >/tmp/pd.log 2>&1; then
      echo "  test   $LABEL [$PROFILE]: PASS ($(grep -c 'test result: ok' /tmp/pd.log) binaries ok)"
    else
      echo "  test   $LABEL [$PROFILE]: FAIL"; grep -E 'FAILED|panicked' /tmp/pd.log | head -5; FAIL=1
    fi
  done
  unset DRIVER_TEST_FEATURES
done

echo
echo "### debug-assertions / overflow-checks forced ON (must not change behaviour)"
if RUSTFLAGS="-C debug-assertions=on -C overflow-checks=on" timeout 900 cargo test -q >/tmp/pd.log 2>&1; then
  echo "  PASS"
else
  echo "  FAIL"; grep -E 'FAILED|panicked' /tmp/pd.log | head -5; FAIL=1
fi

echo
[ $FAIL -eq 0 ] && echo "ALL PHASE D CHECKS PASSED" || echo "PHASE D FAILURES PRESENT"
exit $FAIL

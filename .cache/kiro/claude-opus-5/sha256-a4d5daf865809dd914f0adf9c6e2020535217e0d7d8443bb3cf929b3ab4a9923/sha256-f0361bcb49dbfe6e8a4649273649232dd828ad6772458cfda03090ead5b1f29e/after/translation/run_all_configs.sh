#!/usr/bin/env bash
# Runs the full differential suite across every cargo feature combination and
# both profiles. `translation/Cargo.toml` currently declares no [features], so
# the combination set is just the default build; the loop is written so that any
# feature added later is picked up automatically.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"

# --- build the C ground truth -------------------------------------------------
(
  cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { echo "FAIL: C build"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -name '*.so' | head -1)
echo "C  .so: $C_SO"

# --- enumerate feature combinations from Cargo.toml ---------------------------
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"=");gsub(/ /,"",a[1]); if(a[1]!="default") print a[1]}' Cargo.toml)
COMBOS=("")   # "" == default features
if [ -n "$FEATURES" ]; then
  # all non-empty subsets of the feature list
  mapfile -t FARR <<< "$FEATURES"
  n=${#FARR[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "feature combinations: ${#COMBOS[@]} -> [${COMBOS[*]}]"

RC=0
for profile in debug release; do
  RELFLAG=""; [ "$profile" = release ] && RELFLAG="--release"
  for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then FF=(); LABEL="(default)";
    else FF=(--no-default-features --features "$combo"); LABEL="$combo"; fi

    echo "=== profile=$profile features=$LABEL ==="
    timeout 600 cargo build $RELFLAG "${FF[@]}" -q || { echo "FAIL build"; RC=1; continue; }

    # The tests load the newest available Rust .so; hide the other profile's
    # artifact so the profile under test is the one actually exercised.
    OTHER=$([ "$profile" = release ] && echo debug || echo release)
    HIDDEN="target/$OTHER/libupdate_frame_header_lib.so"
    [ -f "$HIDDEN" ] && mv "$HIDDEN" "$HIDDEN.hidden"
    if [ "$profile" = debug ]; then
      # the loader prefers release; hide it
      [ -f target/release/libupdate_frame_header_lib.so ] && \
        mv target/release/libupdate_frame_header_lib.so target/release/libupdate_frame_header_lib.so.hidden
    fi

    timeout 600 cargo test $RELFLAG "${FF[@]}" 2>&1 | tail -25
    [ "${PIPESTATUS[0]}" -ne 0 ] && { echo "FAIL tests profile=$profile features=$LABEL"; RC=1; }

    for f in target/debug/libupdate_frame_header_lib.so.hidden \
             target/release/libupdate_frame_header_lib.so.hidden; do
      [ -f "$f" ] && mv "$f" "${f%.hidden}"
    done

    # --- symbol parity for this configuration ---
    R_SO=$(ls -t target/$profile/libupdate_frame_header_lib.so 2>/dev/null | head -1)
    MISSING=$(comm -23 \
      <(nm -D --defined-only --format=posix "$C_SO" | awk '{print $1}' | sort) \
      <(nm -D --defined-only --format=posix "$R_SO" | awk '{print $1}' | sort))
    if [ -n "$MISSING" ]; then
      echo "FAIL symbol parity ($profile/$LABEL) missing: $MISSING"; RC=1
    else
      echo "OK symbol parity ($profile/$LABEL): 0 missing"
    fi
  done
done

echo
[ $RC -eq 0 ] && echo "ALL CONFIGURATIONS PASSED" || echo "FAILURES PRESENT"
exit $RC

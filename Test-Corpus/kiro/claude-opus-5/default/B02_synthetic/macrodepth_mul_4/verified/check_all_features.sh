#!/usr/bin/env bash
# cargo check every valid feature combination declared in Cargo.toml.
#
# The [features] table mirrors the two CMake cache variables:
#   OP     -> add | sub | mul
#   REPEAT -> 0 | 1 | ... | 7
# so the valid combinations are the 24-element cross product, plus the two
# degenerate cases the C header also accepts (no -D at all -> #ifndef fallbacks,
# i.e. no features; and "everything on", which Cargo allows via --all-features).
set -u
cd "$(dirname "$0")"

fail=0
run() {
  local desc="$1"; shift
  if timeout 600 cargo "$@" >/tmp/cargo_check.log 2>&1; then
    printf 'ok    %s\n' "$desc"
  else
    printf 'FAIL  %s\n' "$desc"
    sed 's/^/        /' /tmp/cargo_check.log | grep -E 'error|warning: unused' | head -20
    fail=1
  fi
}

for OP in add sub mul; do
  for R in 0 1 2 3 4 5 6 7; do
    run "check $OP,$R" check --no-default-features --features "$OP,$R" --all-targets
  done
done

run "check no-features (header #ifndef fallback)" check --no-default-features --all-targets
run "check default features"                      check --all-targets
run "check --all-features"                        check --all-features --all-targets

[[ $fail -eq 0 ]] && echo "ALL FEATURE COMBINATIONS CHECK CLEAN"
exit $fail

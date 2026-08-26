#!/usr/bin/env bash
# Full verification matrix: every feature combination x every profile.
#
#   ./verify.sh            # check + build + symbol parity + differential tests
#
# Feature combinations are enumerated from Cargo.toml rather than hard-coded, so
# adding a [features] section automatically widens the matrix.
set -uo pipefail
TD="${TMPDIR:-/tmp}/verify-$$"; mkdir -p "$TD"
cd "$(dirname "$0")"

RED=$'\033[31m'; GRN=$'\033[32m'; YEL=$'\033[33m'; RST=$'\033[0m'
fail=0
note() { printf '%s\n' "=== $*"; }
ok()   { printf '%s\n' "  ${GRN}PASS${RST} $*"; }
bad()  { printf '%s\n' "  ${RED}FAIL${RST} $*"; fail=1; }

# ---------------------------------------------------------------- features ----
# Every feature name declared in [features], excluding the "default" key.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
n=${#FEATURES[@]}
note "declared features: ${n} ${FEATURES[*]:-(none)}"

# Power set of the declared features; with none declared this yields exactly one
# combination: the empty set (i.e. --no-default-features on its own).
COMBOS=()
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (( mask & (1 << i) )); then combo+="${FEATURES[i]},"; fi
  done
  COMBOS+=("${combo%,}")
done
note "feature combinations to verify: ${#COMBOS[@]}"

# ----------------------------------------------------------------- C build ----
note "building the C reference shared library"
if (mkdir -p c_src/build && cd c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null) 2>&1 | tail -3; then
  ok "C libdriver.so built"
else
  bad "C build failed"; exit 1
fi
C_SO=c_src/build/libdriver.so
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > "$TD/c_syms"

# ------------------------------------------------------------- the matrix ----
for combo in "${COMBOS[@]}"; do
  featflag=(--no-default-features)
  label="{}"
  if [[ -n $combo ]]; then featflag+=(--features "$combo"); label="{$combo}"; fi

  for profile in dev release; do
    profflag=(); outdir=target/debug
    if [[ $profile == release ]]; then profflag=(--release); outdir=target/release; fi
    note "features=$label profile=$profile"

    if timeout 300 cargo check --offline "${featflag[@]}" "${profflag[@]}" \
         >"$TD/chk" 2>&1; then ok "cargo check"
    else bad "cargo check"; tail -20 "$TD/chk"; continue; fi

    if timeout 300 cargo build --offline "${featflag[@]}" "${profflag[@]}" \
         >"$TD/bld" 2>&1; then ok "cargo build"
    else bad "cargo build"; tail -20 "$TD/bld"; continue; fi

    # -- Phase D: symbol parity against the C .so ---------------------------
    R_SO=$outdir/libdriver.so
    if [[ ! -f $R_SO ]]; then bad "missing $R_SO"; continue; fi
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > "$TD/r_syms"
    missing=$(comm -23 "$TD/c_syms" "$TD/r_syms")
    if [[ -z $missing ]]; then
      ok "symbol parity ($(wc -l < "$TD/c_syms" | tr -d ' ') C symbols, 0 missing)"
    else
      bad "symbols missing from Rust .so:"; printf '        %s\n' $missing
    fi
    # No non-libc undefined symbols.
    undef=$(nm -D --undefined-only "$R_SO" | awk '$1=="U"{print $2}' \
            | grep -vE '@GLIBC|@GCC|^_Unwind|^__|^(abort|bcmp|calloc|close|free|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|printf|read|readlink|realloc|realpath|statx|strlen|syscall|write|writev)$')
    if [[ -z $undef ]]; then ok "no unresolved non-libc symbols"
    else bad "unresolved non-libc symbols:"; printf '        %s\n' $undef; fi

    # -- Phases B & C: differential tests ----------------------------------
    if timeout 600 cargo test --offline "${featflag[@]}" "${profflag[@]}" \
         -- --test-threads=1 >"$TD/tst" 2>&1; then
      ok "differential tests: $(grep -h '^test result:' "$TD/tst" | tr '\n' ' ')"
    else
      bad "differential tests"; grep -E '^(test |---- |thread |assertion)' "$TD/tst" | head -30
    fi
  done
done

rm -rf "$TD"
echo
if [[ $fail -eq 0 ]]; then echo "${GRN}ALL CONFIGURATIONS PASS${RST}"; else echo "${RED}FAILURES PRESENT${RST}"; fi
exit $fail

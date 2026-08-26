#!/usr/bin/env bash
# Phase D driver: run `cargo check` and the whole differential suite for EVERY
# build configuration.
#
# Configurations come from two axes:
#   * the power set of `[features]` in Cargo.toml (there is no `[features]`
#     table, so this is the single empty combination — enumerated mechanically
#     rather than assumed, so the script keeps working if features are added);
#   * the cargo profile, because `[profile.release] panic = "abort"` makes the
#     release binary a genuinely different artifact from the dev one.
#
# Usage: ./run_all_configs.sh [--quick]
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CARGO_FLAGS=(--offline)
QUICK=0
[[ "${1:-}" == "--quick" ]] && QUICK=1

fail=0
log() { printf '%s\n' "$*"; }
hr() { printf '=%.0s' {1..78}; printf '\n'; }

# ---------------------------------------------------------------- feature combos
mapfile -t COMBOS < <(python3 - <<'PY'
import itertools, re, sys

text = open("Cargo.toml").read()
section = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
feats = []
if section:
    for line in section.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        m = re.match(r'^([A-Za-z0-9_.-]+)\s*=', line)
        if m and m.group(1) != "default":
            feats.append(m.group(1))

# The power set, smallest first; the empty set is the no-default-features build.
for r in range(len(feats) + 1):
    for combo in itertools.combinations(feats, r):
        print(",".join(combo))
PY
)

log "Feature combinations discovered: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do log "  - '${c:-<none>}'"; done

# ------------------------------------------------------------- C reference build
hr
log "Building the C reference implementation"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { log "FAILED: C build"; exit 1; }
log "C reference: $(ls -l c_src/build/driver | awk '{print $5}') bytes"

# --------------------------------------------------------------------- the matrix
PROFILES=(dev release)
(( QUICK )) && PROFILES=(dev)

for combo in "${COMBOS[@]}"; do
  feat_args=(--no-default-features)
  [[ -n "$combo" ]] && feat_args+=(--features "$combo")

  for profile in "${PROFILES[@]}"; do
    prof_args=()
    [[ "$profile" == release ]] && prof_args=(--release)
    label="features='${combo:-<none>}' profile=$profile"

    hr
    log "### cargo check   $label"
    if ! timeout 600 cargo check "${CARGO_FLAGS[@]}" "${feat_args[@]}" \
         "${prof_args[@]}" --all-targets 2>&1 | tail -n 5; then
      log "FAILED: cargo check ($label)"; fail=1; continue
    fi

    log "### cargo test    $label"
    if ! timeout 600 cargo test "${CARGO_FLAGS[@]}" "${feat_args[@]}" \
         "${prof_args[@]}" 2>&1 | grep -E 'test result|^test .* FAILED|panicked|^error'; then
      log "FAILED: cargo test ($label)"; fail=1; continue
    fi

    # `grep` above hides the exit status of cargo; re-check it explicitly.
    if ! timeout 600 cargo test "${CARGO_FLAGS[@]}" "${feat_args[@]}" \
         "${prof_args[@]}" >/dev/null 2>&1; then
      log "FAILED: cargo test returned non-zero ($label)"; fail=1
    fi
  done
done

# ------------------------------------------------------- symbol parity (Phase D)
hr
log "### Symbol parity: nm -D --defined-only"
c_syms=$(nm -D --defined-only c_src/build/driver | awk '{print $NF}' | sort)
for profile in "${PROFILES[@]}"; do
  bin="target/debug/driver"
  [[ "$profile" == release ]] && bin="target/release/driver"
  [[ -x "$bin" ]] || continue
  r_syms=$(nm -D --defined-only "$bin" | awk '{print $NF}' | sort)
  missing=$(comm -23 <(printf '%s\n' "$c_syms") <(printf '%s\n' "$r_syms"))
  if [[ -n "$missing" ]]; then
    log "FAILED: symbols exported by C but missing from $bin:"; log "$missing"; fail=1
  else
    log "OK: $bin exports every symbol the C .so exports (diff empty)"
  fi
  log "    C defined dynamic symbols:    $(printf '%s' "$c_syms" | grep -c . || true)"
  log "    Rust defined dynamic symbols: $(printf '%s' "$r_syms" | grep -c . || true)"
  nonlibc=$(nm -D --undefined-only "$bin" | awk '{print $NF}' | sed 's/@.*//' \
    | grep -vE '^(_ITM_|__gmon_start__|_Unwind_|pthread_|__pthread_)' \
    | grep -vxE '__libc_start_main|__errno_location|__cxa_finalize|__cxa_thread_atexit_impl|__tls_get_addr|__xpg_strerror_r|abort|bcmp|calloc|close|dl_iterate_phdr|dup|fcntl|free|fstat64|fstat|getauxval|getcwd|getenv|gettid|lseek|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap64|mprotect|munmap|open64|pause|poll|posix_memalign|puts|printf|read|readlink|realloc|realpath|scanf|__isoc99_scanf|sigaction|sigaltstack|signal|stat64|stat|statx|strlen|syscall|sysconf|write|writev|_ITM_deregisterTMCloneTable|_ITM_registerTMCloneTable')
  if [[ -n "$nonlibc" ]]; then
    log "FAILED: $bin has unresolved non-libc symbols:"; log "$nonlibc"; fail=1
  else
    log "OK: $bin has 0 unresolved non-libc symbols"
  fi
done

hr
if (( fail )); then
  log "RESULT: FAILURES PRESENT"
  exit 1
fi
log "RESULT: all configurations passed"

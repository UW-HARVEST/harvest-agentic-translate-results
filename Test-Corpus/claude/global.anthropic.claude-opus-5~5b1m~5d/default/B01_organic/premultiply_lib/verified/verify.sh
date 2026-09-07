#!/usr/bin/env bash
# Phase D driver: symbol parity + full Phase B/C suite under every feature
# combination and every build profile / C optimization level.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CBUILD="$ROOT/c_src/build"
fail=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf 'PASS  %s\n' "$*"; }
bad()  { printf 'FAIL  %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
step "Build C shared library (as documented, untouched CMakeLists)"
mkdir -p "$CBUILD"
( cd "$CBUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
# The cmake-produced library only; `alt_O*.so` are the supplementary builds
# created further down and must never be mistaken for the ground truth.
C_SO="$(find "$CBUILD" -maxdepth 1 -name '*.so' -not -name 'alt_O*.so' | sort | head -1)"
[ -n "$C_SO" ] || { bad "no C .so produced"; exit 1; }
ok "C .so = $C_SO"

# ---------------------------------------------------------------------------
step "Enumerate feature combinations from Cargo.toml"
FEATURES="$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);if(a[1]!="default")print a[1]}' "$CRATE/Cargo.toml")"
if [ -z "$FEATURES" ]; then
  echo "no [features] table -> the only configuration is the default one"
  COMBOS=("default" "no-default")
else
  COMBOS=("default" "no-default")
  for f in $FEATURES; do COMBOS+=("no-default:$f"); done
  COMBOS+=("all")
fi
printf 'combinations: %s\n' "${COMBOS[*]}"

# ---------------------------------------------------------------------------
run_combo() {
  local combo="$1" profile="$2"
  local -a fflags=()
  case "$combo" in
    default)     fflags=() ;;
    no-default)  fflags=(--no-default-features) ;;
    all)         fflags=(--all-features) ;;
    no-default:*) fflags=(--no-default-features --features "${combo#no-default:}") ;;
  esac
  local -a pflags=()
  [ "$profile" = release ] && pflags=(--release)

  ( cd "$CRATE" && cargo build "${fflags[@]}" "${pflags[@]}" >/dev/null 2>&1 ) \
    || { bad "cargo build [$combo/$profile]"; return; }
  local rso; rso="$(find "$CRATE/target/$profile" -maxdepth 1 -name 'libpremultiply_lib.so' | head -1)"
  [ -n "$rso" ] || { bad "no Rust .so [$combo/$profile]"; return; }

  # -- symbol parity -------------------------------------------------------
  local d
  d="$(diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' \
              | grep -vE '^(_init|_fini|__bss_start|_edata|_end)$' | sort -u) \
            <(nm -D --defined-only "$rso" | awk '{print $NF}' \
              | grep -vE '^(_init|_fini|__bss_start|_edata|_end|__rust_.*|rust_eh_personality)$' | sort -u))"
  if [ -n "$d" ]; then
    bad "symbol parity [$combo/$profile]"; echo "$d"
  else
    ok "symbol parity [$combo/$profile]"
  fi

  # -- undefined non-libc symbols in the Rust .so --------------------------
  local undef
  undef="$(nm -D --undefined-only "$rso" | awk '{print $NF}' \
           | grep -vE '^(_+ITM_|__gmon_start__|__cxa_|_Unwind_|__tls_get_addr)' \
           | grep -vE '@GLIBC|@GCC' || true)"
  if [ -n "$undef" ]; then
    bad "unresolved non-libc symbols [$combo/$profile]"; echo "$undef"
  else
    ok "no unresolved non-libc symbols [$combo/$profile]"
  fi

  # -- full Phase B + Phase C suite against THIS .so pair ------------------
  for cso_label in stock O2 O3; do
    local cso="$C_SO"
    case "$cso_label" in
      O2) cso="$CBUILD/alt_O2.so" ;;
      O3) cso="$CBUILD/alt_O3.so" ;;
    esac
    [ -f "$cso" ] || continue
    if ( cd "$CRATE" && PREMULTIPLY_RUST_SO="$rso" PREMULTIPLY_C_SO="$cso" \
           timeout 600 cargo test "${fflags[@]}" --tests -- --test-threads=4 \
           >"$CRATE/target/test-$combo-$profile-$cso_label.log" 2>&1 ); then
      ok "Phase B+C [$combo/$profile vs C:$cso_label]"
    else
      bad "Phase B+C [$combo/$profile vs C:$cso_label]"
      tail -30 "$CRATE/target/test-$combo-$profile-$cso_label.log"
    fi
  done
}

# ---------------------------------------------------------------------------
step "Supplementary C builds at higher optimization levels"
# The stock cmake build (no CMAKE_BUILD_TYPE) is the ground truth; these extra
# builds only confirm the Rust matches the C's float semantics regardless of how
# aggressively the C is optimized/vectorized. c_src is NOT modified.
for lvl in 2 3; do
  if cc -shared -fPIC -O$lvl -I"$ROOT/c_src/include" \
       "$ROOT/c_src/src/lib.c" -o "$CBUILD/alt_O$lvl.so" 2>/dev/null; then
    ok "built alt_O$lvl.so"
  else
    echo "skip: could not build alt_O$lvl.so"
  fi
done

step "Check for a driver binary in the project"
if grep -qE 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  bad "CMakeLists declares an executable -- stdout comparison required"
else
  ok "no add_executable in c_src/CMakeLists.txt (library only)"
fi
if [ -f "$CRATE/src/main.rs" ] || grep -q '^\[\[bin\]\]' "$CRATE/Cargo.toml"; then
  bad "Rust crate declares a binary but the C project does not"
else
  ok "no Rust binary target (crate-type = cdylib only) -> no stdout to compare"
fi

# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    step "combo=$combo profile=$profile"
    run_combo "$combo" "$profile"
  done
done

step "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL PHASE D CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"

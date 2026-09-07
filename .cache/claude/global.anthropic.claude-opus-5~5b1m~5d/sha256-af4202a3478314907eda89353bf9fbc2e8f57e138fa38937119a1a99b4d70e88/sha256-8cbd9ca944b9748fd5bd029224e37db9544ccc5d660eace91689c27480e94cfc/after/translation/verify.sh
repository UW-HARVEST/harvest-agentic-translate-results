#!/usr/bin/env bash
# Full verification driver.
#
#   ./verify.sh            run every phase against every build configuration
#   ./verify.sh mutate     additionally run the mutation sensitivity check
#
# Every cargo invocation is bounded by `timeout`.

set -uo pipefail
cd "$(dirname "$0")"

ROOT="$(cd .. && pwd)"
BAK=".bak"
mkdir -p "$BAK"
FAILED=0

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
# 0. Build the C reference shared object
# ---------------------------------------------------------------------------
step "Building the C reference .so"
( mkdir -p "$ROOT/c_src/build" \
  && cd "$ROOT/c_src/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) \
  && ok "C .so built" || bad "C .so build"
C_SO="$(ls "$ROOT"/c_src/build/lib*.so 2>/dev/null | head -1)"
[ -n "$C_SO" ] && ok "C .so = $C_SO" || bad "no C .so found"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
step "Enumerating cargo feature combinations"
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);if(a[1]!="default")print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  ok "no [features] declared -> configurations: {default} and {--no-default-features}"
  COMBOS=("default" "none")
else
  # Full power set of declared features.
  COMBOS=("default" "none")
  set -- $FEATURES
  n=$#
  for ((m=1; m<(1<<n); m++)); do
    combo=""
    for ((b=0; b<n; b++)); do
      if (( m & (1<<b) )); then
        eval "f=\${$((b+1))}"
        combo="${combo:+$combo,}$f"
      fi
    done
    COMBOS+=("feat:$combo")
  done
  ok "features: $(echo $FEATURES | tr '\n' ' ')"
fi

cargo_flags_for() {
  case "$1" in
    default) echo "" ;;
    none)    echo "--no-default-features" ;;
    feat:*)  echo "--no-default-features --features ${1#feat:}" ;;
  esac
}

# ---------------------------------------------------------------------------
# 2. Per configuration x per profile: build, symbol diff, run all phases
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  FLAGS=$(cargo_flags_for "$combo")
  for profile in debug release; do
    PFLAG=""; [ "$profile" = release ] && PFLAG="--release"
    step "config=$combo profile=$profile"

    timeout 600 cargo build --offline $PFLAG $FLAGS >/dev/null 2>&1 \
      && ok "cargo build" || { bad "cargo build ($combo/$profile)"; continue; }

    R_SO="target/$profile/libtfm_lib.so"
    [ -f "$R_SO" ] && ok "rust .so = $R_SO" || { bad "missing $R_SO"; continue; }

    # --- Phase D symbol parity -------------------------------------------
    diff <(nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort) \
         <(nm -D --defined-only "$R_SO" | awk '$2=="T"{print $3}' | sort) >/dev/null \
      && ok "symbol diff empty" || { bad "symbol diff non-empty ($combo/$profile)"; \
           diff <(nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort) \
                <(nm -D --defined-only "$R_SO" | awk '$2=="T"{print $3}' | sort); }

    # Every undefined symbol in the Rust .so must be resolvable libc/libgcc.
    UNRES=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
            | grep -vE '^(__|_ITM_|_Unwind_|GLIBC)' \
            | grep -vE '@GLIBC|^(malloc|free|realloc|calloc|memcpy|memmove|memset|memcmp|abort|write|writev|open|close|read|mmap|munmap|mprotect|sysconf|getenv|dl_iterate_phdr|pthread_[a-z_]*|sqrtf|strlen|bcmp|posix_memalign|syscall|gettid|statx|poll|sigaction|sigaltstack|sigemptyset|readlink|getcwd|environ|stat|fstat|lstat|realpath|nanosleep|clock_gettime|pipe2|dup2|execvp|fork|waitpid|kill|raise|exit|_exit|atexit|qsort|strerror_r|snprintf|vsnprintf|fwrite|fputs|fflush|stdout|stderr|isatty|dlsym|dlopen|dlclose|dlerror|dladdr|getrandom|memrchr|strchr|munmap)$' \
            | sort -u)
    if [ -z "$UNRES" ]; then ok "no unexpected undefined symbols"; else
      echo "$UNRES" | sed 's/^/      /'; bad "unexpected undefined symbols ($combo/$profile)"
    fi

    # --- Phases A/B/C: run the whole suite against THIS .so ---------------
    # Tests always run in the dev profile so panics unwind and each failing
    # test is reported individually; TFM_RUST_SO aims them at the .so above.
    TFM_RUST_SO="$PWD/$R_SO" timeout 600 cargo test --offline $FLAGS 2>&1 | tail -40 > "$BAK/last.log"
    if grep -qE 'test result: FAILED|error\[|panicked' "$BAK/last.log"; then
      sed 's/^/      /' "$BAK/last.log"
      bad "test suite ($combo/$profile)"
    else
      grep -E '^test result:' "$BAK/last.log" | sed 's/^/      /'
      ok "all phases pass against $R_SO"
    fi
  done
done

# ---------------------------------------------------------------------------
# 3. Binary executable comparison
# ---------------------------------------------------------------------------
step "Binary (driver) comparison"
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt" || grep -q '\[\[bin\]\]' Cargo.toml; then
  bad "a binary exists but is not compared -- add a stdout diff"
else
  ok "no executable in c_src/CMakeLists.txt and no [[bin]] in Cargo.toml (vacuous)"
fi

# ---------------------------------------------------------------------------
# 4. Optional: mutation sensitivity of the differential suite
# ---------------------------------------------------------------------------
if [ "${1:-}" = mutate ]; then
  step "Mutation sensitivity (each planted bug MUST be caught)"
  cp src/lib.rs "$BAK/lib.rs.orig"
  restore() { cp "$BAK/lib.rs.orig" src/lib.rs; }
  trap restore EXIT

  mutate() {
    desc="$1"; expr="$2"
    restore
    perl -0pi -e "$expr" src/lib.rs
    if cmp -s "$BAK/lib.rs.orig" src/lib.rs; then bad "mutation patch did not apply: $desc"; return; fi
    if ! timeout 600 cargo build --offline --release >/dev/null 2>&1; then
      bad "mutation did not compile: $desc"; restore; return
    fi
    TFM_RUST_SO="$PWD/target/release/libtfm_lib.so" \
      timeout 600 cargo test --offline >/dev/null 2>&1
    rc=$?
    if [ "$rc" -ne 0 ]; then ok "caught (exit $rc): $desc"; else bad "NOT CAUGHT: $desc"; fi
    restore
  }

  mutate "clamp -> f32::max (wrong NaN / -0.0 clamp semantics)" \
    's/if 0\.0f32 > sqd \{\n        0\.0f32\n    \} else \{\n        sqd\n    \}/sqd.max(0.0f32)/s'
  mutate "branch predicate < -> <=" \
    's/if s0 < s1 \{/if s0 <= s1 {/'
  mutate "branch predicate < -> >" \
    's/if s0 < s1 \{/if s0 > s1 {/'
  mutate "else-branch dx2\/dy2 bindings swapped" \
    's/            let dy2: f32 = s0;\n            let dx2: f32 = s1;/            let dy2: f32 = s1;\n            let dx2: f32 = s0;/s'
  mutate "if-branch output slots swapped" \
    's/\*dest\.add\(0\) = fsub\(dx2, lambda\);\n                \*dest\.add\(1\) = dxy;/*dest.add(0) = dxy;\n                *dest.add(1) = fsub(dx2, lambda);/s'
  mutate "sqd final add operands commuted" \
    's/fadd\(dxy_term, acc\)/fadd(acc, dxy_term)/'
  mutate "count compared as unsigned (negative -> huge loop)" \
    's/while i < count \{/while (i as u32) < (count as u32) {/'
  mutate "loop guard <= (one extra iteration)" \
    's/while i < count \{/while i <= count {/'
  mutate "dest stride 2 -> 3" \
    's/dest = unsafe \{ dest\.add\(2\) \};/dest = unsafe { dest.add(3) };/'
  mutate "src stride 3 -> 2" \
    's/src = unsafe \{ src\.add\(3\) \};/src = unsafe { src.add(2) };/'
  # NB: the perturbation must exceed half an ulp or it rounds back to the same
  # f32 and the "mutation" is a no-op. ulp(2.0f)=2.38e-7, so use 2.0000005.
  mutate "2.0f coefficient -> 2.0000005 (1 ulp)" \
    's/fmul\(2\.0f32, dx2\)/fmul(2.0000005f32, dx2)/'
  mutate "4.0f coefficient -> 4.0000005" \
    's/fmul\(4\.0f32, dxy\)/fmul(4.0000005f32, dxy)/'
  mutate "0.5f coefficient -> 0.49999997" \
    's/fmul\(0\.5f32, fadd/fmul(0.49999997f32, fadd/'
  mutate "NaN selection: prefer SOURCE operand over DESTINATION" \
    's/    if a\.is_nan\(\) \{\n        \/\/ Destination operand is NaN: it wins, quieted\.\n        quiet\(a\)\n    \} else if b\.is_nan\(\) \{/    if b.is_nan() {\n        quiet(b)\n    } else if a.is_nan() {/s'
  mutate "quiet() also clears the sign bit" \
    's/f32::from_bits\(x\.to_bits\(\) \| 0x0040_0000\)/f32::from_bits((x.to_bits() | 0x0040_0000) \& 0x7fff_ffff)/'
  mutate "INDEFINITE loses its sign bit" \
    's/const INDEFINITE: u32 = 0xFFC0_0000;/const INDEFINITE: u32 = 0x7FC0_0000;/'
  mutate "fsub(dx2, lambda) operands commuted" \
    's/fsub\(dx2, lambda\)/fsub(lambda, dx2)/g'
  # NOTE: deliberately NOT mutated -- `fsqrt`'s negative-argument branch is
  # provably DEAD. `clamp_nonneg_c` returns `0.0f32` whenever `0.0 > sqd`, so
  # `fsqrt` only ever sees a value that is `>= 0.0`, `-0.0`, or NaN. Perturbing
  # that branch is therefore a semantic no-op and cannot be "caught" by any
  # differential test -- which is correct, not a coverage gap. The equivalent
  # *reachable* behaviours (clamped negative sqd, -0.0, NaN) are covered by
  # ERRORS.md rows 9, 10 and 11.
  mutate "sqrt of NON-negative also returns INDEFINITE (reachable guard flipped)" \
    's/    if x < 0\.0f32 \{/    if !(x < 0.0f32) {/'
  mutate "computation done in f64 then rounded" \
    's/let acc: f32 = fadd\(/let acc: f32 = ((dy2 as f64 * dy2 as f64) - (2.0f64 * dx2 as f64 * dy2 as f64) + (dx2 as f64 * dx2 as f64)) as f32; let _unused = fadd(/'

  restore
  timeout 600 cargo build --offline --release >/dev/null 2>&1
  cmp -s "$BAK/lib.rs.orig" src/lib.rs && ok "src/lib.rs restored byte-identically" \
    || bad "src/lib.rs NOT restored"
  trap - EXIT
fi

step "RESULT"
if [ "$FAILED" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAILED"

#!/usr/bin/env bash
# Mutation check: prove the differential suite has teeth.
#
# Each mutation is a small, plausible translation mistake applied to the Rust
# source. The suite MUST fail for every one of them; if a mutation survives, the
# corresponding behaviour is untested. The original sources are restored at the
# end (and on interrupt).
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

BACKUP="$(mktemp -d)"
cp -r src "$BACKUP/src"
restore() { rm -rf src && cp -r "$BACKUP/src" src; }
trap 'restore; rm -rf "$BACKUP"' EXIT INT TERM

export DIFF_ITERS="${DIFF_ITERS:-500}"

# file | sed expression | description
MUTATIONS=(
  "src/match.rs|s#sum / N_SMOOTH as f64#sum / 15.0#|smoothen: wrong kernel divisor (15 instead of N_SMOOTH)"
  "src/match.rs|s#(contrast >= threshold)#(contrast > threshold)#|match: contrast gate >= becomes >"
  "src/match.rs|s#if total(test_in) < mul_sd#if total(test_in) <= mul_sd#|match: energy gate < becomes <="
  "src/match.rs|s#j < N_SMOOTH \&\& i + j < length#j < N_SMOOTH \&\& i + j + 1 < length#|smoothen: off-by-one window bound"
  "src/spectral_contrast.rs|s#v\[i\] = ((v\[i\] as f64) / magnitude) as f32#v[i] = v[i] / (magnitude as f32)#|normalize: divide in f32 instead of widening to f64"
  "src/spectral_contrast.rs|s#add_sd(mul_ss(b\[i\], a\[i\]) as f64, sum)#add_sd(sum, mul_ss(b[i], a[i]) as f64)#|dot_product: swapped ADDSD destination operand"
  "src/spectral_contrast.rs|s#mul_ss(b\[i\], a\[i\])#mul_ss(a[i], b[i])#|dot_product: swapped MULSS destination operand"
  "src/spectral_contrast.rs|s#dot_product(v, v).sqrt()#dot_product(v, v)#|normalize: missing sqrt"
)

# Mutants that are EXPECTED to survive because they are equivalent through the
# public ABI. Listing them here (rather than quietly omitting them) means the
# reasoning is checked: if one is ever killed, the reasoning was wrong.
#
#   differentiate: reversed difference (`v[i]-v[i+1]` instead of `v[i+1]-v[i]`)
#     Negates every element of the preprocessed `double` buffer (the following
#     `smoothen` is linear). Negating a `double` flips bit 63 only, which lives
#     in the *high* 32-bit word, so of the `float` lanes `spectral_contrast`
#     reads, the even ones are bit-identical and the odd ones have their sign
#     bit flipped -- and `t` and `r` are affected at exactly the same lanes.
#     `normalize` is sign-symmetric, so every product `a[j]*b[j]` in
#     `dot_product` is bit-identical (both factors flip, or neither), the
#     magnitudes are unchanged (squares), and the contrast comes out
#     bit-identical. Only the sign of a `±0.0` contrast could differ, and
#     `-0.0 >= x` and `+0.0 >= x` agree. `differentiate` is `static` and is
#     unreachable from the `spectral_contrast` export, so no caller can observe
#     it. Verified to survive 30000 randomized inputs per CONFIGS.md row.
EQUIVALENT=(
  "src/match.rs|s#v\[i\] = v\[i + 1\] - v\[i\]#v[i] = v[i] - v[i + 1]#|differentiate: reversed difference (ABI-equivalent)"
)

survived=0
killed=0
unexpected=0

# Apply one mutation, build, run the suite. Echoes "killed" or "survived".
run_mutant() {
  local file="$1" expr="$2" desc="$3"
  restore
  local before after
  before="$(md5sum "$file" | cut -d' ' -f1)"
  sed -i "$expr" "$file"
  after="$(md5sum "$file" | cut -d' ' -f1)"
  if [[ "$before" == "$after" ]]; then
    echo "setup-fail"
    return
  fi
  if ! timeout 600 cargo build --release >/dev/null 2>&1; then
    echo "no-compile"
    return
  fi
  if timeout 600 cargo test --release --test configs --test errors >/dev/null 2>&1; then
    echo "survived"
  else
    echo "killed"
  fi
}

echo "== mutants that MUST be detected =="
for m in "${MUTATIONS[@]}"; do
  file="${m%%|*}"; rest="${m#*|}"
  expr="${rest%%|*}"; desc="${rest#*|}"
  outcome="$(run_mutant "$file" "$expr" "$desc")"
  case "$outcome" in
    killed)   echo "  killed      $desc"; killed=$((killed + 1)) ;;
    survived) echo "  SURVIVED    $desc"; survived=$((survived + 1)) ;;
    *)        echo "  SETUP-FAIL  $desc ($outcome)"; survived=$((survived + 1)) ;;
  esac
done

echo
echo "== mutants expected to be ABI-equivalent (must survive; see comments) =="
for m in "${EQUIVALENT[@]}"; do
  file="${m%%|*}"; rest="${m#*|}"
  expr="${rest%%|*}"; desc="${rest#*|}"
  outcome="$(run_mutant "$file" "$expr" "$desc")"
  case "$outcome" in
    survived) echo "  as expected $desc" ;;
    killed)   echo "  UNEXPECTED  $desc — killed, so the equivalence argument is wrong"
              unexpected=$((unexpected + 1)) ;;
    *)        echo "  SETUP-FAIL  $desc ($outcome)"; unexpected=$((unexpected + 1)) ;;
  esac
done

restore
timeout 600 cargo build --release >/dev/null 2>&1

echo
echo "must-detect mutants killed: $killed   survived: $survived"
echo "equivalent mutants behaving unexpectedly: $unexpected"
if (( survived || unexpected )); then
  echo "RESULT: FAIL"
  exit 1
fi
echo "RESULT: PASS — every non-equivalent mutant was detected."

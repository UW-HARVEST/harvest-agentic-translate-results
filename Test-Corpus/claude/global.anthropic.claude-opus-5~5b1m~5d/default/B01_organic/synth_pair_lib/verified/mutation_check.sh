#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Sanity check for the differential test suite (NOT part of verification
# itself): inject a deliberate bug into src/lib.rs, confirm the suite FAILS,
# then restore. A mutation that SURVIVES means the suite has a blind spot.
#
# The backup lives under target/ (sandbox-writable) and is restored on every
# exit path, including Ctrl-C.
# ---------------------------------------------------------------------------
set -u
cd "$(dirname "$0")"

BACKUP=target/lib.rs.mutation-backup
mkdir -p target
cp src/lib.rs "$BACKUP" || { echo "FATAL: cannot create backup"; exit 1; }
[ -s "$BACKUP" ] || { echo "FATAL: backup is empty"; exit 1; }

restore() { cp "$BACKUP" src/lib.rs; }
trap 'restore; exit 130' INT TERM
trap 'restore' EXIT

# Fast suite: the randomized CONFIGS/ERRORS rows + symbol parity (~5 s).
run_suite() {
  cargo build --offline --release >/dev/null 2>&1 || return 2
  cargo test --offline --release --test configs --test errors --test symbols --test rounding_mode \
      >/dev/null 2>&1 && return 0 || return 1
}

# Escalation: the exhaustive 2^32 sweeps (~60 s). Only used on mutants that
# survive the fast suite, to decide "blind spot" vs "provably equivalent".
run_exhaustive() {
  cargo test --offline --release --test exhaustive >/dev/null 2>&1 && return 0 || return 1
}

# Confirm the baseline is green before trusting any "KILLED" verdict.
if ! run_suite || ! run_exhaustive; then
  echo "FATAL: baseline suite does not pass; fix that first."
  exit 1
fi
echo "baseline: PASS (fast + exhaustive)"
echo

declare -a NAMES=() SEDS=()
add() { NAMES+=("$1"); SEDS+=("$2"); }

# --- weights / constants -------------------------------------------------
add "lane0 weight 29 -> 28"              's/\* 29f32/\* 28f32/'
add "lane0 weight 213 -> 212"            's/\* 213f32/\* 212f32/'
add "lane0 weight 459 -> 458"            's/\* 459f32/\* 458f32/'
add "lane0 weight 2037 -> 2036"          's/\* 2037f32/\* 2036f32/'
add "lane0 weight 5153 -> 5152"          's/\* 5153f32/\* 5152f32/'
add "lane0 weight 6574 -> 6573"          's/\* 6574f32/\* 6573f32/'
add "lane0 weight 37489 -> 37488"        's/\* 37489f32/\* 37488f32/'
add "lane0 weight 75038 -> 75037"        's/\* 75038f32/\* 75037f32/'
add "lane1 weight 104 -> 103"            's/\* 104f32/\* 103f32/'
add "lane1 weight 1567 -> 1566"          's/\* 1567f32/\* 1566f32/'
add "lane1 weight 9727 -> 9726"          's/\* 9727f32/\* 9726f32/'
add "lane1 weight 64019 -> 64018"        's/\* 64019f32/\* 64018f32/'
add "lane1 weight -9975 sign flip"       's/\* -9975f32/\* 9975f32/'
add "lane1 weight -45 sign flip"         's/\* -45f32/\* 45f32/'
add "lane1 weight 146 -> 145"            's/\* 146f32/\* 145f32/'
add "lane1 weight -5 sign flip"          's/\* -5f32/\* 5f32/'

# --- mp3d_scale_pcm guards & rounding ------------------------------------
add "hi guard >= -> >"                   's/>= 32766\.5/> 32766.5/'
add "lo guard <= -> <"                   's/if sample as f64 <= -32767\.5/if (sample as f64) < -32767.5/'
add "hi guard const 32766.5 -> 32767.5"  's/>= 32766\.5/>= 32767.5/'
add "lo guard const -32767.5 -> -32766.5" 's/<= -32767\.5/<= -32766.5/'
add "hi clamp 32767 -> 32766"            's/return 32767i16;/return 32766i16;/'
add "lo clamp -32768 -> -32767"          's/return -32768i16;/return -32767i16;/'
add "drop the negative bias correction"  's/s\.wrapping_sub((s < 0) as i16)/s/'
add "bias on s <= 0 instead of s < 0"    's/(s < 0) as i16/(s <= 0) as i16/'
add "round-half instead of truncate"     's/(sample + 0\.5f32) as i32 as i16/sample.round() as i32 as i16/'
add "drop the +0.5 rounding offset"      's/(sample + 0\.5f32) as i32 as i16/sample as i32 as i16/'
add "narrow via f32->i16 directly"       's/(sample + 0\.5f32) as i32 as i16/(sample + 0.5f32) as i16/'
add "NaN mapped to 1 instead of 0"       's/let s: i16 = (sample + 0\.5f32) as i32 as i16;/let s: i16 = if sample.is_nan() { 1 } else { (sample + 0.5f32) as i32 as i16 };/'

# --- tap indices / structure ---------------------------------------------
add "lane0 tap 14*64 -> 13*64"           's/g(z, 14 \* 64) } - unsafe/g(z, 13 * 64) } - unsafe/'
add "lane0 term1 subtract -> add"        's/} - unsafe { g(z, 0) }/} + unsafe { g(z, 0) }/'
add "lane0 term3 subtract -> add"        's/g(z, 12 \* 64) } - unsafe/g(z, 12 * 64) } + unsafe/'
add "lane0 sum pair 1*64 -> 13*64"       's/g(z, 1 \* 64) } + unsafe { g(z, 13 \* 64) }/g(z, 13 * 64) } + unsafe { g(z, 13 * 64) }/'
add "lane1 pointer bump 2 -> 1"          's/z\.offset(2)/z.offset(1)/'
add "lane1 pointer bump 2 -> 3"          's/z\.offset(2)/z.offset(3)/'
add "lane1 drop the pointer bump"        's/let z = unsafe { z\.offset(2) };/let z = z;/'
add "lane0 accumulation reordered"       's/a += (unsafe { g(z, 12 \* 64) } - unsafe { g(z, 2 \* 64) }) \* 459f32;/a = (unsafe { g(z, 12 * 64) } - unsafe { g(z, 2 * 64) }) * 459f32 + a;/'
add "term 4 accumulated in f64 (excess precision)" 's|a += (unsafe { g(z, 3 \* 64) } + unsafe { g(z, 11 \* 64) }) \* 2037f32;|a = (a as f64 + (unsafe { g(z, 3 * 64) } as f64 + unsafe { g(z, 11 * 64) } as f64) * 2037f64) as f32;|'
add "term 8 fused into an FMA (-ffp-contract=fast)" 's|a += unsafe { g(z, 7 \* 64) } \* 75038f32;|a = f32::mul_add(unsafe { g(z, 7 * 64) }, 75038f32, a);|'
add "lane1 term 4 fused into an FMA" 's|a += unsafe { g(z, 8 \* 64) } \* 64019f32;|a = f32::mul_add(unsafe { g(z, 8 * 64) }, 64019f32, a);|'
add "lane0 term 2 computed in f64 throughout" 's|a += (unsafe { g(z, 1 \* 64) } + unsafe { g(z, 13 \* 64) }) \* 213f32;|a = (a as f64 + (unsafe { g(z, 1 * 64) } as f64 + unsafe { g(z, 13 * 64) } as f64) * 213.0f64) as f32;|'

# --- pcm destination index ------------------------------------------------
add "nch stride 16 -> 8"                 's/16i32\.wrapping_mul(nch)/8i32.wrapping_mul(nch)/'
add "nch stride 16 -> 32"                's/16i32\.wrapping_mul(nch)/32i32.wrapping_mul(nch)/'
add "nch index without int wraparound"   's/let idx = 16i32\.wrapping_mul(nch) as isize;/let idx = 16isize * nch as isize;/'
add "lane1 store to pcm[0]"              's/let idx = 16i32\.wrapping_mul(nch) as isize;/let idx = 0isize;/'
add "swap the two pcm stores"            's/\*pcm\.offset(0) = mp3d_scale_pcm(a);/\*pcm.offset(1) = mp3d_scale_pcm(a);/'

killed=0; killed_slow=0; survived=0; skipped=0
declare -a SURVIVORS=()
for i in "${!NAMES[@]}"; do
  cp "$BACKUP" src/lib.rs
  sed -i "${SEDS[$i]}" src/lib.rs
  if cmp -s src/lib.rs "$BACKUP"; then
    printf 'NO-OP    : %s   <-- pattern did not match, fix the script\n' "${NAMES[$i]}"
    survived=$((survived+1)); SURVIVORS+=("${NAMES[$i]} (no-op)"); continue
  fi
  run_suite; rc=$?
  case $rc in
    1) printf 'KILLED   : %s\n' "${NAMES[$i]}"; killed=$((killed+1)) ;;
    2) printf 'NOCOMPILE: %s   (skipped)\n' "${NAMES[$i]}"; skipped=$((skipped+1)) ;;
    0) # Survived the fast suite -> escalate to the exhaustive 2^32 sweeps.
       if run_exhaustive; then
         printf 'SURVIVED : %s   <-- survives even the exhaustive 2^32 sweep\n' "${NAMES[$i]}"
         survived=$((survived+1)); SURVIVORS+=("${NAMES[$i]}")
       else
         printf 'KILLED*  : %s   (only by the exhaustive sweep)\n' "${NAMES[$i]}"
         killed_slow=$((killed_slow+1))
       fi ;;
  esac
done

restore
run_suite || { echo "FATAL: suite not green after restore!"; exit 1; }

echo
echo "-------------------------------------------"
echo "killed by fast suite       : $killed"
echo "killed only by exhaustive  : $killed_slow"
echo "survived everything        : $survived"
echo "did not compile (skipped)  : $skipped"
if [ "$survived" -gt 0 ]; then
  echo
  echo "Survivors (each must be justified as a PROVABLY EQUIVALENT mutant in"
  echo "EQUIVALENT_MUTANTS.md, or it is a real gap in the test suite):"
  for s in "${SURVIVORS[@]}"; do echo "  - $s"; done
fi
echo "baseline restored and green."
exit 0

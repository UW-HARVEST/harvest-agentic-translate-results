#!/usr/bin/env bash
# Full differential verification of the Rust translation against the C original.
#
#   ./verify.sh
#
# 1. builds the C shared libraries with CMake (default options, matching
#    c_src/CMakeLists.txt)
# 2. enumerates every Cargo feature combination declared in Cargo.toml
# 3. for each combination × build profile: builds the Rust cdylib, diffs the
#    exported dynamic symbols against the C .so, and runs the whole test suite
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
tmp="${TMPDIR:-/tmp}"
fail=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad() { printf '\033[31mFAIL\033[0m %s\n' "$*"; fail=1; }
ok()  { printf '\033[32mok\033[0m   %s\n' "$*"; }

# ---------------------------------------------------------------------------
say "building the C reference implementation"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$tmp/cmake.log" 2>&1 \
  && cmake --build . >"$tmp/cbuild.log" 2>&1 ) \
  || { bad "C build failed; see $tmp/cbuild.log"; tail -20 "$tmp/cbuild.log"; exit 1; }
ok "libcjson.so + libcJSON_test.so"

c_syms="$tmp/c_syms.txt"
{ nm -D --defined-only "$root/c_src/build/libcjson.so" | awk '{print $3}'
  nm -D --defined-only "$root/c_src/build/libcJSON_test.so" | awk '{print $3}'
} | sort -u >"$c_syms"
ok "$(wc -l <"$c_syms") exported C symbols"

# ---------------------------------------------------------------------------
# Feature combinations: the power set of the features declared in Cargo.toml.
# (This crate declares none, so the set is {default, --no-default-features}.)
say "enumerating Cargo feature combinations"
mapfile -t features < <(
  cd "$here" && cargo metadata --no-deps --format-version 1 2>/dev/null \
    | python3 -c 'import json,sys
m=json.load(sys.stdin)
for f in sorted(m["packages"][0]["features"]):
    print(f)'
)
printf 'declared features: %s\n' "${features[*]:-<none>}"

combos=()
n=${#features[@]}
if (( n == 0 )); then
  combos=("DEFAULT" "NODEFAULT")
else
  combos=("DEFAULT")
  for (( mask=0; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && sel+=("${features[$i]}")
    done
    combos+=("NODEFAULT:$(IFS=,; echo "${sel[*]}")")
  done
fi
printf 'combinations to verify: %d\n' "${#combos[@]}"

# ---------------------------------------------------------------------------
run_combo() {
  local combo="$1" profile="$2"
  local -a flags=()
  local label="$combo/$profile"
  case "$combo" in
    DEFAULT) ;;
    NODEFAULT) flags+=(--no-default-features) ;;
    NODEFAULT:*) flags+=(--no-default-features)
                 local f="${combo#NODEFAULT:}"
                 [[ -n "$f" ]] && flags+=(--features "$f") ;;
  esac
  [[ "$profile" == release ]] && flags+=(--release)

  say "verify $label"

  ( cd "$here" && cargo build "${flags[@]}" ) >"$tmp/rbuild.log" 2>&1 \
    || { bad "$label: cargo build failed"; tail -25 "$tmp/rbuild.log"; return; }

  local so="$here/target/$profile/libcJSON_test.so"
  [[ -f "$so" ]] || { bad "$label: $so not produced"; return; }

  # ---- symbol parity -------------------------------------------------------
  nm -D --defined-only "$so" | awk '{print $3}' | sort -u >"$tmp/r_syms.txt"
  local missing
  missing="$(comm -23 "$c_syms" "$tmp/r_syms.txt")"
  if [[ -n "$missing" ]]; then
    bad "$label: symbols exported by C but MISSING from Rust:"
    printf '  %s\n' $missing
  else
    ok "$label: symbol diff empty ($(wc -l <"$tmp/r_syms.txt") Rust exports)"
  fi
  local undef
  undef="$(nm -D --undefined-only "$so" | awk '{print $2}' \
           | grep -v -E '@GLIBC|@GCC|^_ITM_|^_Unwind_|^__gmon_start__$' || true)"
  if [[ -n "$undef" ]]; then
    bad "$label: undefined non-libc symbols:"
    printf '  %s\n' $undef
  else
    ok "$label: no undefined non-libc symbols"
  fi

  # ---- differential test suite --------------------------------------------
  if ( cd "$here" && timeout 600 cargo test "${flags[@]}" ) >"$tmp/rtest.log" 2>&1; then
    ok "$label: $(grep -c '^test .* ok$' "$tmp/rtest.log") tests passed"
    grep -E '^test result:' "$tmp/rtest.log" | sed 's/^/     /'
  else
    bad "$label: cargo test failed"
    grep -E '^test .* FAILED|panicked at|^error|test result: FAILED' "$tmp/rtest.log" \
      | head -40 | sed 's/^/     /'
  fi
}

for combo in "${combos[@]}"; do
  for profile in release debug; do
    run_combo "$combo" "$profile"
  done
done

# ---------------------------------------------------------------------------
say "auditing the Phase A artifacts"
if python3 - <<'PYEOF'
import glob, os, re, sys
here = os.path.dirname(os.path.abspath("verify.sh"))
bad = []

def tests_defined():
    d = set()
    for f in glob.glob("tests/*.rs"):
        d |= set(re.findall(r"^#\[test\]\nfn\s+([A-Za-z0-9_]+)", open(f).read(), re.M))
    return d

defined = tests_defined()
print(f"  {len(defined)} #[test] functions defined")

# --- ERRORS.md -------------------------------------------------------------
rows = [l for l in open("ERRORS.md") if re.match(r"^\| \d+ \|", l)]
nums = [int(re.match(r"^\| (\d+) \|", l).group(1)) for l in rows]
if nums != list(range(1, len(rows) + 1)):
    bad.append("ERRORS.md row numbering is not contiguous")
named, hooks, unreach, none = [], [], [], []
for n, l in zip(nums, rows):
    cell = l.rstrip().rsplit("|", 2)[-2]
    if re.search(r"`err_[a-z0-9_]+`", cell):
        named.append(n)
    elif "`hooks`" in cell:
        hooks.append(n)
    elif "unreachable" in cell:
        unreach.append(n)
    else:
        none.append(n)
print(f"  ERRORS.md: {len(rows)} rows = {len(named)} named tests"
      f" + {len(hooks)} hooks-covered + {len(unreach)} documented-unreachable")
if none:
    bad.append(f"ERRORS.md rows with no status: {none}")

md = open("ERRORS.md").read()
for name in sorted(set(re.findall(r"`(err_[a-z0-9_]+)`", md))):
    if name not in defined:
        bad.append(f"ERRORS.md names {name}, which is not a #[test]")

# --- CONFIGS.md ------------------------------------------------------------
cfg = open("CONFIGS.md").read()
crows = [l for l in cfg.splitlines() if re.match(r"^\| \d+ \|", l)]
unchecked = [l for l in crows if l.rstrip().endswith("| [ ] |")]
checked = [l for l in crows if l.rstrip().endswith("| [x] |")]
print(f"  CONFIGS.md: {len(crows)} rows, {len(checked)} checked, {len(unchecked)} unchecked")
if unchecked:
    bad.append(f"CONFIGS.md has {len(unchecked)} unchecked row(s)")
if len(checked) != len(crows):
    bad.append("CONFIGS.md rows without a [x]/[ ] box")
refs = set()
for m in re.finditer(r"`([a-z_]+\.rs)::([A-Za-z0-9_{},]+)`", cfg):
    spec = m.group(2)
    if "{" in spec:
        pre, rest = spec.split("{", 1)
        inner, post = rest.split("}", 1)
        for alt in inner.split(","):
            refs.add(pre + alt + post)
    else:
        refs.update(spec.split(" + "))
for m in re.finditer(r"\+ (cfg[A-Za-z0-9_]+)", cfg):
    refs.add(m.group(1))
for name in sorted(refs):
    if name not in defined:
        bad.append(f"CONFIGS.md names {name}, which is not a #[test]")

# --- SYMBOLS.md ------------------------------------------------------------
sym = open("SYMBOLS.md").read()
if "**(none — the symbol diff is empty" not in sym:
    bad.append("SYMBOLS.md does not report an empty symbol diff")

for b in bad:
    print("  PROBLEM:", b)
sys.exit(1 if bad else 0)
PYEOF
then
  ok "SYMBOLS.md / ERRORS.md / CONFIGS.md are internally consistent"
else
  bad "artifact audit failed"
fi

say "summary"
if (( fail )); then
  echo "VERIFICATION FAILED"
  exit 1
fi
echo "ALL CONFIGURATIONS VERIFIED"

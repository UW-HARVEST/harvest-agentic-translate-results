#!/usr/bin/env bash
# Full C-vs-Rust differential verification driver (Phases A -> D).
#
#   ./run_verification.sh
#
# Phase A  build both sides, enumerate every feature combination
# Phase B  valid-path differential tests   (tests/valid_paths.rs  <- CONFIGS.md)
# Phase C  error-path differential tests   (tests/error_paths.rs  <- ERRORS.md)
# Phase D  `nm -D` symbol parity + repeat B/C under every feature combination
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(pwd)
FAIL=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# --------------------------------------------------------------- Phase A ---
step "Phase A.1 — build the C side (executable + shared object)"
mkdir -p c_src/build
(
  cd c_src/build
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >cmake.log 2>&1 &&
    cmake --build . >>cmake.log 2>&1
) || { bad "cmake build"; exit 1; }
# CMakeLists.txt only declares an executable; also produce a .so from the same
# single translation unit so the exported symbols can be diffed and dlopen'd.
gcc -shared -fPIC -o c_src/build/libdriver_c.so c_src/src/main.c ||
  { bad "gcc -shared"; exit 1; }
ok "c_src/build/driver + c_src/build/libdriver_c.so"

step "Phase A.2 — enumerate feature combinations"
# Powerset of the package's declared features. `driver` declares none, so this
# yields exactly one combination (the empty set); the loop below still works
# unchanged if features are ever added.
mapfile -t COMBOS < <(
  cargo metadata --no-deps --format-version 1 2>/dev/null |
    python3 -c '
import itertools, json, sys
feats = sorted(f for f in json.load(sys.stdin)["packages"][0]["features"] if f != "default")
for r in range(len(feats) + 1):
    for c in itertools.combinations(feats, r):
        print(",".join(c))
'
)
echo "  declared features: ${#COMBOS[@]} combination(s)"
for c in "${COMBOS[@]}"; do echo "    - '${c:-<none>}'"; done

# ------------------------------------------------- Phases B/C/D per combo ---
for COMBO in "${COMBOS[@]}"; do
  LABEL="${COMBO:-<no features>}"
  if [ -z "$COMBO" ]; then FEATFLAGS=(--no-default-features)
  else FEATFLAGS=(--no-default-features --features "$COMBO"); fi

  step "cargo check  [$LABEL]"
  if cargo check --all-targets "${FEATFLAGS[@]}" 2>&1 | tail -5; then
    ok "cargo check [$LABEL]"
  else
    bad "cargo check [$LABEL]"
  fi

  step "cargo build  [$LABEL]"
  cargo build --lib --bins --examples "${FEATFLAGS[@]}" 2>&1 | tail -3
  cargo build --release --lib --bins "${FEATFLAGS[@]}" 2>&1 | tail -3
  [ -f target/debug/libdriver.so ] && ok "target/debug/libdriver.so" ||
    bad "missing Rust cdylib"

  step "Phase D.1 — nm -D symbol parity  [$LABEL]"
  # Use a writable scratch dir: /tmp may be read-only, and an unwritable temp
  # file would make `comm` compare two empty lists and report a false PASS.
  SCRATCH=$(mktemp -d "${TMPDIR:-.}/symdiff.XXXXXX") || { bad "mktemp -d"; exit 1; }
  nm -D --defined-only c_src/build/libdriver_c.so | awk '{print $3}' | sort >"$SCRATCH/c_syms"
  C_COUNT=$(wc -l <"$SCRATCH/c_syms")
  if [ "$C_COUNT" -lt 5 ]; then
    bad "expected >=5 exported C symbols, got $C_COUNT (symbol extraction broken)"
  fi
  for so in target/debug/libdriver.so target/release/libdriver.so; do
    [ -f "$so" ] || continue
    nm -D --defined-only "$so" | awk '{print $3}' | sort >"$SCRATCH/r_syms"
    MISSING=$(comm -23 "$SCRATCH/c_syms" "$SCRATCH/r_syms")
    if [ -z "$MISSING" ] && [ "$C_COUNT" -ge 5 ]; then
      ok "$so exports all $C_COUNT C symbols ($(tr '\n' ' ' <"$SCRATCH/c_syms"))"
    else
      bad "$so is missing: $(echo "$MISSING" | tr '\n' ' ')"
    fi
    # No undefined non-libc / non-libgcc symbols.
    UNDEF=$(nm -D --undefined-only "$so" | awk '{print $NF}' |
      sed 's/@.*//' | sort -u |
      grep -vE '^(_ITM_|__cxa_|__gmon_|__tls_get_addr|__errno_location|_Unwind_|_?_?libc)' |
      grep -vxE 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_[a-z_]*|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev|printf|puts|__isoc99_scanf|sysconf|open|fstat|lseek|mprotect|madvise|getrandom|poll|sigaltstack|sigaction|pipe2|dup|dup2|fcntl|__libc_start_main|environ')
    if [ -z "$UNDEF" ]; then
      ok "$so has 0 undefined non-libc symbols"
    else
      bad "$so undefined non-libc: $(echo "$UNDEF" | tr '\n' ' ')"
    fi
  done
  rm -rf "$SCRATCH"

  step "Phase C — error-path differential tests (ERRORS.md)  [$LABEL]"
  if timeout 600 cargo test "${FEATFLAGS[@]}" --test error_paths -- --test-threads=1 2>&1 |
    tail -4; then ok "error_paths [$LABEL]"; else bad "error_paths [$LABEL]"; fi

  step "Phase B — valid-path differential tests (CONFIGS.md)  [$LABEL]"
  if timeout 600 cargo test "${FEATFLAGS[@]}" --test valid_paths -- --test-threads=1 2>&1 |
    tail -4; then ok "valid_paths [$LABEL]"; else bad "valid_paths [$LABEL]"; fi
done

# ------------------------------------------- end-to-end executable sweep ---
step "Phase B/C extra — end-to-end executable sweep over stdin corpus"
python3 - "$ROOT" <<'PY'
import itertools, random, subprocess, sys, os
root = sys.argv[1]
c_exe = os.path.join(root, "c_src/build/driver")
r_exe = os.path.join(root, "target/release/driver")
cases = [b"", b"0", b"1", b"-1", b"+1", b"abc", b"0x10", b"  7  ", b"\x0b5", b"\x0c5",
         b"\r5", b"\t5", b"\n5", b" 5", b"-", b"+", b"- 5", b"12abc", b"4294967296",
         b"99999999999999999999", b"-99999999999999999999", b"2147483647",
         b"-2147483648", b"0000", b"\x00", b"\xff", b"\x1c5", b"\xa05",
         b"0" * 400, b"0" * 400 + b"1", b"9" * 100, b" " * 5000 + b"3"]
rnd = random.Random(0x5EED1234ABCD0001)
alpha = b"0123456789+-\t\n\x0b\x0c\r abcxX.,eE\x00\xff\x80\xa0"
cases += [bytes(rnd.choice(alpha) for _ in range(rnd.randrange(0, 25))) for _ in range(600)]
cases += [bytes(rnd.randrange(256) for _ in range(rnd.randrange(0, 64))) for _ in range(300)]

def run(exe, data):
    p = subprocess.run([exe], input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return p.returncode, p.stdout, p.stderr

mismatch = 0
for i, data in enumerate(cases):
    a, b = run(c_exe, data), run(r_exe, data)
    if a != b:
        mismatch += 1
        print(f"  MISMATCH #{i} stdin={data!r}\n    C   ={a!r}\n    Rust={b!r}")
        if mismatch > 10:
            break
print(f"  {len(cases)} stdin cases compared, {mismatch} mismatch(es)")
sys.exit(1 if mismatch else 0)
PY
if [ $? -eq 0 ]; then ok "executable sweep"; else bad "executable sweep"; fi

# ------------------------------------------------------------------ done ---
step "RESULT"
if [ "$FAIL" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAIL"

#!/usr/bin/env bash
# Sanity check for the differential harness itself: inject a known behavioural
# divergence into translation/src, confirm the tests CATCH it, and restore.
#
# A test suite that passes is only meaningful if it can fail. Every mutation
# below is a plausible translation mistake.
#
#   ./mutation_check.sh
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
cd "$HERE"
BK="$HERE/.mutation-backup"

restore() {
  if [ -d "$BK" ]; then
    rm -rf src && cp -r "$BK" src && rm -rf "$BK"
    echo "src/ restored from backup"
  fi
  rm -f "$HERE/.mutation-run.log"
}
trap restore EXIT INT TERM

rm -rf "$BK"; cp -r src "$BK"

# Build + test. Writes the log to $LOG, returns cargo's exit status.
LOG="$HERE/.mutation-run.log"
run_tests() {
  if ! cargo build --offline -q >"$LOG" 2>&1; then
    echo "buildfail"
    return 0
  fi
  cargo test --offline -q -- --test-threads=1 >"$LOG" 2>&1
  local rc=$?
  # `-q` renders failures as "<name> --- FAILED"; the verbose form is
  # "test <name> ... FAILED". Count both.
  local n
  n=$(grep -cE '(^| )--- FAILED|\.\.\. FAILED' "$LOG")
  if [ "$rc" -ne 0 ] && [ "$n" -eq 0 ]; then n=1; fi   # e.g. a harness abort
  echo "$n"
}
first_failures() {
  grep -oE '^[a-z0-9_]+ --- FAILED|^test [a-z0-9_]+ \.\.\. FAILED' "$LOG" \
    | sed -E 's/^test //; s/ (---|\.\.\.) FAILED//' | head -4 | paste -sd' '
}

PASS=0
FAILED_TO_DETECT=()

check() {
  local name="$1" file="$2" from="$3" to="$4"
  rm -rf src && cp -r "$BK" src
  python3 - "$file" "$from" "$to" <<'PY' || { echo "  [$name] SKIP (pattern not found)"; return; }
import sys, pathlib
p = pathlib.Path("src")/sys.argv[1]
t = p.read_text()
if sys.argv[2] not in t:
    sys.exit(1)
p.write_text(t.replace(sys.argv[2], sys.argv[3], 1))
PY
  local n; n=$(run_tests)
  if [ "$n" = "buildfail" ]; then
    printf '  \033[33mN/A\033[0m     %-42s (mutation does not compile)\n' "$name"
    return
  fi
  if [ "${n:-0}" -gt 0 ]; then
    printf '  \033[32mCAUGHT\033[0m  %-42s %s failing test(s): %s\n' "$name" "$n" "$(first_failures)"
    PASS=$((PASS+1))
  else
    printf '  \033[31mMISSED\033[0m  %-42s 0 failing tests\n' "$name"
    FAILED_TO_DETECT+=("$name")
  fi
}

echo "baseline (must be 0 failing):"
rm -rf src && cp -r "$BK" src
echo "  $(run_tests) failing tests"
echo
echo "mutations:"

check "logger tag [INFO] -> [Info]"        logger.rs       '[INFO] %s\n'                       '[Info] %s\n'
check "logger tag [WARNING] -> [WARN]"     logger.rs       '[WARNING] %s\n'                    '[WARN] %s\n'
check "initialize_logger returns -2"       logger.rs       'return -1;'                        'return -2;'
check "default log name changed"           logger.rs       'c"default.log"'                    'c"defaults.log"'
check "fopen mode a -> w"                  logger.rs       'c"a".as_ptr()'                     'c"w".as_ptr()'
check "finalize_logger resets the static"  logger.rs       'cstd::fclose(stream);'             'cstd::fclose(stream); LOG_FILE = ptr::null_mut();'
check "default max_tasks 10 -> 11"         task_manager.rs '            10
        };'                                                '            11
        };'
check "strncpy limit 255 -> 254"           task_manager.rs 'description, 256 - 1'              'description, 256 - 2'
check "forced NUL at 254 not 255"          task_manager.rs '*desc.add(256 - 1) = 0;'           '*desc.add(256 - 2) = 0;'
check "capacity gate >= becomes >"         task_manager.rs 'task_count >= (*manager).max_tasks' 'task_count > (*manager).max_tasks'
check "print index i+1 -> i"               task_manager.rs 'i.wrapping_add(1),'                 'i,'
check "print header text"                  task_manager.rs 'c"Tasks:\n"'                        'c"Tasks:\r\n"'
check "size uses usize not sign-extend"    task_manager.rs '(*manager).max_tasks as isize as usize' '(*manager).max_tasks as u32 as usize'
check "destroy frees in reverse order"     task_manager.rs 'cstd::free((*manager).tasks as *mut c_void);
        cstd::free(manager as *mut c_void);'                'cstd::free(manager as *mut c_void);'
check "defensive null check in add_task"   task_manager.rs 'if (*manager).task_count'           'if manager.is_null() { return; } if (*manager).task_count'
check "EXIT_FAILURE 1 -> 2"                driver.rs       'const EXIT_FAILURE: c_int = 1;'     'const EXIT_FAILURE: c_int = 2;'
check "priority not incremented"           driver.rs       'priority = priority.wrapping_add(1);' '{}'
check "newline skip off by one"            driver.rs       'end.add(1)'                        'end'
check "driver forgets finalize_logger"     driver.rs       '    destroy_task_manager(manager);
    finalize_logger();'                                    '    destroy_task_manager(manager);'
check "driver returns 0 on logger failure" driver.rs       '    if res != 0 {
        return EXIT_FAILURE;'                              '    if res != 0 {
        return 0;'

echo
if [ "${#FAILED_TO_DETECT[@]}" -eq 0 ]; then
  echo "ALL $PASS MUTATIONS DETECTED — the differential harness is not vacuous."
else
  echo "UNDETECTED MUTATIONS (${#FAILED_TO_DETECT[@]}): ${FAILED_TO_DETECT[*]}"
  exit 1
fi

#!/usr/bin/env bash
# Harness self-check by mutation testing.
#
# A differential suite that passes but cannot fail proves nothing.  This script
# injects a known divergence into the Rust translation, checks that the suite
# CATCHES it, and restores the original source afterwards.
#
# Two categories:
#   mutate      - a real behavioural divergence; the suite MUST fail.
#   equivalent  - a source change that is provably unobservable through the
#                 library's public surface on this platform; the suite is
#                 EXPECTED to pass, and the reason is documented inline.
set -uo pipefail
cd "$(dirname "$0")"

BACKUP=$(mktemp -d)
cp -r src "$BACKUP/"
restore() { rm -rf src; cp -r "$BACKUP/src" .; }
# Rebuilding on the way out is MANDATORY: `cargo test` does not rebuild the
# cdylib, so leaving a mutant target/release/libdriver.so behind would poison
# every later test run with results that describe a binary nobody asked about.
cleanup() { restore; timeout 600 cargo build --release >/dev/null 2>&1; rm -rf "$BACKUP"; }
trap cleanup EXIT

TOTAL=0; CAUGHT=0; EQ_TOTAL=0; EQ_OK=0

apply() { # file from to  -> 0 on success
  python3 - "$1" "$2" "$3" <<'PY'
import sys
f, a, b = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(f).read()
if a not in s:
    sys.exit(2)
open(f, 'w').write(s.replace(a, b, 1))
PY
}

run_suite() { timeout 600 cargo test --release -- --test-threads=1 >/tmp/mut.log 2>&1; }

mutate() { # name file from to
  local name="$1"; TOTAL=$((TOTAL + 1)); restore
  apply "$2" "$3" "$4" || { echo "SKIP   $name (pattern not found)"; return; }
  timeout 600 cargo build --release >/dev/null 2>&1 || { echo "SKIP   $name (does not compile)"; return; }
  if run_suite; then
    echo "MISS   $name  <-- the suite did NOT catch this"
  else
    local who
    who=$(grep -m1 -oE 'DIVERGENCE in `[^`]*`' /tmp/mut.log)
    [ -z "$who" ] && who=$(grep -m1 -oE '^---- [a-zA-Z0-9_]+ stdout' /tmp/mut.log | awk '{print $2}')
    echo "CAUGHT $name  [$who]"
    CAUGHT=$((CAUGHT + 1))
  fi
}

equivalent() { # name file from to reason
  local name="$1"; EQ_TOTAL=$((EQ_TOTAL + 1)); restore
  apply "$2" "$3" "$4" || { echo "SKIP   $name (pattern not found)"; return; }
  timeout 600 cargo build --release >/dev/null 2>&1 || { echo "SKIP   $name (does not compile)"; return; }
  if run_suite; then
    echo "EQUIV  $name  ($5)"
    EQ_OK=$((EQ_OK + 1))
  else
    echo "NOTEQ  $name  -- expected to be unobservable but the suite failed"
  fi
}

echo "### mutants that must be caught"

mutate "add_task: strncpy bound 255 -> 254" src/task_manager.rs \
  "strncpy(desc, description, 256 - 1);" "strncpy(desc, description, 256 - 2);"

mutate "add_task: drop the explicit description[255] = 0" src/task_manager.rs \
  "    *desc.add(256 - 1) = 0;
" ""

mutate "add_task: capacity test >= -> >" src/task_manager.rs \
  "if (*manager).task_count >= (*manager).max_tasks {" \
  "if (*manager).task_count > (*manager).max_tasks {"

mutate "add_task: task_count incremented after the write" src/task_manager.rs \
  "    let index = (*manager).task_count;
    (*manager).task_count = index + 1;" \
  "    let index = (*manager).task_count;"

mutate "create_task_manager: default capacity 10 -> 16" src/task_manager.rs \
  "        10
    };" "        16
    };"

mutate "create_task_manager: int->size_t zero-extended instead of sign-extended" src/task_manager.rs \
  "(max_tasks as isize as usize).wrapping_mul(size_of::<Task>())" \
  "(max_tasks as u32 as usize).wrapping_mul(size_of::<Task>())"

mutate "create_task_manager: negative capacity clamped to 0 (malloc(0) succeeds)" src/task_manager.rs \
  "(max_tasks as isize as usize).wrapping_mul(size_of::<Task>())" \
  "(max_tasks.max(0) as usize).wrapping_mul(size_of::<Task>())"

mutate "create_task_manager: free(manager) removed (leaks on the failure path)" src/task_manager.rs \
  "        free(manager as *mut c_void);
" ""

mutate "create_task_manager: task_count initialised to 1" src/task_manager.rs \
  "addr_of_mut!((*manager).task_count).write(0);" \
  "addr_of_mut!((*manager).task_count).write(1);"

mutate "print_tasks: index printed as i instead of i+1" src/task_manager.rs \
  "            i + 1," "            i,"

mutate "print_tasks: header text" src/task_manager.rs \
  'printf(c"Tasks:\n".as_ptr());' 'printf(c"Tasks: \n".as_ptr());'

mutate "destroy_task_manager: tasks array leaked" src/task_manager.rs \
  "    free((*manager).tasks as *mut c_void);
" ""

mutate "logger: [WARNING] tag -> [WARN]" src/logger.rs \
  'c"[WARNING] %s\n"' 'c"[WARN] %s\n"'

mutate "logger: default path" src/logger.rs \
  'c"default.log"' 'c"defaults.log"'

mutate "logger: fopen mode a -> w (truncate instead of append)" src/logger.rs \
  'c"a".as_ptr()' 'c"w".as_ptr()'

mutate "logger: initialize_logger error return -1 -> -2" src/logger.rs \
  "        return -1;" "        return -2;"

mutate "logger: env var name LOG_FILE -> LOGFILE" src/logger.rs \
  'c"LOG_FILE".as_ptr()' 'c"LOGFILE".as_ptr()'

mutate "logger: log_error routed to the [INFO] tag" src/logger.rs \
  'fprintf(LOG_FILE, c"[ERROR] %s\n".as_ptr(), message);' \
  'fprintf(LOG_FILE, c"[INFO] %s\n".as_ptr(), message);'

mutate "logger: message used as the format string" src/logger.rs \
  'fprintf(LOG_FILE, c"[INFO] %s\n".as_ptr(), message);' \
  'fprintf(LOG_FILE, message);'

mutate "driver: EXIT_FAILURE 1 -> 2" src/cbind.rs \
  "pub const EXIT_FAILURE: c_int = 1;" "pub const EXIT_FAILURE: c_int = 2;"

mutate "driver: trailing-newline advance always end+1" src/driver.rs \
  "        start = if *end == b'\\n' as c_char {
            end.add(1)
        } else {
            end
        };" "        start = end.add(1);"

mutate "driver: priority starts at 0" src/driver.rs \
  "    let mut priority: c_int = 1;" "    let mut priority: c_int = 0;"

mutate "driver: priority never incremented" src/driver.rs \
  "        priority += 1;
" ""

mutate "driver: empty lines skipped" src/driver.rs \
  "        add_task(manager, task, priority);" \
  "        if length > 0 { add_task(manager, task, priority); }"

mutate "driver: per-line copy leaked" src/driver.rs \
  "        free(task as *mut c_void);
" ""

mutate "driver: finalize_logger dropped on the success path" src/driver.rs \
  "    destroy_task_manager(manager);
    finalize_logger();

    0" "    destroy_task_manager(manager);

    0"

mutate "driver: finalize_logger added to the create-failure path" src/driver.rs \
  "    let manager = create_task_manager();
    if manager.is_null() {
        return EXIT_FAILURE;
    }" \
  "    let manager = create_task_manager();
    if manager.is_null() {
        finalize_logger();
        return EXIT_FAILURE;
    }"

mutate "add_task: NULL-guard added (the C has none)" src/task_manager.rs \
  "    if (*manager).task_count >= (*manager).max_tasks {" \
  "    if manager.is_null() || description.is_null() { return; }
    if (*manager).task_count >= (*manager).max_tasks {"

mutate "print_tasks: NULL-guard added (the C has none)" src/task_manager.rs \
  "    printf(c\"Tasks:\\n\".as_ptr());" \
  "    if manager.is_null() { return; }
    printf(c\"Tasks:\\n\".as_ptr());"

echo
echo "### changes that are provably unobservable through the public surface"

equivalent "add_task: strncpy bound 255 -> 256" src/task_manager.rs \
  "strncpy(desc, description, 256 - 1);" "strncpy(desc, description, 256);" \
  "byte 255 is overwritten with 0 on the next line either way"

equivalent "logger: finalize_logger resets the handle to NULL" src/logger.rs \
  "        fclose(LOG_FILE);" \
  "        fclose(LOG_FILE);
        LOG_FILE = core::ptr::null_mut();" \
  "glibc writes nothing and does not fault on the fclose'd FILE*, see ERRORS.md row 27"

echo
echo "=== mutation summary ==="
echo "real divergences caught : $CAUGHT / $TOTAL"
echo "equivalent mutants OK   : $EQ_OK / $EQ_TOTAL"
[ "$CAUGHT" -eq "$TOTAL" ] && [ "$EQ_OK" -eq "$EQ_TOTAL" ]

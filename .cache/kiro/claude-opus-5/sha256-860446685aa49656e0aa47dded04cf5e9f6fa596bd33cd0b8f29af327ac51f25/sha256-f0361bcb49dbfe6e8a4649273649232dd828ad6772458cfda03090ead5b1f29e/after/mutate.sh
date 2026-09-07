#!/usr/bin/env bash
# Mutation-coverage check: inject a known bug into the Rust translation, confirm
# the differential suite catches it, then revert. Proves the tests have teeth
# and are not passing vacuously (e.g. against a stale .so or empty captures).
set -uo pipefail
cd "$(dirname "$0")/translation"
mkdir -p /tmp/mutbak && cp src/*.rs /tmp/mutbak/

restore() { cp /tmp/mutbak/*.rs src/; }
trap restore EXIT

# subst <file> <from> <to>  — literal, first occurrence only, exit 1 if absent.
subst() {
  python3 - "$1" "$2" "$3" <<'PY'
import sys
path, frm, to = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
if frm not in s:
    sys.exit(1)
open(path, "w").write(s.replace(frm, to, 1))
PY
}

pass=0; miss=0; skip=0
run_one() {
  local name="$1" file="$2" from="$3" to="$4"
  restore
  if ! subst "src/$file" "$from" "$to"; then
    echo "SKIP   $name (pattern absent)"; skip=$((skip+1)); return
  fi
  if ! timeout 300 cargo build --release >/dev/null 2>&1; then
    echo "SKIP   $name (mutant does not compile)"; skip=$((skip+1)); return
  fi
  local out rc
  out=$(timeout 300 cargo test --release -- --test-threads=1 2>&1); rc=$?
  if echo "$out" | grep -q 'test result: FAILED' || [ $rc -ne 0 ]; then
    local which
    which=$(echo "$out" | grep -oE '^test [a-z0-9_]+ \.\.\. FAILED' | head -3 | awk '{print $2}' | paste -sd, -)
    [ -n "$which" ] || which="(see log)"
    echo "CAUGHT $name  by: $which"
    pass=$((pass+1))
  else
    echo "MISSED $name  <-- BLIND SPOT"
    miss=$((miss+1))
  fi
}

# ---- add_task -------------------------------------------------------------
run_one M01_trunc_off_by_one   task_manager.rs 'strncpy(desc, description, 256 - 1)' 'strncpy(desc, description, 256)'
run_one M02_trunc_too_short    task_manager.rs 'strncpy(desc, description, 256 - 1)' 'strncpy(desc, description, 256 - 2)'
run_one M03_no_forced_nul      task_manager.rs '*desc.add(256 - 1) = 0;' ''
run_one M04_capacity_gt        task_manager.rs '(*manager).task_count >= (*manager).max_tasks' '(*manager).task_count > (*manager).max_tasks'
run_one M05_no_count_increment task_manager.rs '(*manager).task_count = index + 1;' ''
run_one M06_wrong_warn_text    task_manager.rs 'Cannot add task: Maximum task limit reached.' 'Cannot add task: maximum task limit reached.'
# ---- create_task_manager --------------------------------------------------
run_one M07_default_max_tasks  task_manager.rs 'atoi(max_tasks_env)
    } else {
        10' 'atoi(max_tasks_env)
    } else {
        11'
run_one M08_maxtasks_env_name  task_manager.rs 'c"MAX_TASKS"' 'c"MAXTASKS"'
run_one M09_count_not_zeroed   task_manager.rs 'addr_of_mut!((*manager).task_count).write(0);' 'addr_of_mut!((*manager).task_count).write(1);'
run_one M10_alloc_size         task_manager.rs 'wrapping_mul(size_of::<Task>())' 'wrapping_mul(size_of::<Task>() + 1)'
# ---- print_tasks ----------------------------------------------------------
run_one M11_print_index_base   task_manager.rs 'i + 1,' 'i,'
run_one M12_print_header       task_manager.rs 'c"Tasks:\n"' 'c"Tasks:\r\n"'
run_one M13_print_format       task_manager.rs '  [%d] %s (Priority: %d)\n' '  [%d] %s (priority: %d)\n'
# ---- logger ---------------------------------------------------------------
run_one M14_logfile_env_name   logger.rs 'c"LOG_FILE"' 'c"LOGFILE"'
run_one M15_default_log_name   logger.rs 'c"default.log"' 'c"defaults.log"'
run_one M16_open_mode_w        logger.rs 'c"a".as_ptr()' 'c"w".as_ptr()'
run_one M17_init_err_code      logger.rs 'return -1;' 'return -2;'
run_one M18_no_init_log_line   logger.rs 'log_info(c"Logger initialized.".as_ptr());' ''
run_one M19_finalize_order     logger.rs 'log_info(c"Logger finalized.".as_ptr());
        fclose(LOG_FILE);' 'fclose(LOG_FILE);'
run_one M20_info_label         logger.rs 'c"[INFO] %s\n"' 'c"[INFO]%s\n"'
run_one M21_warning_label      logger.rs 'c"[WARNING] %s\n"' 'c"[WARN] %s\n"'
run_one M22_error_label        logger.rs 'c"[ERROR] %s\n"' 'c"[ERR] %s\n"'
run_one M23_stderr_msg         logger.rs 'Failed to open log file: %s\n' 'Failed to open logfile: %s\n'
# ---- driver ---------------------------------------------------------------
run_one M24_priority_pre_inc   driver.rs 'add_task(manager, task, priority);
        priority += 1;' 'priority += 1;
        add_task(manager, task, priority);'
run_one M25_priority_start     driver.rs 'let mut priority: c_int = 1;' 'let mut priority: c_int = 0;'
run_one M26_priority_no_inc    driver.rs 'priority += 1;' ''
run_one M27_success_ret        driver.rs '    finalize_logger();

    0
}' '    finalize_logger();

    7
}'
run_one M28_exit_failure_val   driver.rs 'return EXIT_FAILURE;
    }

    let manager' 'return 2;
    }

    let manager'
run_one M29_no_finalize        driver.rs 'destroy_task_manager(manager);
    finalize_logger();' 'destroy_task_manager(manager);'
run_one M30_skip_empty_lines   driver.rs 'add_task(manager, task, priority);' 'if length > 0 { add_task(manager, task, priority); }'
run_one M31_no_print           driver.rs 'print_tasks(manager);' ''
run_one M32_newline_reanchor   driver.rs 'end.add(1)' 'end.add(0)'

run_one M33_addtask_offset     task_manager.rs '(*manager).tasks.offset(index as isize)' '(*manager).tasks.offset(index as isize + 1)'
run_one M34_print_offset       task_manager.rs '(*manager).tasks.offset(i as isize)' '(*manager).tasks.offset(i as isize + 1)'
run_one M35_driver_no_nul      driver.rs '*task.add(length) = 0;' ''
run_one M36_strchr_char        driver.rs "strchr(start, b'\\n' as c_int)" "strchr(start, b'\\r' as c_int)"
run_one M37_length_off_by_one  driver.rs 'let length = end.offset_from(start) as usize;' 'let length = (end.offset_from(start) as usize).saturating_sub(1);'
run_one M39_print_before_add   task_manager.rs 'log_info(c"Task added successfully.".as_ptr());' 'log_info(c"Task added successfully".as_ptr());'
run_one M40_create_log_text    task_manager.rs 'TaskManager created successfully.' 'TaskManager created successfully!'
run_one M41_destroy_log_text   task_manager.rs 'TaskManager destroyed successfully.' 'TaskManager destroyed successfully!'
run_one M42_alloc_fail_text    task_manager.rs 'Failed to allocate memory for tasks.' 'Failed to allocate memory for task.'

restore
cargo build --release >/dev/null 2>&1
echo
echo "=== mutation coverage: $pass caught, $miss MISSED, $skip skipped ==="
[ "$miss" -eq 0 ]

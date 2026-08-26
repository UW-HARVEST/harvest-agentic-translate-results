//! Phase B — valid-path differential tests for the **lowest-level** entry
//! points: `initialize_logger`, `log_info`, `log_warning`, `log_error`,
//! `finalize_logger`.
//!
//! Covers `CONFIGS.md` rows 1-6.

mod common;

use common::*;

/// CONFIGS row 1 — L1 fresh log path, plain open → close cycle.
fn cfg_01_logger_init_finalize() {
    let obs = diff("cfg_01", &Cfg::fresh(), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        (api.finalize_logger)();
    });
    assert_eq!(obs.rets, vec![0]);
    assert_eq!(
        obs.log,
        b"[INFO] Logger initialized.\n[INFO] Logger finalized.\n".to_vec(),
        "C reference log content changed unexpectedly: {:?}",
        String::from_utf8_lossy(&obs.log)
    );
    assert!(obs.stdout.is_empty());
    assert!(obs.stderr.is_empty());
}

/// CONFIGS row 2 — every severity, with randomized message payloads.
///
/// One `diff` run logs 100 generated messages (empty, 1-byte, long, `%`-bearing,
/// control bytes, high bytes) round-robin across the three severities. The PRNG
/// is seeded identically inside both runs so the two implementations see exactly
/// the same input sequence.
fn cfg_02_logger_all_severities_random() {
    const SEED: u64 = 0xB0B1_C0DE_1234_0002;
    let obs = diff("cfg_02", &Cfg::fresh(), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        let mut rng = Rng::new(SEED);
        for i in 0..100usize {
            let len = match i {
                0 => 0,
                1 => 1,
                2 => 255,
                3 => 256,
                4 => 1000,
                _ => rng.range(0, 300),
            };
            let msg = cstr(&rng.text(len));
            let p = msg.as_ptr() as *const _;
            match i % 3 {
                0 => (api.log_info)(p),
                1 => (api.log_warning)(p),
                _ => (api.log_error)(p),
            }
        }
        (api.finalize_logger)();
    });
    // Sanity: the reference run really produced all 100 records + 2 lifecycle
    // lines (messages never contain '\n', so lines == records).
    let lines = obs.log.iter().filter(|&&b| b == b'\n').count();
    assert_eq!(lines, 102, "unexpected C record count");
}

/// CONFIGS row 3 — `fopen(path, "a")` must APPEND to an existing file.
fn cfg_03_logger_append_existing() {
    let seed = b"PRE-EXISTING LINE 1\nPRE-EXISTING LINE 2\n".to_vec();
    let cfg = Cfg::fresh().log(LogSetting::PreExisting(seed.clone()));
    let obs = diff("cfg_03", &cfg, |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        let m = cstr(b"appended");
        (api.log_warning)(m.as_ptr() as *const _);
        (api.finalize_logger)();
    });
    assert!(
        obs.log.starts_with(&seed),
        "append mode truncated the file: {:?}",
        String::from_utf8_lossy(&obs.log)
    );
    assert_eq!(
        obs.log,
        [
            seed.as_slice(),
            b"[INFO] Logger initialized.\n",
            b"[WARNING] appended\n",
            b"[INFO] Logger finalized.\n",
        ]
        .concat()
    );
}

/// CONFIGS row 4 — `LOG_FILE` unset → `./default.log` relative to the CWD.
fn cfg_04_logger_default_path() {
    let cfg = Cfg::fresh().log(LogSetting::UnsetUseCwdDefault);
    let obs = diff("cfg_04", &cfg, |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        let m = cstr(b"default path");
        (api.log_info)(m.as_ptr() as *const _);
        (api.finalize_logger)();
    });
    assert_eq!(obs.rets, vec![0]);
    assert_eq!(
        obs.log,
        b"[INFO] Logger initialized.\n[INFO] default path\n[INFO] Logger finalized.\n".to_vec(),
        "default.log was not written where C writes it"
    );
}

/// CONFIGS row 5 — re-initialise without finalising. C assigns the new handle
/// over the old one, leaking the first stream; both implementations must show
/// the same resulting file contents.
fn cfg_05_logger_double_init() {
    let obs = diff("cfg_05", &Cfg::fresh(), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        rec.ret((api.initialize_logger)());
        let m = cstr(b"after second init");
        (api.log_error)(m.as_ptr() as *const _);
        (api.finalize_logger)();
    });
    assert_eq!(obs.rets, vec![0, 0]);
    // Two "Logger initialized." lines must be present (one per open handle).
    let text = String::from_utf8_lossy(&obs.log);
    assert_eq!(
        text.matches("[INFO] Logger initialized.").count(),
        2,
        "unexpected C log: {text:?}"
    );
    assert!(text.contains("[ERROR] after second init"));
}

/// CONFIGS row 6 — two complete open/close cycles on the same path.
fn cfg_06_logger_two_cycles() {
    let obs = diff("cfg_06", &Cfg::fresh(), |api, rec| unsafe {
        rec.ret((api.initialize_logger)());
        (api.finalize_logger)();
        rec.ret((api.initialize_logger)());
        (api.finalize_logger)();
    });
    assert_eq!(obs.rets, vec![0, 0]);
    assert_eq!(
        obs.log,
        b"[INFO] Logger initialized.\n[INFO] Logger finalized.\n\
          [INFO] Logger initialized.\n[INFO] Logger finalized.\n"
            .to_vec()
    );
}

// ---------------------------------------------------------------------------
// Single serialized entry point.
//
// The libtest harness writes its own "test NAME ... ok" progress lines to fd 1
// from the main thread while other test threads are still running. Because this
// harness temporarily redirects fd 1/fd 2 to capture what the *libraries* print,
// concurrently-running tests would pollute the capture. Exposing exactly one
// #[test] removes that race entirely; each scenario still reports itself through
// the label carried in the assertion message.
// ---------------------------------------------------------------------------
#[test]
fn phase_b_logger_all() {
    cfg_01_logger_init_finalize();
    cfg_02_logger_all_severities_random();
    cfg_03_logger_append_existing();
    cfg_04_logger_default_path();
    cfg_05_logger_double_init();
    cfg_06_logger_two_cycles();
}

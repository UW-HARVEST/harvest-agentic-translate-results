mod common;

use common::*;

/// Harness self-test: both `.so`s load, all ten symbols resolve, and the
/// capture / sandbox machinery works.
#[test]
fn harness_loads_both_libraries() {
    let _g = lock();
    println!("C    = {}", c_api().path.display());
    println!("Rust = {}", rust_api().path.display());
}

#[test]
fn smoke_driver_two_lines() {
    let _g = lock();
    diff(
        "smoke_driver_two_lines",
        LogCfg::InDir("app.log".into()),
        Some("10"),
        |api, _| {
            let input = cs(b"alpha\nbeta");
            let rc = unsafe { (api.driver)(input.as_ptr()) };
            format!("driver={rc}")
        },
    );
}

#[test]
fn smoke_low_level_pipeline() {
    let _g = lock();
    diff(
        "smoke_low_level_pipeline",
        LogCfg::InDir("app.log".into()),
        Some("3"),
        |api, _| {
            let mut obs = String::new();
            unsafe {
                obs += &format!("init={};", (api.initialize_logger)());
                let m = (api.create_task_manager)();
                obs += &format!("created_null={};", m.is_null());
                for (d, p) in [(&b"one"[..], 1i32), (b"two", -5), (b"three", i32::MAX)] {
                    let c = cs(d);
                    (api.add_task)(m, c.as_ptr(), p);
                }
                obs += &format!("{}\n", dump_manager(m));
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
            }
            obs
        },
    );
}

/// Proves the harness channels are live (not vacuously equal): the C side must
/// produce exactly the bytes the C source dictates, on stdout AND in the log.
#[test]
fn harness_channels_are_live() {
    let _g = lock();
    for api in [c_api(), rust_api()] {
        let o = run_side(api, &LogCfg::InDir("app.log".into()), Some("10"), |api, _| {
            let input = cs(b"alpha\nbeta");
            format!("driver={}", unsafe { (api.driver)(input.as_ptr()) })
        });
        assert_eq!(o.obs, "driver=0", "{}", api.name);
        assert_eq!(
            String::from_utf8_lossy(&o.stdout),
            "Tasks:\n  [1] alpha (Priority: 1)\n  [2] beta (Priority: 2)\n",
            "{} stdout", api.name
        );
        assert_eq!(
            String::from_utf8_lossy(&o.log),
            "[INFO] Logger initialized.\n\
             [INFO] TaskManager created successfully.\n\
             [INFO] Task added successfully.\n\
             [INFO] Task added successfully.\n\
             [INFO] TaskManager destroyed successfully.\n\
             [INFO] Logger finalized.\n",
            "{} log", api.name
        );
        assert!(o.stderr.is_empty(), "{} stderr", api.name);
    }
}

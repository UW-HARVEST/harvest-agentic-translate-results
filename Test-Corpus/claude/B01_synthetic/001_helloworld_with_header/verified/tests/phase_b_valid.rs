//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects through their exported symbols
//! (`dlopen` + `dlsym`, never a direct Rust call) in the configuration of its
//! row and asserts the results are byte-identical. Randomized rows use a fixed
//! seed so failures reproduce.
//!
//! Run with `-- --test-threads=1` (see `verify.sh`): the scenarios fork, and
//! serializing them keeps the forks away from other threads' allocator state.

mod common;

use common::*;

/// Row 1 — `helloworld`, fd 1 → regular file, default buffering, one call.
///
/// The API takes no arguments, so this row's input space is a single point; it
/// is repeated to catch any per-run state dependence.
#[test]
fn cfg01_single_call_to_file() {
    for _ in 0..16 {
        assert_same_and_expect(
            "cfg01",
            &[Step::hello()],
            &RunOpts::default(),
            HELLO,
            1,
        );
    }
}

/// Row 2 — zero calls: the "empty" boundary. Both libraries must emit nothing.
#[test]
fn cfg02_zero_calls_emit_nothing() {
    assert_same_and_expect("cfg02", &[], &RunOpts::default(), b"", 0);
    // Also with markers only, to prove the harness itself is faithful.
    let steps = [Step::Marker(b"only-the-caller-wrote-this".to_vec())];
    assert_same_and_expect("cfg02b", &steps, &RunOpts::default(), b"only-the-caller-wrote-this", 0);
}

/// Row 3 — `helloworld`, fd 1 → regular file, many calls (randomized count).
#[test]
fn cfg03_many_calls_to_file() {
    let mut rng = Rng::new(0x0303_0303);
    for _ in 0..24 {
        let n = rng.range(2, 64) as usize;
        let steps: Vec<Step> = (0..n).map(|_| Step::hello()).collect();
        assert_same_and_expect("cfg03", &steps, &RunOpts::default(), &hello_repeated(n), n);
    }
}

/// Row 4 — fd 1 → pipe (glibc chooses full buffering), randomized count.
#[test]
fn cfg04_many_calls_to_pipe() {
    let mut rng = Rng::new(0x0404_0404);
    for _ in 0..24 {
        let n = rng.range(1, 64) as usize;
        let steps: Vec<Step> = (0..n).map(|_| Step::hello()).collect();
        assert_same_and_expect(
            "cfg04",
            &steps,
            &RunOpts::dest(Dest::Pipe),
            &hello_repeated(n),
            n,
        );
    }
}

/// Row 5 — fd 1 → `/dev/null`: writes succeed but produce nothing observable.
#[test]
fn cfg05_calls_to_dev_null() {
    let mut rng = Rng::new(0x0505_0505);
    for _ in 0..16 {
        let n = rng.range(1, 32) as usize;
        let steps: Vec<Step> = (0..n).map(|_| Step::hello()).collect();
        let r = assert_same("cfg05", &steps, &RunOpts::dest(Dest::DevNull));
        assert_eq!(r.rets, vec![0; n], "cfg05: every call must return 0");
        assert_eq!(r.exit, 0);
    }
}

/// Row 6 — `setvbuf(_IONBF)`: unbuffered stream, so every `printf` reaches fd 1
/// immediately.
#[test]
fn cfg06_unbuffered_stream() {
    let mut rng = Rng::new(0x0606_0606);
    for _ in 0..16 {
        let n = rng.range(1, 32) as usize;
        let steps: Vec<Step> = (0..n).map(|_| Step::hello()).collect();
        assert_same_and_expect(
            "cfg06",
            &steps,
            &RunOpts::vbuf(IONBF, 0),
            &hello_repeated(n),
            n,
        );
    }
}

/// Row 7 — `setvbuf(_IOLBF)`: line buffered, with and without a caller buffer.
#[test]
fn cfg07_line_buffered_stream() {
    let mut rng = Rng::new(0x0707_0707);
    for size in [0usize, 128, 4096] {
        for _ in 0..8 {
            let n = rng.range(1, 32) as usize;
            let steps: Vec<Step> = (0..n).map(|_| Step::hello()).collect();
            assert_same_and_expect(
                "cfg07",
                &steps,
                &RunOpts::vbuf(IOLBF, size),
                &hello_repeated(n),
                n,
            );
        }
    }
}

/// Row 8 — `setvbuf(_IOFBF, buf, size)`: fully buffered with a caller-supplied
/// buffer, flushed once at the end. Tiny buffers force mid-line flushes, so the
/// byte stream is only identical if both libraries write through the same
/// stream in the same way.
#[test]
fn cfg08_fully_buffered_stream() {
    let mut rng = Rng::new(0x0808_0808);
    for size in [8usize, 64, 1024, 8192] {
        for _ in 0..8 {
            let n = rng.range(1, 40) as usize;
            let steps: Vec<Step> = (0..n).map(|_| Step::hello()).collect();
            assert_same_and_expect(
                "cfg08",
                &steps,
                &RunOpts::vbuf(IOFBF, size),
                &hello_repeated(n),
                n,
            );
        }
    }
}

/// Row 9 — raw `write(2)` markers interleaved between calls.
///
/// The markers bypass stdio, so their position in the resulting file reveals
/// *when* the library's output is flushed. With the default full buffering of a
/// regular file (4096-byte buffer, and less than that produced here) all
/// markers must appear first and the buffered lines only at the final flush —
/// an implementation that wrote eagerly would produce a different order.
#[test]
fn cfg09_interleaved_raw_write_markers() {
    let mut rng = Rng::new(0x0909_0909);
    for _ in 0..24 {
        let n = rng.range(1, 48) as usize;
        let mut steps = Vec::new();
        let mut markers = Vec::new();
        for _ in 0..n {
            if rng.bool() {
                let m = rng.blob(1, 24);
                markers.extend_from_slice(&m);
                steps.push(Step::Marker(m));
            }
            steps.push(Step::hello());
        }
        let calls = steps.iter().filter(|s| matches!(s, Step::Call { .. })).count();
        let mut expected = markers;
        expected.extend_from_slice(&hello_repeated(calls));
        assert_same_and_expect("cfg09", &steps, &RunOpts::default(), &expected, calls);
    }
}

/// Row 10 — the caller `printf`s through the same `FILE *stdout` between calls,
/// so everything shares one buffer and the exact interleaving is observable.
#[test]
fn cfg10_interleaved_caller_printf() {
    let mut rng = Rng::new(0x0a0a_0a0a);
    for _ in 0..24 {
        let n = rng.range(1, 32) as usize;
        let mut steps = Vec::new();
        let mut expected = Vec::new();
        for _ in 0..n {
            if rng.bool() {
                let m = rng.blob(1, 20);
                expected.extend_from_slice(&m);
                steps.push(Step::caller_print(&m));
            }
            expected.extend_from_slice(HELLO);
            steps.push(Step::hello());
        }
        assert_same_and_expect("cfg10", &steps, &RunOpts::default(), &expected, n);
    }
}

/// Row 11 — explicit `fflush(stdout)` after every call.
#[test]
fn cfg11_flush_after_every_call() {
    let mut rng = Rng::new(0x0b0b_0b0b);
    for _ in 0..16 {
        let n = rng.range(1, 32) as usize;
        let mut steps = Vec::new();
        for _ in 0..n {
            steps.push(Step::hello());
            steps.push(Step::Flush);
        }
        assert_same_and_expect("cfg11", &steps, &RunOpts::default(), &hello_repeated(n), n);
    }
}

/// Row 12 — nothing is flushed until a single `fflush(NULL)` at the end.
#[test]
fn cfg12_no_intermediate_flush() {
    let mut rng = Rng::new(0x0c0c_0c0c);
    for _ in 0..16 {
        let n = rng.range(1, 48) as usize;
        let mut steps: Vec<Step> = (0..n).map(|_| Step::hello()).collect();
        steps.push(Step::FlushAll);
        assert_same_and_expect("cfg12", &steps, &RunOpts::default(), &hello_repeated(n), n);
    }
}

/// Row 13 — `main`, one call: the composed `main -> helloworld` path, whose
/// return value is the program's exit status.
#[test]
fn cfg13_main_single_call() {
    for _ in 0..16 {
        assert_same_and_expect("cfg13", &[Step::main_()], &RunOpts::default(), HELLO, 1);
    }
}

/// Row 14 — `main`, many randomized calls.
#[test]
fn cfg14_main_many_calls() {
    let mut rng = Rng::new(0x0e0e_0e0e);
    for _ in 0..24 {
        let n = rng.range(2, 64) as usize;
        let steps: Vec<Step> = (0..n).map(|_| Step::main_()).collect();
        assert_same_and_expect("cfg14", &steps, &RunOpts::default(), &hello_repeated(n), n);
    }
}

/// Row 15 — randomized mix of both entry points in one stream.
#[test]
fn cfg15_mixed_entry_points() {
    let mut rng = Rng::new(0x0f0f_0f0f);
    for _ in 0..24 {
        let n = rng.range(2, 64) as usize;
        let steps: Vec<Step> = (0..n)
            .map(|_| if rng.bool() { Step::hello() } else { Step::main_() })
            .collect();
        assert_same_and_expect("cfg15", &steps, &RunOpts::default(), &hello_repeated(n), n);
    }
}

/// Row 16 — the C `.so` and the Rust `.so` called alternately in one process,
/// sharing one `FILE *stdout`, with raw markers in between.
///
/// The mixed stream must equal the all-C stream and the all-Rust stream, which
/// is the strongest ordering/buffering check available: a library that flushed
/// at different times would shuffle the markers.
#[test]
fn cfg16_cross_library_interleaving() {
    let mut rng = Rng::new(0x1010_1010);
    let c = c_fns();
    let r = rust_fns();
    for _ in 0..24 {
        let n = rng.range(2, 48) as usize;
        let mut mixed = Vec::new();
        let mut pure = Vec::new();
        let mut markers = Vec::new();
        let mut calls = 0usize;
        for _ in 0..n {
            if rng.bool() {
                let m = rng.blob(1, 16);
                markers.extend_from_slice(&m);
                mixed.push(Step::Marker(m.clone()));
                pure.push(Step::Marker(m));
            }
            let entry = if rng.bool() { Entry::Hello } else { Entry::Main };
            let lib = if rng.bool() { 0 } else { 1 };
            mixed.push(Step::call(entry, lib));
            pure.push(Step::call(entry, 0));
            calls += 1;
        }
        let mut expected = markers;
        expected.extend_from_slice(&hello_repeated(calls));

        let opts = RunOpts::default();
        let all_c = run(&pure, [c, c], &opts);
        let all_rust = run(&pure, [r, r], &opts);
        let mix_cr = run(&mixed, [c, r], &opts);
        let mix_rc = run(&mixed, [r, c], &opts);

        assert_eq!(all_c.bytes, expected, "cfg16: C-only stream unexpected");
        assert_eq!(all_rust.bytes, all_c.bytes, "cfg16: Rust-only stream differs from C-only");
        assert_eq!(mix_cr.bytes, all_c.bytes, "cfg16: C/Rust mixed stream differs");
        assert_eq!(mix_rc.bytes, all_c.bytes, "cfg16: Rust/C mixed stream differs");
        for res in [&all_c, &all_rust, &mix_cr, &mix_rc] {
            assert_eq!(res.rets, vec![0; calls], "cfg16: a call returned non-zero");
            assert_eq!(res.exit, 0);
        }
    }
}

/// Row 17 — concurrent calls from several threads. C stdio locks the stream, so
/// the result must be exactly `threads * per_thread` intact lines with no
/// interleaved fragments.
#[test]
fn cfg17_concurrent_threads() {
    let mut rng = Rng::new(0x1111_1111);
    for _ in 0..6 {
        let threads = rng.range(2, 8) as usize;
        let per_thread = rng.range(1, 16) as usize;
        let expected = hello_repeated(threads * per_thread);

        let opts = RunOpts::default();
        let c = run_threaded(c_fns(), threads, per_thread, &opts);
        let r = run_threaded(rust_fns(), threads, per_thread, &opts);
        assert_eq!(
            c.bytes.len(),
            expected.len(),
            "cfg17: C produced {} bytes, expected {}",
            c.bytes.len(),
            expected.len()
        );
        assert_eq!(c.bytes, expected, "cfg17: C produced interleaved/partial lines");
        assert_eq!(r.bytes, c.bytes, "cfg17: Rust and C differ under concurrency");
        assert_eq!(r.exit, c.exit);
    }
}

/// Row 18 — `dlclose` + `dlopen` between batches: fresh relocations must behave
/// identically.
#[test]
fn cfg18_reload_between_batches() {
    let mut rng = Rng::new(0x1212_1212);
    let cpath = c_lib_path();
    let rpath = rust_lib_path();
    for _ in 0..6 {
        let n = rng.range(1, 24) as usize;
        let steps: Vec<Step> = (0..n).map(|_| Step::hello()).collect();
        let expected = hello_repeated(n);
        for _round in 0..2 {
            let (clib, cf) = open_lib(&cpath);
            let (rlib, rf) = open_lib(&rpath);
            let c = run(&steps, [cf, cf], &RunOpts::default());
            let r = run(&steps, [rf, rf], &RunOpts::default());
            assert_eq!(c.bytes, expected, "cfg18: C after reload");
            assert_eq!(r.bytes, c.bytes, "cfg18: Rust after reload differs from C");
            assert_eq!(r.rets, c.rets);
            assert_eq!(r.exit, c.exit);
            drop(clib); // dlclose
            drop(rlib);
        }
    }
}

/// Row 19 — whole program, stdout → pipe.
#[test]
fn cfg19_program_pipe() {
    for _ in 0..8 {
        let c = std::process::Command::new(c_exe_path()).output().expect("run C driver");
        let r = std::process::Command::new(rust_exe_path()).output().expect("run Rust driver");
        assert_eq!(c.stdout, HELLO, "cfg19: C program stdout unexpected");
        assert_eq!(r.stdout, c.stdout, "cfg19: program stdout differs");
        assert_eq!(r.stderr, c.stderr, "cfg19: program stderr differs");
        assert_eq!(r.status.code(), c.status.code(), "cfg19: exit status differs");
        assert_eq!(c.status.code(), Some(0));
    }
}

/// Row 20 — whole program, stdout → regular file.
#[test]
fn cfg20_program_file() {
    for i in 0..4 {
        let mut out = Vec::new();
        let mut codes = Vec::new();
        for exe in [c_exe_path(), rust_exe_path()] {
            let path = tmp_path(&format!("prog{i}"));
            let file = std::fs::File::create(&path).expect("create out file");
            let st = std::process::Command::new(&exe)
                .stdout(std::process::Stdio::from(file))
                .status()
                .expect("run driver");
            out.push(std::fs::read(&path).expect("read out file"));
            let _ = std::fs::remove_file(&path);
            codes.push(st.code());
        }
        assert_eq!(out[0], HELLO, "cfg20: C program file output unexpected");
        assert_eq!(out[1], out[0], "cfg20: program file output differs");
        assert_eq!(codes[1], codes[0], "cfg20: exit status differs");
        assert_eq!(codes[0], Some(0));
    }
}

/// Row 21 — whole program with extra argv and an emptied environment: `main()`
/// declares no parameters, so neither may be looked at.
#[test]
fn cfg21_program_argv_env() {
    let mut rng = Rng::new(0x1515_1515);
    for _ in 0..6 {
        let argc = rng.range(0, 5) as usize;
        let args: Vec<String> = (0..argc)
            .map(|_| String::from_utf8(rng.blob(1, 12)).unwrap())
            .collect();
        let mut outs = Vec::new();
        for exe in [c_exe_path(), rust_exe_path()] {
            let out = std::process::Command::new(&exe)
                .args(&args)
                .env_clear()
                .output()
                .expect("run driver");
            outs.push((out.stdout, out.stderr, out.status.code()));
        }
        assert_eq!(outs[0].0, HELLO, "cfg21: C program stdout unexpected");
        assert_eq!(outs[1], outs[0], "cfg21: differs with args {:?}", args);
    }
}

/// Row 22 — whole program with stdout closed before `exec`: the write fails, but
/// the exit status must still be 0 for both.
#[test]
fn cfg22_program_closed_stdout() {
    use std::os::unix::process::CommandExt;
    for _ in 0..4 {
        let mut codes = Vec::new();
        let mut errs = Vec::new();
        for exe in [c_exe_path(), rust_exe_path()] {
            let mut cmd = std::process::Command::new(&exe);
            unsafe {
                cmd.pre_exec(|| {
                    // Close fd 1 in the child, before exec.
                    if libc_close(1) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            let out = cmd.stderr(std::process::Stdio::piped()).output().expect("run driver");
            codes.push(out.status.code());
            errs.push(out.stderr);
        }
        assert_eq!(codes[0], Some(0), "cfg22: C program exit status changed");
        assert_eq!(codes[1], codes[0], "cfg22: exit status differs with stdout closed");
        assert_eq!(errs[1], errs[0], "cfg22: stderr differs with stdout closed");
    }
}

/// Row 23 — buffered output is never flushed (the process is `_exit`ed instead
/// of `exit`ed): both libraries must lose exactly the same bytes.
#[test]
fn cfg23_unflushed_output_is_lost_identically() {
    let mut rng = Rng::new(0x1717_1717);
    for dest in [Dest::File, Dest::Pipe] {
        for _ in 0..8 {
            let n = rng.range(1, 32) as usize;
            let steps: Vec<Step> = (0..n).map(|_| Step::hello()).collect();
            let opts = RunOpts::dest(dest).no_final_flush();
            // Fully buffered on a file/pipe and below the buffer size, so
            // nothing reaches fd 1 at all.
            assert_same_and_expect("cfg23", &steps, &opts, b"", n);
        }
    }
}

/// Row 24 — unflushed, but unbuffered (`_IONBF`): now everything has already
/// reached fd 1, so nothing is lost. Complements row 23.
#[test]
fn cfg24_unflushed_but_unbuffered_loses_nothing() {
    let mut rng = Rng::new(0x1818_1818);
    for _ in 0..8 {
        let n = rng.range(1, 32) as usize;
        let steps: Vec<Step> = (0..n).map(|_| Step::hello()).collect();
        let opts = RunOpts { setvbuf_first: Some((IONBF, 0)), final_flush: false, ..Default::default() };
        assert_same_and_expect("cfg24", &steps, &opts, &hello_repeated(n), n);
    }
}

/// Row 25 — whole program with stdout on `/dev/full`: every write fails with
/// `ENOSPC` at the exit-time flush, which `exit()` ignores, so the status must
/// still be 0 and nothing must be produced — for both programs.
#[test]
fn cfg25_program_stdout_enospc() {
    for _ in 0..4 {
        let mut results = Vec::new();
        for exe in [c_exe_path(), rust_exe_path()] {
            let devfull = std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .expect("open /dev/full");
            let out = std::process::Command::new(&exe)
                .stdout(std::process::Stdio::from(devfull))
                .stderr(std::process::Stdio::piped())
                .output()
                .expect("run driver");
            results.push((out.status.code(), out.stderr));
        }
        assert_eq!(results[0].0, Some(0), "cfg25: C program status changed on ENOSPC");
        assert_eq!(results[1], results[0], "cfg25: program differs on ENOSPC stdout");
    }
}

extern "C" {
    #[link_name = "close"]
    fn libc_close(fd: std::os::raw::c_int) -> std::os::raw::c_int;
}

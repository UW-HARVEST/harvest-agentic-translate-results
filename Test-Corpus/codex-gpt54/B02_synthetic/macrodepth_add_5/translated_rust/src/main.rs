use std::env;
use std::ffi::CString;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process;

const OP_NAME: &str = "add";
const REPEAT: i32 = 5;

type OpFn = fn(i32, i32) -> i32;

fn op_add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

fn op_sub(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b)
}

fn op_mul(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b)
}

fn init_for_add() -> i32 {
    0
}

fn init_for_sub() -> i32 {
    0
}

fn init_for_mul() -> i32 {
    1
}

fn step_add(acc: &mut i32, i: i32) {
    *acc = acc.wrapping_add(i);
}

fn step_sub(acc: &mut i32, i: i32) {
    *acc = acc.wrapping_sub(i);
}

fn step_mul(acc: &mut i32, i: i32) {
    *acc = acc.wrapping_mul(i.wrapping_add(1));
}

fn op_fn() -> OpFn {
    op_add
}

fn init_for() -> i32 {
    init_for_add()
}

fn run_loop(acc: &mut i32, n: i32) {
    match n {
        0 => {}
        1 => {
            step_add(acc, 0);
        }
        2 => {
            step_add(acc, 0);
            step_add(acc, 1);
        }
        3 => {
            step_add(acc, 0);
            step_add(acc, 1);
            step_add(acc, 2);
        }
        4 => {
            step_add(acc, 0);
            step_add(acc, 1);
            step_add(acc, 2);
            step_add(acc, 3);
        }
        5 => {
            step_add(acc, 0);
            step_add(acc, 1);
            step_add(acc, 2);
            step_add(acc, 3);
            step_add(acc, 4);
        }
        6 => {
            step_add(acc, 0);
            step_add(acc, 1);
            step_add(acc, 2);
            step_add(acc, 3);
            step_add(acc, 4);
            step_add(acc, 5);
        }
        _ => {}
    }
}

fn accum_add(n: i32) -> i32 {
    let mut acc = init_for_add();
    run_loop(&mut acc, n);
    acc
}

fn helper_call(a: i32, b: i32, out: &mut dyn Write) -> io::Result<i32> {
    let r = op_fn()(a, b);
    let mut acc = init_for();
    run_loop(&mut acc, REPEAT);
    writeln!(out, "helper.call={} helper.acc={}", r, acc)?;
    Ok(r.wrapping_add(acc))
}

fn helper_ptr(a: i32, b: i32, out: &mut dyn Write) -> io::Result<i32> {
    let fp = op_fn();
    let r = fp(a, b);
    writeln!(out, "helper.ptr={}", r)?;
    Ok(r)
}

fn use_generated(n: i32, out: &mut dyn Write) -> io::Result<i32> {
    let r = accum_add(n);
    writeln!(out, "gen.acc={}", r)?;
    Ok(r)
}

fn atoi_arg(bytes: &[u8]) -> i32 {
    let c_string = CString::new(bytes).expect("argv cannot contain interior NUL");
    // SAFETY: `c_string` is a valid NUL-terminated C string for the duration of the call.
    unsafe { libc::atoi(c_string.as_ptr()) as i32 }
}

fn write_usage_and_exit(program: &[u8]) -> ! {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(b"usage: ");
    let _ = stderr.write_all(program);
    let _ = stderr.write_all(b" A B\n");
    process::exit(2);
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    if args.len() < 3 {
        let program = args
            .get(0)
            .map(|arg| arg.as_os_str().as_bytes())
            .unwrap_or(b"");
        write_usage_and_exit(program);
    }

    let a = atoi_arg(args[1].as_os_str().as_bytes());
    let b = atoi_arg(args[2].as_os_str().as_bytes());

    let mut out = io::BufWriter::new(io::stdout().lock());

    let r_call = op_fn()(a, b);
    let mut acc = init_for();
    run_loop(&mut acc, REPEAT);

    let x1 = helper_call(a, b, &mut out).expect("stdout write failed");
    let x2 = helper_ptr(a, b, &mut out).expect("stdout write failed");
    let x3 = use_generated(REPEAT, &mut out).expect("stdout write failed");
    let g_op: OpFn = op_fn();
    let g = g_op(a, b);

    writeln!(out, "op={} call={} acc={} g.call={}", OP_NAME, r_call, acc, g)
        .expect("stdout write failed");
    writeln!(
        out,
        "summary={}",
        r_call
            .wrapping_add(acc)
            .wrapping_add(x1)
            .wrapping_add(x2)
            .wrapping_add(x3)
            .wrapping_add(g)
    )
    .expect("stdout write failed");

    let _ = (op_sub as OpFn, op_mul as OpFn, init_for_sub(), init_for_mul(), step_sub, step_mul);
}

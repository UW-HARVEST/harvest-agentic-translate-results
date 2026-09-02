use std::os::fd::{FromRawFd, OwnedFd};
use std::process::{Command, Stdio};
use std::os::unix::process::ExitStatusExt;
extern "C" { fn pipe2(fds: *mut i32, flags: i32) -> i32; }
fn go(bin:&str)->(Option<i32>,Option<i32>){
    const O_CLOEXEC: i32 = 0o2000000;
    let mut fds=[-1i32;2];
    assert_eq!(unsafe{pipe2(fds.as_mut_ptr(),O_CLOEXEC)},0);
    let r=unsafe{OwnedFd::from_raw_fd(fds[0])};
    let w=unsafe{OwnedFd::from_raw_fd(fds[1])};
    let c=Command::new(bin).stdin(Stdio::null()).stdout(Stdio::from(w))
        .stderr(Stdio::piped()).spawn().unwrap();
    drop(r);
    let o=c.wait_with_output().unwrap();
    (o.status.code(), o.status.signal())
}
fn main(){ for b in ["./sp_c","./sp_rs","./sp_rs_fixed"] { println!("{b}: {:?}", go(b)); } }

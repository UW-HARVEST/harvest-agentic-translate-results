//! Harness self-check: both `.so`s load, export all three symbols, and agree
//! on a handful of trivial inputs. Run this first when debugging.

mod common;
use common::*;

#[test]
fn smoke_libraries_load_and_agree() {
    let (c, r) = open_pair();
    let cases = vec![
        Case::arch(b"Linux host 5.15.0 x86_64"),
        Case::arch(b"nothing here"),
        Case::regex(b"^([0-9]+)\\.*", b"10.0.19041", 2, 2),
        Case::parse(b"Microsoft Windows 10 [Ver: 10.0.19041]"),
        Case::parse(b"host |ubuntu [Ubuntu: 22.04 (jammy)] x86_64"),
    ];
    assert_same(&c, &r, &cases, "smoke");
}

#[test]
fn smoke_fork_harness_reports_signals() {
    // A NULL os_header dereferences inside strstr in *both* libraries; the
    // harness must observe the fatal signal instead of dying itself.
    let (c, r) = open_pair();
    let cases = vec![Case::Arch { input: None }];
    assert_same_isolated(&c, &r, &cases, "smoke-null");
}

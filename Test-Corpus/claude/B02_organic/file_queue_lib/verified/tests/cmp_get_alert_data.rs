//! Compare GetAlertData / FreeAlertData behavior between the C and Rust .so.

mod common;

use common::*;

fn run_one(input: &[u8], flag: i32) {
    let c_lib = load_c();
    let r_lib = load_rust();

    unsafe {
        let c_get: libloading::Symbol<FnGetAlertData> = sym(&c_lib, b"GetAlertData");
        let r_get: libloading::Symbol<FnGetAlertData> = sym(&r_lib, b"GetAlertData");
        let c_free: libloading::Symbol<FnFreeAlertData> = sym(&c_lib, b"FreeAlertData");
        let r_free: libloading::Symbol<FnFreeAlertData> = sym(&r_lib, b"FreeAlertData");

        let cf = fmemopen_ro(input);
        let rf = fmemopen_ro(input);
        assert!(!cf.is_null() && !rf.is_null(), "fmemopen failed");

        let c_out = c_get(flag, cf);
        let r_out = r_get(flag, rf);

        let c_snap = snapshot_alert(c_out);
        let r_snap = snapshot_alert(r_out);
        assert_eq!(c_snap, r_snap, "GetAlertData mismatch for flag={flag}");

        if !c_out.is_null() {
            c_free(c_out);
        }
        if !r_out.is_null() {
            r_free(r_out);
        }
        libc::fclose(cf);
        libc::fclose(rf);
    }
}

#[test]
fn empty_input() {
    run_one(b"", 0);
}

#[test]
fn truncated_alert_only_header() {
    let s = b"** Alert 1234567890.123: mail - syscheck,\n";
    run_one(s, 0);
}

#[test]
fn full_alert_no_mail_flag() {
    let s = b"** Alert 1500000000.123: mail - syscheck,pci_dss_10\n\
              2017 Jul 14 10:00:00 (host) 1.2.3.4->/var/log/auth.log\n\
              Rule: 1002 (level 5) -> 'Unknown problem'\n\
              Src IP: 192.168.0.1\n\
              Src Port: 22\n\
              Dst IP: 10.0.0.1\n\
              Dst Port: 443\n\
              User: alice\n\
              Some log line\n\
              ** Alert 1500000001.500: mail - other\n";
    run_one(s, 0);
}

#[test]
fn full_alert_with_mail_flag() {
    let s = b"** Alert 1500000000.123: mail - syscheck,pci_dss_10\n\
              2017 Jul 14 10:00:00 (host) 1.2.3.4->/var/log/auth.log\n\
              Rule: 1002 (level 5) -> 'Unknown problem'\n\
              Src IP: 192.168.0.1\n\
              Src Port: 22\n\
              Dst IP: 10.0.0.1\n\
              Dst Port: 443\n\
              User: alice\n\
              Some log line\n";
    run_one(s, 0x001);
}

#[test]
fn syscheck_with_integrity() {
    let s = b"** Alert 1500000000.999: mail - syscheck\n\
              2017 Jul 14 10:00:00 (host) 1.2.3.4->syscheck\n\
              Rule: 550 (level 7) -> 'Integrity checksum changed.'\n\
              Integrity checksum changed for: '/etc/passwd'\n";
    run_one(s, 0);
}

#[test]
fn alert_no_colon_after_id() {
    // Should hit `continue` due to missing ':' in alert header.
    let s = b"** Alert 1234 something\n";
    run_one(s, 0);
}

#[test]
fn alert_skip_non_mail_when_mail_flag_set() {
    let s = b"** Alert 1500000000.001: foo - bar\n\
              ** Alert 1500000000.002: mail - good\n\
              2017 Jul 14 10:00:00 host->loc\n\
              Rule: 100 (level 3) -> 'msg'\n";
    run_one(s, 0x001);
}

#[test]
fn empty_email_alert_no_dash() {
    let s = b"** Alert 1234567890.123: mail\n";
    run_one(s, 0);
}

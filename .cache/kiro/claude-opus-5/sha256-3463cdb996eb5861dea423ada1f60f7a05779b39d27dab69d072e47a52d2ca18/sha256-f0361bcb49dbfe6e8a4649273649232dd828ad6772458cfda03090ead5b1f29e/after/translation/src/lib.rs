//! Rust translation of the `driver` C shared library (Wazuh alert file queue
//! reader).
//!
//! Every public symbol exported by the original `libdriver.so` is reproduced
//! here with the identical name, signature and observable behaviour, including
//! the helpers that `shared.h` defines with external linkage.

pub mod cbind;
pub mod driver;
pub mod file_queue;
pub mod read_alert;
pub mod shared;

#[cfg(test)]
mod layout_tests {
    use super::cbind::{stat, tm};
    use super::file_queue::file_queue;
    use super::read_alert::alert_data;

    #[test]
    fn abi_layout_matches_c() {
        assert_eq!(size_of::<stat>(), 144);
        assert_eq!(size_of::<tm>(), 56);
        assert_eq!(size_of::<file_queue>(), 440);
        assert_eq!(size_of::<alert_data>(), 96);

        assert_eq!(core::mem::offset_of!(stat, st_mtim), 88);
        assert_eq!(core::mem::offset_of!(tm, tm_mday), 12);
        assert_eq!(core::mem::offset_of!(tm, tm_mon), 16);
        assert_eq!(core::mem::offset_of!(tm, tm_year), 20);

        assert_eq!(core::mem::offset_of!(file_queue, last_change), 0);
        assert_eq!(core::mem::offset_of!(file_queue, year), 8);
        assert_eq!(core::mem::offset_of!(file_queue, day), 12);
        assert_eq!(core::mem::offset_of!(file_queue, flags), 16);
        assert_eq!(core::mem::offset_of!(file_queue, mon), 20);
        assert_eq!(core::mem::offset_of!(file_queue, file_name), 24);
        assert_eq!(core::mem::offset_of!(file_queue, fp), 288);
        assert_eq!(core::mem::offset_of!(file_queue, f_status), 296);

        assert_eq!(core::mem::offset_of!(alert_data, level), 4);
        assert_eq!(core::mem::offset_of!(alert_data, alertid), 8);
        assert_eq!(core::mem::offset_of!(alert_data, srcport), 56);
        assert_eq!(core::mem::offset_of!(alert_data, filename), 88);
    }
}

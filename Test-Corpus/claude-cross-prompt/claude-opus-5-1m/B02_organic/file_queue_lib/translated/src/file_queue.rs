// Translation of c_src/src/file-queue.c
//
// Provides FileQueue, Init_FileQueue, Read_FileMon and an AlertSource
// abstraction to back the FILE* used by the C code.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use crate::read_alert::{get_alert_data, AlertData, AlertReader};

pub const ALERTS_DAILY: &str = "alerts.log";

pub const CRALERT_FP_SET: i32 = 0x010;
pub const CRALERT_READ_ALL: i32 = 0x004;

pub const MAX_FQUEUE: usize = 256;
pub const FQ_TIMEOUT: u64 = 5;

const S_MONTH: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Backs the C `FILE *fp`. Either an opened on-disk file or an in-memory
/// buffer (used when `CRALERT_FP_SET` is set, mirroring the C library
/// reading from stdin).
pub enum AlertSource {
    File(File, u64, u64), // file, position, length
    Memory(Vec<u8>, usize),
}

impl AlertSource {
    pub fn new_in_memory(data: Vec<u8>) -> Self {
        AlertSource::Memory(data, 0)
    }

    pub fn new_from_path(path: &str) -> Option<Self> {
        match File::open(path) {
            Ok(mut f) => {
                let len = f.metadata().ok().map(|m| m.len()).unwrap_or(0);
                let _ = f.seek(SeekFrom::Start(0));
                Some(AlertSource::File(f, 0, len))
            }
            Err(_) => None,
        }
    }

    pub fn seek_to_end(&mut self) -> bool {
        match self {
            AlertSource::File(f, pos, _len) => {
                if let Ok(end) = f.seek(SeekFrom::End(0)) {
                    *pos = end;
                    true
                } else {
                    false
                }
            }
            AlertSource::Memory(data, pos) => {
                *pos = data.len();
                true
            }
        }
    }
}

impl AlertReader for AlertSource {
    fn fgets(&mut self, max: usize) -> Option<Vec<u8>> {
        // Read up to max-1 bytes, stopping at '\n' (which is included).
        let limit = if max == 0 { 0 } else { max - 1 };
        match self {
            AlertSource::File(f, pos, _len) => {
                let mut buf = Vec::with_capacity(64);
                let mut byte = [0u8; 1];
                while buf.len() < limit {
                    match f.read(&mut byte) {
                        Ok(0) => break,
                        Ok(_) => {
                            *pos += 1;
                            buf.push(byte[0]);
                            if byte[0] == b'\n' {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                if buf.is_empty() {
                    None
                } else {
                    Some(buf)
                }
            }
            AlertSource::Memory(data, pos) => {
                if *pos >= data.len() {
                    return None;
                }
                let start = *pos;
                let mut end = start;
                while end < data.len() && (end - start) < limit {
                    let b = data[end];
                    end += 1;
                    if b == b'\n' {
                        break;
                    }
                }
                let out = data[start..end].to_vec();
                *pos = end;
                if out.is_empty() {
                    None
                } else {
                    Some(out)
                }
            }
        }
    }

    fn rewind_bytes(&mut self, n: usize) -> bool {
        match self {
            AlertSource::File(f, pos, _len) => {
                let new_pos = pos.saturating_sub(n as u64);
                if f.seek(SeekFrom::Start(new_pos)).is_ok() {
                    *pos = new_pos;
                    true
                } else {
                    false
                }
            }
            AlertSource::Memory(_data, pos) => {
                let new_pos = pos.saturating_sub(n);
                *pos = new_pos;
                true
            }
        }
    }

    fn at_eof(&self) -> bool {
        match self {
            AlertSource::File(_f, pos, len) => *pos >= *len,
            AlertSource::Memory(data, pos) => *pos >= data.len(),
        }
    }

    fn clear_err(&mut self) {
        // No persistent error state in our reader.
    }
}

/// Mirrors the `file_queue` struct.
pub struct FileQueue {
    pub last_change: i64,
    pub year: i32,
    pub day: i32,
    pub flags: i32,
    pub mon: [u8; 4],
    pub file_name: Vec<u8>,
    pub fp: Option<AlertSource>,
}

impl FileQueue {
    pub fn new() -> Self {
        FileQueue {
            last_change: 0,
            year: 0,
            day: 0,
            flags: 0,
            mon: [0; 4],
            file_name: vec![0u8; MAX_FQUEUE + 1],
            fp: None,
        }
    }
}

/// `struct tm`-style fields used by the driver.
#[derive(Default, Clone, Copy)]
pub struct Tm {
    pub tm_mday: i32,
    pub tm_mon: i32,
    pub tm_year: i32,
}

fn get_file_queue(fileq: &mut FileQueue) {
    // Mirror snprintf into file_name buffer
    let name = if (fileq.flags & CRALERT_FP_SET) != 0 {
        "<stdin>"
    } else {
        ALERTS_DAILY
    };
    fileq.file_name.fill(0);
    let bytes = name.as_bytes();
    let n = bytes.len().min(MAX_FQUEUE - 1);
    fileq.file_name[..n].copy_from_slice(&bytes[..n]);
    // ensure terminator (already zeroed)
}

fn handle_queue(fileq: &mut FileQueue, flags: i32, preset: Option<AlertSource>) -> i32 {
    if (flags & CRALERT_FP_SET) == 0 {
        // Close if open; reopen the file.
        fileq.fp = None;
        let name_str = String::from_utf8_lossy(
            &fileq.file_name[..fileq
                .file_name
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(fileq.file_name.len())],
        )
        .into_owned();
        match AlertSource::new_from_path(&name_str) {
            Some(s) => fileq.fp = Some(s),
            None => return 0,
        }
    } else if fileq.fp.is_none() {
        // First call when CRALERT_FP_SET is set: install caller-supplied source.
        fileq.fp = preset;
    }

    if (flags & CRALERT_READ_ALL) == 0 {
        match fileq.fp.as_mut() {
            None => return 0,
            Some(src) => {
                if !src.seek_to_end() {
                    fileq.fp = None;
                    return -1;
                }
            }
        }
    }

    fileq.last_change = 0;
    1
}

pub fn init_file_queue(
    fileq: &mut FileQueue,
    p: &Tm,
    flags: i32,
    initial_fp: Option<AlertSource>,
) -> i32 {
    if (flags & CRALERT_FP_SET) == 0 {
        fileq.fp = None;
    } else {
        fileq.fp = initial_fp;
    }
    fileq.last_change = 0;
    fileq.flags = 0;
    fileq.day = p.tm_mday;
    fileq.year = p.tm_year + 1900;

    let mon_idx = p.tm_mon.rem_euclid(12) as usize;
    let mon_bytes = S_MONTH[mon_idx].as_bytes();
    fileq.mon = [0; 4];
    let n = mon_bytes.len().min(3);
    fileq.mon[..n].copy_from_slice(&mon_bytes[..n]);

    fileq.file_name.fill(0);
    fileq.flags = flags;

    get_file_queue(fileq);

    if handle_queue(fileq, fileq.flags, None) < 0 {
        return -1;
    }
    0
}

pub fn read_file_mon(
    fileq: &mut FileQueue,
    p: &Tm,
    timeout: u32,
    sleep_fn: &mut dyn FnMut(),
) -> Option<AlertData> {
    if fileq.fp.is_none() {
        if handle_queue(fileq, 0, None) != 1 {
            sleep_fn();
            return None;
        }
    }

    if fileq.fp.is_none() {
        return None;
    }

    if let Some(fp) = fileq.fp.as_mut() {
        if let Some(d) = get_alert_data(fileq.flags, fp) {
            return Some(d);
        }
    }

    fileq.day = p.tm_mday;
    fileq.year = p.tm_year + 1900;
    let mon_idx = p.tm_mon.rem_euclid(12) as usize;
    let mon_bytes = S_MONTH[mon_idx].as_bytes();
    fileq.mon = [0; 4];
    let n = mon_bytes.len().min(3);
    fileq.mon[..n].copy_from_slice(&mon_bytes[..n]);

    get_file_queue(fileq);

    if handle_queue(fileq, 0, None) != 1 {
        sleep_fn();
        return None;
    }

    let mut i = 0u32;
    while i < timeout {
        if let Some(fp) = fileq.fp.as_mut() {
            if let Some(d) = get_alert_data(fileq.flags, fp) {
                return Some(d);
            }
        }
        i += 1;
        sleep_fn();
    }
    None
}

#[allow(dead_code)]
pub fn file_sleep() {
    std::thread::sleep(Duration::from_secs(FQ_TIMEOUT));
}

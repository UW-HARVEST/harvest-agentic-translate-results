use crate::dberror::{PAGE_SIZE, RC};
#[allow(unused_imports)]
use crate::tables::{Schema, Value};
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

pub struct SM_FileHandle {
    pub file_name: String,
    pub total_num_pages: i32,
    pub cur_page_pos: i32,
    pub mgmt_info: Option<Box<dyn std::any::Any>>,
}

pub type SM_PageHandle = String;

pub(crate) struct FileMgmt {
    pub file: RefCell<File>,
}

fn page_size() -> usize {
    PAGE_SIZE as usize
}

/// Helper: construct a String of the given length filled with NUL bytes
#[allow(dead_code)]
pub(crate) fn make_zero_page() -> String {
    // SAFETY: All-zero bytes are valid UTF-8.
    String::from_utf8(vec![0u8; page_size()]).unwrap()
}

/// Helper: convert an arbitrary byte slice to String. Uses unsafe because the
/// C interface stores arbitrary binary page data inside the SM_PageHandle
/// (typed `String`), which would not generally be valid UTF-8. The signature
/// of the public Rust API forces us to use String here.
pub(crate) fn bytes_to_handle_string(bytes: Vec<u8>) -> String {
    // SAFETY: String layout matches Vec<u8>; we accept the trade-off of
    // potentially non-UTF-8 content because the public API mandates String.
    unsafe { String::from_utf8_unchecked(bytes) }
}

pub(crate) fn handle_string_as_bytes_mut(s: &mut String) -> &mut [u8] {
    // SAFETY: We may write non-UTF-8 binary data here. The String storage
    // is just a heap buffer of bytes.
    unsafe { s.as_mut_vec().as_mut_slice() }
}

pub fn init_storage_manager() {
    // No-op: the C version sets a global pageFile to NULL.
}

pub fn create_page_file(file_name: &str) -> RC {
    let file = match OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(file_name)
    {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let empty = vec![0u8; page_size()];
    let mut f = file;
    if f.write_all(&empty).is_err() {
        return RC::WriteFailed;
    }
    if f.flush().is_err() {
        return RC::WriteFailed;
    }
    RC::Ok
}

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    let file = match OpenOptions::new().read(true).write(true).open(file_name) {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let mut f = file;
    let mut header = vec![0u8; page_size()];
    if f.read_exact(&mut header).is_err() {
        return RC::ReadFailed;
    }
    // Parse total num pages from the header (atoi-like behavior).
    let header_str: String = header
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| b as char)
        .collect();
    let total_num_pages: i32 = parse_atoi(&header_str);

    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total_num_pages;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(FileMgmt {
        file: RefCell::new(f),
    }));
    RC::Ok
}

fn parse_atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let mut chars = s.chars().peekable();
    let mut sign = 1i64;
    if let Some(&c) = chars.peek() {
        if c == '-' {
            sign = -1;
            chars.next();
        } else if c == '+' {
            chars.next();
        }
    }
    let mut n: i64 = 0;
    for c in chars {
        if let Some(d) = c.to_digit(10) {
            n = n.saturating_mul(10).saturating_add(d as i64);
        } else {
            break;
        }
    }
    (sign * n) as i32
}

fn get_file_mut<'a>(f_handle: &'a SM_FileHandle) -> Option<&'a FileMgmt> {
    f_handle
        .mgmt_info
        .as_ref()
        .and_then(|b| b.downcast_ref::<FileMgmt>())
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let mgmt = match f_handle.mgmt_info.as_ref() {
        Some(b) => match b.downcast_ref::<FileMgmt>() {
            Some(m) => m,
            None => return RC::FileHandleNotInit,
        },
        None => return RC::FileHandleNotInit,
    };

    // Write the header containing the total number of pages.
    let mut header = vec![0u8; page_size()];
    let total_str = f_handle.total_num_pages.to_string();
    let bytes = total_str.as_bytes();
    let n = bytes.len().min(header.len());
    header[..n].copy_from_slice(&bytes[..n]);

    {
        let mut file = mgmt.file.borrow_mut();
        if file.seek(SeekFrom::Start(0)).is_err() {
            return RC::SeekFailed;
        }
        if file.write_all(&header).is_err() {
            return RC::WriteFailed;
        }
        if file.flush().is_err() {
            return RC::WriteFailed;
        }
    }
    f_handle.mgmt_info = None;
    RC::Ok
}

pub fn destroy_page_file(file_name: &str) -> RC {
    for _ in 0..3 {
        if std::fs::remove_file(file_name).is_ok() {
            return RC::Ok;
        }
    }
    RC::DestroyFailed
}

fn ensure_handle_capacity(mem_page: &mut SM_PageHandle) {
    let needed = page_size();
    if mem_page.len() < needed {
        let mut bytes: Vec<u8> = std::mem::take(mem_page).into_bytes();
        bytes.resize(needed, 0);
        *mem_page = bytes_to_handle_string(bytes);
    }
}

pub fn read_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let mgmt = match get_file_mut(f_handle) {
        Some(m) => m,
        None => return RC::FileHandleNotInit,
    };
    let offset = (page_num as u64 + 1) * page_size() as u64;
    {
        let mut file = mgmt.file.borrow_mut();
        if file.seek(SeekFrom::Start(offset)).is_err() {
            return RC::SeekFailed;
        }
        ensure_handle_capacity(mem_page);
        let buf = handle_string_as_bytes_mut(mem_page);
        if file.read_exact(&mut buf[..page_size()]).is_err() {
            return RC::ReadFailed;
        }
    }
    f_handle.cur_page_pos = page_num;
    RC::Ok
}

pub fn get_block_pos(f_handle: &SM_FileHandle) -> i32 {
    f_handle.cur_page_pos
}

pub fn read_first_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    read_block(0, f_handle, mem_page)
}

pub fn read_previous_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos - 1;
    if page_num < 0 {
        return RC::ReadNonExistingPage;
    }
    read_block(page_num, f_handle, mem_page)
}

pub fn read_current_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if f_handle.mgmt_info.is_none() {
        return RC::FileHandleNotInit;
    }
    let pn = f_handle.cur_page_pos;
    if pn < 0 || pn >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    read_block(pn, f_handle, mem_page)
}

pub fn read_next_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos + 1;
    read_block(page_num, f_handle, mem_page)
}

pub fn read_last_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let pn = f_handle.total_num_pages - 1;
    if pn < 0 {
        return RC::ReadNonExistingPage;
    }
    read_block(pn, f_handle, mem_page)
}

pub fn write_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
    if page_num < 0 {
        return RC::WriteFailed;
    }
    if f_handle.mgmt_info.is_none() {
        return RC::FileNotFound;
    }
    let offset = (page_num as u64 + 1) * page_size() as u64;
    let mut increment_pages = false;
    {
        let mgmt = match get_file_mut(f_handle) {
            Some(m) => m,
            None => return RC::FileNotFound,
        };
        let mut file = mgmt.file.borrow_mut();
        let file_size = match file.seek(SeekFrom::End(0)) {
            Ok(p) => p,
            Err(_) => return RC::SeekFailed,
        };
        if offset > file_size {
            if page_num == f_handle.total_num_pages {
                if file.seek(SeekFrom::Start(file_size)).is_err() {
                    return RC::SeekFailed;
                }
                let pad = vec![0u8; (offset - file_size) as usize];
                if file.write_all(&pad).is_err() {
                    return RC::WriteFailed;
                }
                increment_pages = true;
            } else {
                return RC::WriteFailed;
            }
        }
        if file.seek(SeekFrom::Start(offset)).is_err() {
            return RC::SeekFailed;
        }
        let bytes = mem_page.as_bytes();
        let n = page_size().min(bytes.len());
        if file.write_all(&bytes[..n]).is_err() {
            return RC::WriteFailed;
        }
        if n < page_size() {
            let pad = vec![0u8; page_size() - n];
            if file.write_all(&pad).is_err() {
                return RC::WriteFailed;
            }
        }
        if file.flush().is_err() {
            return RC::WriteFailed;
        }
    }
    if increment_pages {
        f_handle.total_num_pages += 1;
    }
    f_handle.cur_page_pos = page_num;
    RC::Ok
}

pub fn write_current_block(f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
    let pn = f_handle.cur_page_pos;
    if pn < 0 || pn >= f_handle.total_num_pages {
        return RC::WriteFailed;
    }
    write_block(pn, f_handle, mem_page)
}

pub fn append_empty_block(f_handle: &mut SM_FileHandle) -> RC {
    let mgmt = match get_file_mut(f_handle) {
        Some(m) => m,
        None => return RC::FileHandleNotInit,
    };
    {
        let mut file = mgmt.file.borrow_mut();
        if file.seek(SeekFrom::End(0)).is_err() {
            return RC::SeekFailed;
        }
        let pad = vec![0u8; page_size()];
        if file.write_all(&pad).is_err() {
            return RC::WriteFailed;
        }
        if file.flush().is_err() {
            return RC::WriteFailed;
        }
    }
    f_handle.total_num_pages += 1;
    f_handle.cur_page_pos = f_handle.total_num_pages - 1;
    RC::Ok
}

pub fn ensure_capacity(number_of_pages: i32, f_handle: &mut SM_FileHandle) -> RC {
    if f_handle.mgmt_info.is_none() {
        return RC::FileHandleNotInit;
    }
    let active = f_handle.total_num_pages;
    if number_of_pages <= active {
        return RC::Ok;
    }
    let increment = number_of_pages - active;
    for _ in 0..increment {
        let rc = append_empty_block(f_handle);
        if rc != RC::Ok {
            return rc;
        }
    }
    RC::Ok
}

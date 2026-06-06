use crate::dberror::{RC, PAGE_SIZE};
use crate::tables::{Value, Schema};
use std::fs::{File, OpenOptions, remove_file};
use std::io::{Read, Write, Seek, SeekFrom};

pub struct SM_FileHandle {
    pub file_name: String,
    pub total_num_pages: i32,
    pub cur_page_pos: i32,
    pub mgmt_info: Option<Box<dyn std::any::Any>>,
}
pub type SM_PageHandle = String;

pub fn init_storage_manager() {
    // No-op
}

pub fn create_page_file(file_name: &str) -> RC {
    let result = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_name);
    let mut file = match result {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let empty_page = vec![0u8; PAGE_SIZE as usize];
    match file.write_all(&empty_page) {
        Ok(_) => RC::Ok,
        Err(_) => RC::WriteFailed,
    }
}

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    let mut file = match OpenOptions::new().read(true).write(true).open(file_name) {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let mut page_data = vec![0u8; PAGE_SIZE as usize];
    if file.read_exact(&mut page_data).is_err() {
        return RC::ReadFailed;
    }
    // Read total_num_pages from start of buffer (atoi-like behavior - parse leading digits)
    let total_num_pages = parse_leading_int(&page_data);
    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total_num_pages;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(file));
    RC::Ok
}

fn parse_leading_int(buf: &[u8]) -> i32 {
    // Mimics atoi: skip whitespace, optional sign, then digits.
    let mut i = 0usize;
    while i < buf.len() && (buf[i] == b' ' || buf[i] == b'\t' || buf[i] == b'\n' || buf[i] == b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < buf.len() && (buf[i] == b'-' || buf[i] == b'+') {
        if buf[i] == b'-' {
            neg = true;
        }
        i += 1;
    }
    let mut result: i32 = 0;
    while i < buf.len() && buf[i].is_ascii_digit() {
        result = result.saturating_mul(10).saturating_add((buf[i] - b'0') as i32);
        i += 1;
    }
    if neg { -result } else { result }
}

fn get_file_mut(f_handle: &mut SM_FileHandle) -> Option<&mut File> {
    f_handle.mgmt_info.as_mut().and_then(|b| b.downcast_mut::<File>())
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let total_num_pages = f_handle.total_num_pages;
    let file = match get_file_mut(f_handle) {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::Start(0)).is_err() {
        return RC::SeekFailed;
    }
    let header_str = format!("{}", total_num_pages);
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    let bytes = header_str.as_bytes();
    let n = bytes.len().min(buf.len());
    buf[..n].copy_from_slice(&bytes[..n]);
    if file.write_all(&buf).is_err() {
        return RC::WriteFailed;
    }
    // Drop the file by replacing mgmt_info with None
    f_handle.mgmt_info = None;
    RC::Ok
}

pub fn destroy_page_file(file_name: &str) -> RC {
    for _ in 0..3 {
        if remove_file(file_name).is_ok() {
            return RC::Ok;
        }
    }
    RC::DestroyFailed
}

pub fn read_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let offset = ((page_num + 1) as u64) * (PAGE_SIZE as u64);
    let file = match get_file_mut(f_handle) {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    if file.read_exact(&mut buf).is_err() {
        return RC::ReadFailed;
    }
    set_page_handle_bytes(mem_page, &buf);
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
    let page_num = f_handle.cur_page_pos;
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    read_block(page_num, f_handle, mem_page)
}

pub fn read_next_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos + 1;
    read_block(page_num, f_handle, mem_page)
}

pub fn read_last_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.total_num_pages - 1;
    if page_num < 0 {
        return RC::ReadNonExistingPage;
    }
    read_block(page_num, f_handle, mem_page)
}

pub fn write_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
    if page_num < 0 {
        return RC::WriteFailed;
    }
    let offset = ((page_num + 1) as u64) * (PAGE_SIZE as u64);
    let total_num_pages = f_handle.total_num_pages;
    let file = match get_file_mut(f_handle) {
        Some(f) => f,
        None => return RC::FileNotFound,
    };
    let file_size = match file.seek(SeekFrom::End(0)) {
        Ok(p) => p,
        Err(_) => return RC::SeekFailed,
    };
    let mut new_total = total_num_pages;
    if offset > file_size {
        if page_num == total_num_pages {
            // Pad with zeros up to offset
            let pad_len = (offset - file_size) as usize;
            if file.seek(SeekFrom::Start(file_size)).is_err() {
                return RC::SeekFailed;
            }
            let zeros = vec![0u8; pad_len];
            if file.write_all(&zeros).is_err() {
                return RC::WriteFailed;
            }
            new_total += 1;
        } else {
            return RC::WriteFailed;
        }
    }
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let bytes = page_handle_bytes(mem_page);
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    let n = bytes.len().min(buf.len());
    buf[..n].copy_from_slice(&bytes[..n]);
    if file.write_all(&buf).is_err() {
        return RC::WriteFailed;
    }
    f_handle.total_num_pages = new_total;
    f_handle.cur_page_pos = page_num;
    RC::Ok
}

pub fn write_current_block(f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos;
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::WriteFailed;
    }
    write_block(page_num, f_handle, mem_page)
}

pub fn append_empty_block(f_handle: &mut SM_FileHandle) -> RC {
    let file = match get_file_mut(f_handle) {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::End(0)).is_err() {
        return RC::SeekFailed;
    }
    let zeros = vec![0u8; PAGE_SIZE as usize];
    if file.write_all(&zeros).is_err() {
        return RC::WriteFailed;
    }
    f_handle.total_num_pages += 1;
    f_handle.cur_page_pos = f_handle.total_num_pages - 1;
    RC::Ok
}

pub fn ensure_capacity(number_of_pages: i32, f_handle: &mut SM_FileHandle) -> RC {
    if f_handle.mgmt_info.is_none() {
        return RC::FileHandleNotInit;
    }
    while f_handle.total_num_pages < number_of_pages {
        let rc = append_empty_block(f_handle);
        if rc != RC::Ok {
            return rc;
        }
    }
    RC::Ok
}

// Helper functions for working with String as binary buffer.
// The Rust signatures use `String` (== SM_PageHandle) for what is logically
// a byte buffer in C. We use unsafe { as_mut_vec() } because the data may
// contain arbitrary bytes that aren't valid UTF-8 (e.g. raw integer values).
pub(crate) fn page_handle_bytes(s: &SM_PageHandle) -> &[u8] {
    s.as_bytes()
}

pub(crate) fn set_page_handle_bytes(s: &mut SM_PageHandle, bytes: &[u8]) {
    unsafe {
        let v = s.as_mut_vec();
        v.clear();
        v.extend_from_slice(bytes);
    }
}

pub(crate) fn make_page_handle(size: usize) -> SM_PageHandle {
    let mut s = String::new();
    unsafe {
        s.as_mut_vec().resize(size, 0);
    }
    s
}

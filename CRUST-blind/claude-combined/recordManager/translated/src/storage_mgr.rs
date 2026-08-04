use crate::dberror::{RC, PAGE_SIZE};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

#[allow(non_camel_case_types)]
pub struct SM_FileHandle {
    pub file_name: String,
    pub total_num_pages: i32,
    pub cur_page_pos: i32,
    pub mgmt_info: Option<Box<dyn std::any::Any>>,
}

pub type SM_PageHandle = String;

pub fn init_storage_manager() {
    // No-op: equivalent of pageFile = NULL in C.
}

pub fn create_page_file(file_name: &str) -> RC {
    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_name)
    {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };

    let empty_page = vec![0u8; PAGE_SIZE as usize];
    let mut f = file;
    let written = match f.write(&empty_page) {
        Ok(n) => n,
        Err(_) => return RC::WriteFailed,
    };
    if let Err(_) = f.flush() {
        return RC::WriteFailed;
    }
    if written < PAGE_SIZE as usize {
        return RC::WriteFailed;
    }
    RC::Ok
}

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    let mut file = match OpenOptions::new().read(true).write(true).open(file_name) {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };

    let mut page_data = vec![0u8; PAGE_SIZE as usize];
    if let Err(_) = file.read_exact(&mut page_data) {
        return RC::ReadFailed;
    }
    // Parse total num pages from null-terminated ASCII string at start of buffer.
    let total_num_pages = parse_leading_int(&page_data);
    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total_num_pages;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(file));
    RC::Ok
}

fn parse_leading_int(buf: &[u8]) -> i32 {
    // C atoi: skip whitespace, parse optional sign, then digits until non-digit.
    let mut i = 0usize;
    while i < buf.len() && (buf[i] == b' ' || buf[i] == b'\t' || buf[i] == b'\n') {
        i += 1;
    }
    let mut neg = false;
    if i < buf.len() && (buf[i] == b'-' || buf[i] == b'+') {
        neg = buf[i] == b'-';
        i += 1;
    }
    let mut val: i64 = 0;
    while i < buf.len() && buf[i].is_ascii_digit() {
        val = val * 10 + (buf[i] - b'0') as i64;
        i += 1;
    }
    if neg {
        val = -val;
    }
    val as i32
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let mgmt = match f_handle.mgmt_info.as_mut() {
        Some(m) => m,
        None => return RC::FileHandleNotInit,
    };
    let file = match mgmt.downcast_mut::<File>() {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::Start(0)).is_err() {
        return RC::SeekFailed;
    }
    let header = format!("{}", f_handle.total_num_pages);
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    let bytes = header.as_bytes();
    let n = bytes.len().min(buf.len());
    buf[..n].copy_from_slice(&bytes[..n]);
    if file.write_all(&buf).is_err() {
        return RC::WriteFailed;
    }
    if file.flush().is_err() {
        return RC::WriteFailed;
    }
    f_handle.mgmt_info = None;
    RC::Ok
}

pub fn destroy_page_file(file_name: &str) -> RC {
    let mut trial = 0;
    let max_trial = 3;
    while trial < max_trial {
        if std::fs::remove_file(file_name).is_ok() {
            return RC::Ok;
        }
        trial += 1;
    }
    RC::DestroyFailed
}

pub fn read_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let mgmt = match f_handle.mgmt_info.as_mut() {
        Some(m) => m,
        None => return RC::FileHandleNotInit,
    };
    let file = match mgmt.downcast_mut::<File>() {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    let offset = (page_num + 1) as u64 * PAGE_SIZE as u64;
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    if file.read_exact(&mut buf).is_err() {
        return RC::ReadFailed;
    }
    // Use latin-1: each byte becomes a single char with the same numeric value.
    *mem_page = buf.iter().map(|&b| b as char).collect();
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
    if f_handle.mgmt_info.is_none() {
        return RC::FileNotFound;
    }
    if page_num < 0 {
        return RC::WriteFailed;
    }
    let total_pages = f_handle.total_num_pages;
    let mgmt = f_handle.mgmt_info.as_mut().unwrap();
    let file = match mgmt.downcast_mut::<File>() {
        Some(f) => f,
        None => return RC::FileNotFound,
    };
    let offset = (page_num + 1) as u64 * PAGE_SIZE as u64;
    let file_size = match file.seek(SeekFrom::End(0)) {
        Ok(s) => s,
        Err(_) => return RC::SeekFailed,
    };
    if offset > file_size {
        if page_num == total_pages {
            // pad with zero bytes up to offset
            if file.seek(SeekFrom::Start(file_size)).is_err() {
                return RC::SeekFailed;
            }
            let pad_len = (offset - file_size) as usize;
            let pad = vec![0u8; pad_len];
            if file.write_all(&pad).is_err() {
                return RC::WriteFailed;
            }
            f_handle.total_num_pages += 1;
        } else {
            return RC::WriteFailed;
        }
    }
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    // Convert chars back to bytes (latin-1)
    let chars: Vec<char> = mem_page.chars().collect();
    let n = chars.len().min(buf.len());
    for i in 0..n {
        buf[i] = chars[i] as u8;
    }
    if file.write_all(&buf).is_err() {
        return RC::WriteFailed;
    }
    if file.flush().is_err() {
        return RC::WriteFailed;
    }
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
    let mgmt = match f_handle.mgmt_info.as_mut() {
        Some(m) => m,
        None => return RC::FileHandleNotInit,
    };
    let file = match mgmt.downcast_mut::<File>() {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::End(0)).is_err() {
        return RC::SeekFailed;
    }
    let buf = vec![0u8; PAGE_SIZE as usize];
    if file.write_all(&buf).is_err() {
        return RC::WriteFailed;
    }
    if file.flush().is_err() {
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

use crate::dberror::RC;
use crate::tables::{Value, Schema};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};

pub struct SM_FileHandle {
    pub file_name: String,
    pub total_num_pages: i32,
    pub cur_page_pos: i32,
    pub mgmt_info: Option<Box<dyn std::any::Any>>,
}

pub type SM_PageHandle = String;

/// Helper: create a String of exactly PAGE_SIZE bytes (all null bytes)
fn empty_page() -> String {
    let bytes = vec![0u8; crate::dberror::PAGE_SIZE as usize];
    unsafe { String::from_utf8_unchecked(bytes) }
}

/// Helper: ensure a String is exactly PAGE_SIZE bytes, padding with nulls
fn pad_to_page(s: &str) -> String {
    let ps = crate::dberror::PAGE_SIZE as usize;
    let mut bytes = s.as_bytes().to_vec();
    bytes.resize(ps, 0);
    unsafe { String::from_utf8_unchecked(bytes) }
}

fn get_file(f_handle: &mut SM_FileHandle) -> Option<&mut File> {
    f_handle.mgmt_info.as_mut()?.downcast_mut::<File>()
}

pub fn init_storage_manager() {
    // nothing to do
}

pub fn create_page_file(file_name: &str) -> RC {
    let mut file = match File::create(file_name) {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let page = vec![0u8; crate::dberror::PAGE_SIZE as usize];
    match file.write_all(&page) {
        Ok(_) => RC::Ok,
        Err(_) => RC::WriteFailed,
    }
}

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    let mut file = match OpenOptions::new().read(true).write(true).open(file_name) {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let ps = crate::dberror::PAGE_SIZE as usize;
    let mut buf = vec![0u8; ps];
    match file.read_exact(&mut buf) {
        Ok(_) => {}
        Err(_) => return RC::ReadFailed,
    }
    // totalNumPages is stored as text in the header page
    let nul_pos = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let header_str = std::str::from_utf8(&buf[..nul_pos]).unwrap_or("0");
    let total = header_str.parse::<i32>().unwrap_or(0);

    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(file));
    RC::Ok
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let ps = crate::dberror::PAGE_SIZE as usize;
    let header_text = format!("{}", f_handle.total_num_pages);
    let mut page = vec![0u8; ps];
    let bytes = header_text.as_bytes();
    page[..bytes.len()].copy_from_slice(bytes);

    let file = match get_file(f_handle) {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::Start(0)).is_err() {
        return RC::SeekFailed;
    }
    if file.write_all(&page).is_err() {
        return RC::WriteFailed;
    }
    // drop the file to close it
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

pub fn read_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let ps = crate::dberror::PAGE_SIZE as usize;
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let offset = ((page_num + 1) as u64) * (ps as u64);
    let file = match get_file(f_handle) {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut buf = vec![0u8; ps];
    match file.read_exact(&mut buf) {
        Ok(_) => {}
        Err(_) => return RC::ReadFailed,
    }
    *mem_page = unsafe { String::from_utf8_unchecked(buf) };
    f_handle.cur_page_pos = page_num;
    RC::Ok
}

pub fn get_block_pos(f_handle: &SM_FileHandle) -> i32 {
    f_handle.cur_page_pos
}

pub fn read_first_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    for _ in 0..3 {
        let rc = read_block(0, f_handle, mem_page);
        if rc == RC::Ok { return rc; }
    }
    read_block(0, f_handle, mem_page)
}

pub fn read_previous_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos - 1;
    if page_num < 0 {
        return RC::ReadNonExistingPage;
    }
    for _ in 0..2 {
        let rc = read_block(page_num, f_handle, mem_page);
        if rc == RC::Ok { return rc; }
    }
    read_block(page_num, f_handle, mem_page)
}

pub fn read_current_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos;
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    for _ in 0..3 {
        let rc = read_block(page_num, f_handle, mem_page);
        if rc == RC::Ok { return rc; }
    }
    read_block(page_num, f_handle, mem_page)
}

pub fn read_next_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos + 1;
    for _ in 0..2 {
        let rc = read_block(page_num, f_handle, mem_page);
        if rc == RC::Ok { return rc; }
    }
    read_block(page_num, f_handle, mem_page)
}

pub fn read_last_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.total_num_pages - 1;
    if page_num < 0 {
        return RC::ReadNonExistingPage;
    }
    for _ in 0..3 {
        let rc = read_block(page_num, f_handle, mem_page);
        if rc == RC::Ok { return rc; }
    }
    read_block(page_num, f_handle, mem_page)
}

pub fn write_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
    let ps = crate::dberror::PAGE_SIZE as usize;
    if page_num < 0 {
        return RC::WriteFailed;
    }
    let offset = ((page_num + 1) as u64) * (ps as u64);
    let total_pages = f_handle.total_num_pages;
    let file = match get_file(f_handle) {
        Some(f) => f,
        None => return RC::FileNotFound,
    };
    // Check file size
    let file_size = match file.seek(SeekFrom::End(0)) {
        Ok(s) => s,
        Err(_) => return RC::SeekFailed,
    };
    if offset > file_size {
        if page_num == total_pages {
            // fill gap with zeros
            let _ = file.seek(SeekFrom::Start(file_size));
            let gap = (offset - file_size) as usize;
            let zeros = vec![0u8; gap];
            if file.write_all(&zeros).is_err() {
                return RC::WriteFailed;
            }
            f_handle.total_num_pages += 1;
        } else {
            return RC::WriteFailed;
        }
    }
    let file = match get_file(f_handle) {
        Some(f) => f,
        None => return RC::FileNotFound,
    };
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let data = mem_page.as_bytes();
    let mut buf = vec![0u8; ps];
    let copy_len = data.len().min(ps);
    buf[..copy_len].copy_from_slice(&data[..copy_len]);
    if file.write_all(&buf).is_err() {
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
    for _ in 0..3 {
        let rc = write_block(page_num, f_handle, mem_page);
        if rc == RC::Ok { return RC::Ok; }
    }
    RC::WriteFailed
}

pub fn append_empty_block(f_handle: &mut SM_FileHandle) -> RC {
    let ps = crate::dberror::PAGE_SIZE as usize;
    let file = match get_file(f_handle) {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::End(0)).is_err() {
        return RC::SeekFailed;
    }
    let page = vec![0u8; ps];
    for _ in 0..2 {
        if file.write_all(&page).is_ok() {
            f_handle.total_num_pages += 1;
            f_handle.cur_page_pos = f_handle.total_num_pages - 1;
            return RC::Ok;
        }
    }
    RC::WriteFailed
}

pub fn ensure_capacity(number_of_pages: i32, f_handle: &mut SM_FileHandle) -> RC {
    if number_of_pages <= f_handle.total_num_pages {
        return RC::Ok;
    }
    let increment = number_of_pages - f_handle.total_num_pages;
    for _ in 0..increment {
        let rc = append_empty_block(f_handle);
        if rc != RC::Ok {
            return rc;
        }
    }
    RC::Ok
}

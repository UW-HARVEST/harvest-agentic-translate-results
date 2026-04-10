use crate::dberror::{RC, PAGE_SIZE};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};

pub struct SM_FileHandle {
    pub file_name: String,
    pub total_num_pages: i32,
    pub cur_page_pos: i32,
    pub mgmt_info: Option<Box<dyn std::any::Any>>,
}

pub type SM_PageHandle = String;

pub fn init_storage_manager() {
    // Nothing to do in Rust
}

pub fn create_page_file(file_name: &str) -> RC {
    let mut file = match File::create(file_name) {
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
    match file.read_exact(&mut page_data) {
        Ok(_) => {}
        Err(_) => return RC::ReadFailed,
    }
    // Parse totalNumPages from the header page (stored as text string like C's atoi)
    let null_pos = page_data.iter().position(|&b| b == 0).unwrap_or(page_data.len());
    let header_str = String::from_utf8_lossy(&page_data[..null_pos]);
    let total_num_pages = header_str.trim().parse::<i32>().unwrap_or(0);

    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total_num_pages;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(file));
    RC::Ok
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let file = match f_handle.mgmt_info.as_mut() {
        Some(f) => f.downcast_mut::<File>().unwrap(),
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::Start(0)).is_err() {
        return RC::SeekFailed;
    }
    let mut page_data = vec![0u8; PAGE_SIZE as usize];
    let num_str = format!("{}", f_handle.total_num_pages);
    page_data[..num_str.len()].copy_from_slice(num_str.as_bytes());
    if file.write_all(&page_data).is_err() {
        return RC::WriteFailed;
    }
    // Drop the file to close it (take ownership)
    f_handle.mgmt_info = None;
    RC::Ok
}

pub fn destroy_page_file(file_name: &str) -> RC {
    for _ in 0..3 {
        if fs::remove_file(file_name).is_ok() {
            return RC::Ok;
        }
    }
    RC::DestroyFailed
}

pub fn read_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let file = match f_handle.mgmt_info.as_mut() {
        Some(f) => f.downcast_mut::<File>().unwrap(),
        None => return RC::FileHandleNotInit,
    };
    let offset = ((page_num + 1) as u64) * (PAGE_SIZE as u64);
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    match file.read_exact(&mut buf) {
        Ok(_) => {}
        Err(_) => return RC::ReadFailed,
    }
    // Store raw bytes as a String using Latin-1 mapping (each byte -> char)
    *mem_page = buf.iter().map(|&b| b as char).collect();
    f_handle.cur_page_pos = page_num;
    RC::Ok
}

pub fn get_block_pos(f_handle: &SM_FileHandle) -> i32 {
    f_handle.cur_page_pos
}

pub fn read_first_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let mut rc = RC::ReadFailed;
    for _ in 0..3 {
        rc = read_block(0, f_handle, mem_page);
        if rc == RC::Ok { break; }
    }
    rc
}

pub fn read_previous_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos - 1;
    if page_num < 0 {
        return RC::ReadNonExistingPage;
    }
    let mut rc = RC::ReadFailed;
    for _ in 0..2 {
        rc = read_block(page_num, f_handle, mem_page);
        if rc == RC::Ok { break; }
    }
    rc
}

pub fn read_current_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos;
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let mut rc = RC::ReadNonExistingPage;
    for _ in 0..3 {
        rc = read_block(page_num, f_handle, mem_page);
        if rc == RC::Ok { break; }
    }
    rc
}

pub fn read_next_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos + 1;
    let mut rc = RC::ReadFailed;
    for _ in 0..2 {
        rc = read_block(page_num, f_handle, mem_page);
        if rc == RC::Ok { break; }
    }
    rc
}

pub fn read_last_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.total_num_pages - 1;
    if page_num < 0 {
        return RC::ReadNonExistingPage;
    }
    let mut rc = RC::ReadFailed;
    for _ in 0..3 {
        rc = read_block(page_num, f_handle, mem_page);
        if rc == RC::Ok { break; }
    }
    rc
}

pub fn write_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
    let file = match f_handle.mgmt_info.as_mut() {
        Some(f) => f.downcast_mut::<File>().unwrap(),
        None => return RC::FileNotFound,
    };
    if page_num < 0 {
        return RC::WriteFailed;
    }
    let offset = ((page_num + 1) as u64) * (PAGE_SIZE as u64);
    let file_size = file.seek(SeekFrom::End(0)).unwrap_or(0);
    if offset > file_size {
        if page_num == f_handle.total_num_pages {
            // Pad with zeros up to offset
            if file.seek(SeekFrom::Start(file_size)).is_err() {
                return RC::SeekFailed;
            }
            let padding = vec![0u8; (offset - file_size) as usize];
            if file.write_all(&padding).is_err() {
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
    // Convert String back to bytes (Latin-1 mapping)
    let bytes: Vec<u8> = mem_page.chars().map(|c| c as u8).collect();
    let mut write_buf = vec![0u8; PAGE_SIZE as usize];
    let copy_len = bytes.len().min(PAGE_SIZE as usize);
    write_buf[..copy_len].copy_from_slice(&bytes[..copy_len]);
    if file.write_all(&write_buf).is_err() {
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
    let file = match f_handle.mgmt_info.as_mut() {
        Some(f) => f.downcast_mut::<File>().unwrap(),
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::End(0)).is_err() {
        return RC::SeekFailed;
    }
    let empty = vec![0u8; PAGE_SIZE as usize];
    for _ in 0..2 {
        if file.write_all(&empty).is_ok() {
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

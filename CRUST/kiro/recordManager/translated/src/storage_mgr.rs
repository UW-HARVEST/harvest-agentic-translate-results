use crate::dberror::RC;
use crate::tables::{Value, Schema};
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
    // Nothing to do
}

pub fn create_page_file(file_name: &str) -> RC {
    let mut file = match File::create(file_name) {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let empty_page = vec![0u8; crate::dberror::PAGE_SIZE as usize];
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
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let mut page_data = vec![0u8; page_size];
    match file.read_exact(&mut page_data) {
        Ok(_) => {},
        Err(_) => return RC::ReadFailed,
    }
    // The C code does atoi on the page data to get totalNumPages
    let nul_pos = page_data.iter().position(|&b| b == 0).unwrap_or(page_data.len());
    let header_str = String::from_utf8_lossy(&page_data[..nul_pos]);
    let total_num_pages: i32 = header_str.trim().parse().unwrap_or(0);

    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total_num_pages;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(file));
    RC::Ok
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let file = match f_handle.mgmt_info.as_mut() {
        Some(f) => f.downcast_mut::<File>().unwrap(),
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::Start(0)).is_err() {
        return RC::SeekFailed;
    }
    let mut page_data = vec![0u8; page_size];
    let header_str = format!("{}", f_handle.total_num_pages);
    page_data[..header_str.len()].copy_from_slice(header_str.as_bytes());
    match file.write_all(&page_data) {
        Ok(_) => {},
        Err(_) => return RC::WriteFailed,
    }
    // Drop the file to close it
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
    let page_size = crate::dberror::PAGE_SIZE as usize;
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let file = match f_handle.mgmt_info.as_mut() {
        Some(f) => f.downcast_mut::<File>().unwrap(),
        None => return RC::FileHandleNotInit,
    };
    let offset = ((page_num + 1) as u64) * (page_size as u64);
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut buf = vec![0u8; page_size];
    match file.read_exact(&mut buf) {
        Ok(_) => {},
        Err(_) => return RC::ReadFailed,
    }
    // Convert bytes to String using latin1 (each byte maps to a char)
    *mem_page = buf.iter().map(|&b| b as char).collect();
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
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let file = match f_handle.mgmt_info.as_mut() {
        Some(f) => f.downcast_mut::<File>().unwrap(),
        None => return RC::FileNotFound,
    };
    if page_num < 0 {
        return RC::WriteFailed;
    }
    let offset = ((page_num + 1) as u64) * (page_size as u64);
    let file_size = file.seek(SeekFrom::End(0)).unwrap_or(0);
    if offset > file_size {
        if page_num == f_handle.total_num_pages {
            // Extend file
            file.seek(SeekFrom::Start(file_size)).unwrap();
            let zeros = vec![0u8; (offset - file_size) as usize];
            file.write_all(&zeros).unwrap();
            f_handle.total_num_pages += 1;
        } else {
            return RC::WriteFailed;
        }
    }
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    // Convert String to bytes (latin1)
    let mut buf = vec![0u8; page_size];
    for (i, ch) in mem_page.chars().enumerate() {
        if i >= page_size { break; }
        buf[i] = ch as u8;
    }
    match file.write_all(&buf) {
        Ok(_) => {},
        Err(_) => return RC::WriteFailed,
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
    let page_size = crate::dberror::PAGE_SIZE as usize;
    let file = match f_handle.mgmt_info.as_mut() {
        Some(f) => f.downcast_mut::<File>().unwrap(),
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::End(0)).is_err() {
        return RC::SeekFailed;
    }
    let empty = vec![0u8; page_size];
    match file.write_all(&empty) {
        Ok(_) => {},
        Err(_) => return RC::WriteFailed,
    }
    f_handle.total_num_pages += 1;
    f_handle.cur_page_pos = f_handle.total_num_pages - 1;
    RC::Ok
}

pub fn ensure_capacity(number_of_pages: i32, f_handle: &mut SM_FileHandle) -> RC {
    while f_handle.total_num_pages < number_of_pages {
        let rc = append_empty_block(f_handle);
        if rc != RC::Ok {
            return rc;
        }
    }
    RC::Ok
}

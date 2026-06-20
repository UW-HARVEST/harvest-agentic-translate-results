use crate::dberror::{RC, PAGE_SIZE};
use crate::tables::{bytes_to_data, data_to_bytes, ensure_byte_len};
use std::any::Any;
use std::fs::{remove_file, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

pub struct SM_FileHandle {
    pub file_name: String,
    pub total_num_pages: i32,
    pub cur_page_pos: i32,
    pub mgmt_info: Option<Box<dyn Any>>,
}

pub type SM_PageHandle = String;

fn get_file_mut(f_handle: &mut SM_FileHandle) -> Result<&mut File, RC> {
    f_handle
        .mgmt_info
        .as_mut()
        .and_then(|info| info.downcast_mut::<File>())
        .ok_or(RC::FileHandleNotInit)
}

pub fn init_storage_manager() {}

pub fn create_page_file(file_name: &str) -> RC {
    match File::create(file_name) {
        Ok(mut file) => {
            let empty_page = vec![0u8; PAGE_SIZE as usize];
            match file.write_all(&empty_page) {
                Ok(_) => RC::Ok,
                Err(_) => RC::WriteFailed,
            }
        }
        Err(_) => RC::FileNotFound,
    }
}

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    match OpenOptions::new().read(true).write(true).open(file_name) {
        Ok(mut file) => {
            let mut header = vec![0u8; PAGE_SIZE as usize];
            if file.read_exact(&mut header).is_err() {
                return RC::ReadFailed;
            }
            let total_num_pages = String::from_utf8_lossy(&header)
                .trim_matches(char::from(0))
                .trim()
                .parse::<i32>()
                .unwrap_or(0);
            f_handle.file_name = file_name.to_string();
            f_handle.total_num_pages = total_num_pages;
            f_handle.cur_page_pos = 0;
            f_handle.mgmt_info = Some(Box::new(file));
            RC::Ok
        }
        Err(_) => RC::FileNotFound,
    }
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let total_pages = f_handle.total_num_pages;
    let file = match get_file_mut(f_handle) {
        Ok(file) => file,
        Err(rc) => return rc,
    };

    if file.seek(SeekFrom::Start(0)).is_err() {
        return RC::SeekFailed;
    }

    let mut header = vec![0u8; PAGE_SIZE as usize];
    let count = total_pages.to_string();
    header[..count.len()].copy_from_slice(count.as_bytes());

    if file.write_all(&header).is_err() {
        return RC::WriteFailed;
    }

    f_handle.mgmt_info = None;
    RC::Ok
}

pub fn destroy_page_file(file_name: &str) -> RC {
    match remove_file(file_name) {
        Ok(_) => RC::Ok,
        Err(_) => RC::DestroyFailed,
    }
}

pub fn read_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }

    let file = match get_file_mut(f_handle) {
        Ok(file) => file,
        Err(rc) => return rc,
    };

    let offset = ((page_num + 1) * PAGE_SIZE) as u64;
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }

    let mut buffer = vec![0u8; PAGE_SIZE as usize];
    if file.read_exact(&mut buffer).is_err() {
        return RC::ReadFailed;
    }

    *mem_page = bytes_to_data(&buffer);
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
    read_block(f_handle.cur_page_pos - 1, f_handle, mem_page)
}

pub fn read_current_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if f_handle.mgmt_info.is_none() {
        return RC::FileHandleNotInit;
    }
    read_block(f_handle.cur_page_pos, f_handle, mem_page)
}

pub fn read_next_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    read_block(f_handle.cur_page_pos + 1, f_handle, mem_page)
}

pub fn read_last_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    read_block(f_handle.total_num_pages - 1, f_handle, mem_page)
}

pub fn write_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
    if page_num < 0 {
        return RC::WriteFailed;
    }

    let total_num_pages = f_handle.total_num_pages;
    let file = match get_file_mut(f_handle) {
        Ok(file) => file,
        Err(_) => return RC::FileNotFound,
    };

    let offset = ((page_num + 1) * PAGE_SIZE) as u64;
    let file_size = match file.seek(SeekFrom::End(0)) {
        Ok(size) => size,
        Err(_) => return RC::SeekFailed,
    };

    if offset > file_size {
        if page_num == total_num_pages {
            if file.seek(SeekFrom::Start(file_size)).is_err() {
                return RC::SeekFailed;
            }
            let padding = vec![0u8; (offset - file_size) as usize];
            if file.write_all(&padding).is_err() {
                return RC::WriteFailed;
            }
        } else {
            return RC::WriteFailed;
        }
    }

    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }

    let mut data = data_to_bytes(mem_page);
    ensure_byte_len(&mut data, PAGE_SIZE as usize);
    if file.write_all(&data).is_err() {
        return RC::WriteFailed;
    }

    if offset > file_size && page_num == total_num_pages {
        f_handle.total_num_pages += 1;
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
    let file = match get_file_mut(f_handle) {
        Ok(file) => file,
        Err(rc) => return rc,
    };

    if file.seek(SeekFrom::End(0)).is_err() {
        return RC::SeekFailed;
    }
    let page = vec![0u8; PAGE_SIZE as usize];
    if file.write_all(&page).is_err() {
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

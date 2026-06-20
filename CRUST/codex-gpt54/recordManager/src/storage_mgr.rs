use crate::dberror::RC;
use crate::dberror::PAGE_SIZE;
use crate::tables::string_from_bytes;
use std::any::Any;
use std::fs::{remove_file, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
pub struct SM_FileHandle {
pub file_name: String,
pub total_num_pages: i32,
pub cur_page_pos: i32,
pub mgmt_info: Option<Box<dyn std::any::Any>>,
}
pub type SM_PageHandle = String;
pub fn init_storage_manager() {
}
struct StorageHandle {
    file: File,
}
pub fn create_page_file(file_name: &str) -> RC {
    let mut file = match OpenOptions::new().create(true).write(true).truncate(true).read(true).open(file_name) {
        Ok(file) => file,
        Err(_) => return RC::FileNotFound,
    };

    let mut header = vec![0_u8; PAGE_SIZE as usize];
    header[..1].copy_from_slice(b"1");
    if file.write_all(&header).is_err() || file.write_all(&vec![0_u8; PAGE_SIZE as usize]).is_err() {
        return RC::WriteFailed;
    }
    RC::Ok
}
pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    let mut file = match OpenOptions::new().read(true).write(true).open(file_name) {
        Ok(file) => file,
        Err(_) => return RC::FileNotFound,
    };
    let mut header = vec![0_u8; PAGE_SIZE as usize];
    if file.read_exact(&mut header).is_err() {
        return RC::ReadFailed;
    }
    let header_text = String::from_utf8_lossy(&header);
    let total_num_pages = header_text
        .trim_matches(char::from(0))
        .trim()
        .parse::<i32>()
        .ok()
        .filter(|pages| *pages > 0)
        .unwrap_or(1);

    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total_num_pages;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(StorageHandle { file }));
    RC::Ok
}
pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let Some(storage) = f_handle.mgmt_info.as_mut().and_then(|info| info.downcast_mut::<StorageHandle>()) else {
        return RC::FileHandleNotInit;
    };

    if storage.file.seek(SeekFrom::Start(0)).is_err() {
        return RC::SeekFailed;
    }
    let mut header = vec![0_u8; PAGE_SIZE as usize];
    let count = f_handle.total_num_pages.to_string();
    let copy_len = count.len().min(header.len());
    header[..copy_len].copy_from_slice(&count.as_bytes()[..copy_len]);
    if storage.file.write_all(&header).is_err() || storage.file.flush().is_err() {
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
    let Some(storage) = f_handle.mgmt_info.as_mut().and_then(|info| info.downcast_mut::<StorageHandle>()) else {
        return RC::FileHandleNotInit;
    };

    let offset = ((page_num + 1) * PAGE_SIZE) as u64;
    if storage.file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut buf = vec![0_u8; PAGE_SIZE as usize];
    if storage.file.read_exact(&mut buf).is_err() {
        return RC::ReadFailed;
    }
    *mem_page = string_from_bytes(buf);
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
    if page_num > f_handle.total_num_pages {
        return RC::WriteFailed;
    }
    if page_num == f_handle.total_num_pages {
        let rc = ensure_capacity(page_num + 1, f_handle);
        if rc != RC::Ok {
            return rc;
        }
    }

    let Some(storage) = f_handle.mgmt_info.as_mut().and_then(|info| info.downcast_mut::<StorageHandle>()) else {
        return RC::FileHandleNotInit;
    };
    let offset = ((page_num + 1) * PAGE_SIZE) as u64;
    if storage.file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut data = crate::tables::bytes_from_string(mem_page);
    data.resize(PAGE_SIZE as usize, 0);
    if storage.file.write_all(&data[..PAGE_SIZE as usize]).is_err() {
        return RC::WriteFailed;
    }
    f_handle.cur_page_pos = page_num;
    RC::Ok
}
pub fn write_current_block(f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
write_block(f_handle.cur_page_pos, f_handle, mem_page)
}
pub fn append_empty_block(f_handle: &mut SM_FileHandle) -> RC {
    let Some(storage) = f_handle.mgmt_info.as_mut().and_then(|info| info.downcast_mut::<StorageHandle>()) else {
        return RC::FileHandleNotInit;
    };
    if storage.file.seek(SeekFrom::End(0)).is_err() {
        return RC::SeekFailed;
    }
    if storage.file.write_all(&vec![0_u8; PAGE_SIZE as usize]).is_err() {
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

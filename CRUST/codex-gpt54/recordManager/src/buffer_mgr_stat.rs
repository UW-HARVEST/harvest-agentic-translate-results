use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle};
pub fn print_pool_content(bm: &BM_BufferPool) {
    println!("{}", sprint_pool_content(bm));
}
pub fn print_page_content(page: &BM_PageHandle) {
    println!("{}", sprint_page_content(page));
}
pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let frame_content = crate::buffer_mgr::get_frame_contents(bm);
    let dirty = crate::buffer_mgr::get_dirty_flags(bm);
    let fix_count = crate::buffer_mgr::get_fix_counts(bm);
    let mut message = String::new();
    for index in 0..bm.num_pages as usize {
        if index > 0 {
            message.push(',');
        }
        let page_num = frame_content.get(index).copied().unwrap_or(-1);
        let dirty_flag = dirty.get(index).copied().unwrap_or(false);
        let fix = fix_count.get(index).copied().unwrap_or(0);
        message.push_str(&format!("[{}{}{}]", page_num, if dirty_flag { "x" } else { " " }, fix));
    }
    message
}
pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let bytes = crate::tables::bytes_from_string(&page.data);
    let mut message = format!("[Page {}]\n", page.page_num);
    for (index, byte) in bytes.iter().enumerate().take(crate::dberror::PAGE_SIZE as usize) {
        message.push_str(&format!("{byte:02X}"));
        if (index + 1) % 8 == 0 {
            message.push(' ');
        }
        if (index + 1) % 64 == 0 {
            message.push('\n');
        }
    }
    message
}
pub fn print_strat(bm: &BM_BufferPool) {
    let strategy = match bm.strategy {
        crate::buffer_mgr::ReplacementStrategy::RsFifo => "FIFO",
        crate::buffer_mgr::ReplacementStrategy::RsLru => "LRU",
        crate::buffer_mgr::ReplacementStrategy::RsClock => "CLOCK",
        crate::buffer_mgr::ReplacementStrategy::RsLfu => "LFU",
        crate::buffer_mgr::ReplacementStrategy::RsLruK => "LRU-K",
    };
    print!("{strategy}");
}

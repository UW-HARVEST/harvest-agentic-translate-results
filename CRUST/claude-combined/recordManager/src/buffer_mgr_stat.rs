use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle, ReplacementStrategy};

pub fn print_pool_content(bm: &BM_BufferPool) {
    print!("{{");
    print_strat(bm);
    println!(" {}}}: ", bm.num_pages);
}

pub fn print_page_content(page: &BM_PageHandle) {
    println!("[Page {}]", page.page_num);
}

pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let mut message = String::new();
    let frame_content = crate::buffer_mgr::get_frame_contents(bm);
    let dirty = crate::buffer_mgr::get_dirty_flags(bm);
    let fix_count = crate::buffer_mgr::get_fix_counts(bm);
    for i in 0..bm.num_pages as usize {
        let prefix = if i == 0 { "" } else { "," };
        let fc = frame_content.get(i).copied().unwrap_or(-1);
        let d = dirty.get(i).copied().unwrap_or(false);
        let f = fix_count.get(i).copied().unwrap_or(0);
        message.push_str(&format!("{}[{}{}{}]", prefix, fc, if d { "x" } else { " " }, f));
    }
    message
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    format!("[Page {}]\n", page.page_num)
}

pub fn print_strat(bm: &BM_BufferPool) {
    match bm.strategy {
        ReplacementStrategy::RsFifo => print!("FIFO"),
        ReplacementStrategy::RsLru => print!("LRU"),
        ReplacementStrategy::RsClock => print!("CLOCK"),
        ReplacementStrategy::RsLfu => print!("LFU"),
        ReplacementStrategy::RsLruK => print!("LRU-K"),
    }
}

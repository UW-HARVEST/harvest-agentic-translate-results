use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle, ReplacementStrategy, get_frame_contents, get_dirty_flags, get_fix_counts};
use crate::dberror::PAGE_SIZE;

pub fn print_pool_content(bm: &BM_BufferPool) {
    print!("{}", sprint_pool_content(bm));
    println!();
}

pub fn print_page_content(page: &BM_PageHandle) {
    print!("{}", sprint_page_content(page));
}

pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let frame_content = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix_count = get_fix_counts(bm);
    let mut msg = String::new();
    for i in 0..bm.num_pages as usize {
        if i > 0 { msg.push(','); }
        msg.push_str(&format!("[{}{}{}]", frame_content[i], if dirty[i] { "x" } else { " " }, fix_count[i]));
    }
    msg
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let mut msg = format!("[Page {}]\n", page.page_num);
    let chars: Vec<char> = page.data.chars().collect();
    for i in 1..=PAGE_SIZE as usize {
        let byte = if i < chars.len() { chars[i] as u8 } else { 0 };
        msg.push_str(&format!("{:02X}", byte));
        if i % 8 == 0 { msg.push(' '); }
        if i % 64 == 0 { msg.push('\n'); }
    }
    msg
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

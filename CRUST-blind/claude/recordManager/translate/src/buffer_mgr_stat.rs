use crate::buffer_mgr::{
    get_dirty_flags, get_fix_counts, get_frame_contents, BM_BufferPool, BM_PageHandle,
    ReplacementStrategy,
};
use crate::dberror::PAGE_SIZE;

pub fn print_pool_content(bm: &BM_BufferPool) {
    println!("{}", sprint_pool_content(bm));
}

pub fn print_page_content(page: &BM_PageHandle) {
    println!("{}", sprint_page_content(page));
}

pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let mut out = String::new();
    out.push('{');
    push_strat(&mut out, bm);
    out.push_str(&format!(" {}}}: ", bm.num_pages));
    let frame = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix = get_fix_counts(bm);
    for i in 0..(bm.num_pages as usize) {
        let prefix = if i == 0 { "" } else { "," };
        let dirty_marker = if i < dirty.len() && dirty[i] { "x" } else { " " };
        let frame_val = if i < frame.len() { frame[i] } else { -1 };
        let fix_val = if i < fix.len() { fix[i] } else { 0 };
        out.push_str(&format!(
            "{}[{}{}{}]",
            prefix, frame_val, dirty_marker, fix_val
        ));
    }
    out
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let mut out = String::new();
    out.push_str(&format!("[Page {}]\n", page.page_num));
    let bytes = page.data.as_bytes();
    for i in 1..=(PAGE_SIZE as usize) {
        let b = if i < bytes.len() { bytes[i] } else { 0 };
        out.push_str(&format!("{:02X}", b));
        if i % 8 == 0 {
            out.push(' ');
        }
        if i % 64 == 0 {
            out.push('\n');
        }
    }
    out
}

pub fn print_strat(bm: &BM_BufferPool) {
    let mut tmp = String::new();
    push_strat(&mut tmp, bm);
    print!("{}", tmp);
}

fn push_strat(out: &mut String, bm: &BM_BufferPool) {
    match bm.strategy {
        ReplacementStrategy::RsFifo => out.push_str("FIFO"),
        ReplacementStrategy::RsLru => out.push_str("LRU"),
        ReplacementStrategy::RsClock => out.push_str("CLOCK"),
        ReplacementStrategy::RsLfu => out.push_str("LFU"),
        ReplacementStrategy::RsLruK => out.push_str("LRU-K"),
    }
}

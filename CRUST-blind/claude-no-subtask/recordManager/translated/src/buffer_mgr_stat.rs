use crate::buffer_mgr::{
    get_dirty_flags, get_fix_counts, get_frame_contents, BM_BufferPool, BM_PageHandle,
    ReplacementStrategy,
};
use crate::dberror::PAGE_SIZE;

pub fn print_pool_content(bm: &BM_BufferPool) {
    print!("{}", sprint_pool_content_full(bm));
    println!();
}

pub fn print_page_content(page: &BM_PageHandle) {
    print!("{}", sprint_page_content(page));
}

pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let frames = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix_counts = get_fix_counts(bm);
    let mut out = String::new();
    for i in 0..bm.num_pages as usize {
        let prefix = if i == 0 { "" } else { "," };
        let dirty_marker = if dirty.get(i).copied().unwrap_or(false) {
            "x"
        } else {
            " "
        };
        out.push_str(&format!(
            "{}[{}{}{}]",
            prefix,
            frames.get(i).copied().unwrap_or(-1),
            dirty_marker,
            fix_counts.get(i).copied().unwrap_or(0)
        ));
    }
    out
}

fn sprint_pool_content_full(bm: &BM_BufferPool) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str(&strategy_label(&bm.strategy));
    out.push_str(&format!(" {}}}: ", bm.num_pages));
    out.push_str(&sprint_pool_content(bm));
    out
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let mut out = String::new();
    out.push_str(&format!("[Page {}]\n", page.page_num));
    let bytes: Vec<u8> = page.data.chars().map(|c| (c as u32) as u8).collect();
    let ps = PAGE_SIZE as usize;
    for i in 1..=ps {
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
    print!("{}", strategy_label(&bm.strategy));
}

fn strategy_label(strategy: &ReplacementStrategy) -> String {
    match strategy {
        ReplacementStrategy::RsFifo => "FIFO".to_string(),
        ReplacementStrategy::RsLru => "LRU".to_string(),
        ReplacementStrategy::RsClock => "CLOCK".to_string(),
        ReplacementStrategy::RsLfu => "LFU".to_string(),
        ReplacementStrategy::RsLruK => "LRU-K".to_string(),
    }
}

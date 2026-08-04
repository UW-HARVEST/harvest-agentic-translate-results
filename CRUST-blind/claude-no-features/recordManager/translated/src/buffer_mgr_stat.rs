use crate::buffer_mgr::{
    get_dirty_flags, get_fix_counts, get_frame_contents, BM_BufferPool, BM_PageHandle,
    ReplacementStrategy,
};
use crate::dberror::PAGE_SIZE;

pub fn print_pool_content(bm: &BM_BufferPool) {
    let frame_content = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix_count = get_fix_counts(bm);

    print!("{{");
    print_strat(bm);
    print!(" {}}}: ", bm.num_pages);

    for i in 0..bm.num_pages as usize {
        let pre = if i == 0 { "" } else { "," };
        let dirty_marker = if dirty[i] { "x" } else { " " };
        print!(
            "{}[{}{}{}]",
            pre, frame_content[i], dirty_marker, fix_count[i]
        );
    }
    println!();
}

pub fn print_page_content(page: &BM_PageHandle) {
    println!("[Page {}]", page.page_num);
    let bytes = page.data.as_bytes();
    for i in 1..=PAGE_SIZE as usize {
        let b = if i < bytes.len() { bytes[i] } else { 0 };
        let space = if i % 8 == 0 { " " } else { "" };
        let nl = if i % 64 == 0 { "\n" } else { "" };
        print!("{:02X}{}{}", b, space, nl);
    }
}

pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let frame_content = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix_count = get_fix_counts(bm);
    let mut s = String::new();
    for i in 0..bm.num_pages as usize {
        let pre = if i == 0 { "" } else { "," };
        let dirty_marker = if dirty[i] { "x" } else { " " };
        s.push_str(&format!(
            "{}[{}{}{}]",
            pre, frame_content[i], dirty_marker, fix_count[i]
        ));
    }
    s
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let mut s = String::new();
    s.push_str(&format!("[Page {}]\n", page.page_num));
    let bytes = page.data.as_bytes();
    for i in 1..=PAGE_SIZE as usize {
        let b = if i < bytes.len() { bytes[i] } else { 0 };
        let space = if i % 8 == 0 { " " } else { "" };
        let nl = if i % 64 == 0 { "\n" } else { "" };
        s.push_str(&format!("{:02X}{}{}", b, space, nl));
    }
    s
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

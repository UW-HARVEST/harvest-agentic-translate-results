use crate::buffer_mgr::{get_dirty_flags, get_fix_counts, get_frame_contents, BM_BufferPool, BM_PageHandle};
use crate::dberror::PAGE_SIZE;
use crate::tables::data_to_bytes;

pub fn print_pool_content(bm: &BM_BufferPool) {
    print!("{{");
    print_strat(bm);
    print!(" {}}}: ", bm.num_pages);
    println!("{}", sprint_pool_content(bm));
}

pub fn print_page_content(page: &BM_PageHandle) {
    print!("{}", sprint_page_content(page));
}

pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let frame_content = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix_count = get_fix_counts(bm);
    let mut out = String::new();
    for i in 0..bm.num_pages as usize {
        if i > 0 {
            out.push(',');
        }
        let frame = *frame_content.get(i).unwrap_or(&-1);
        let dirty_marker = if *dirty.get(i).unwrap_or(&false) { "x" } else { " " };
        let fix = *fix_count.get(i).unwrap_or(&0);
        out.push_str(&format!("[{}{}{}]", frame, dirty_marker, fix));
    }
    out
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let bytes = data_to_bytes(&page.data);
    let mut out = format!("[Page {}]\n", page.page_num);
    for i in 1..=PAGE_SIZE as usize {
        let byte = bytes.get(i).copied().unwrap_or(0);
        out.push_str(&format!(
            "{:02X}{}{}",
            byte,
            if i % 8 == 0 { " " } else { "" },
            if i % 64 == 0 { "\n" } else { "" }
        ));
    }
    out
}

pub fn print_strat(bm: &BM_BufferPool) {
    let name = match bm.strategy {
        crate::buffer_mgr::ReplacementStrategy::RsFifo => "FIFO",
        crate::buffer_mgr::ReplacementStrategy::RsLru => "LRU",
        crate::buffer_mgr::ReplacementStrategy::RsClock => "CLOCK",
        crate::buffer_mgr::ReplacementStrategy::RsLfu => "LFU",
        crate::buffer_mgr::ReplacementStrategy::RsLruK => "LRU-K",
    };
    print!("{name}");
}

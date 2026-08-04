pub mod range {
#[derive(Debug, Clone, PartialEq)]
pub enum RangeType {
Empty,
Single,
StartEnd,
GreaterEqual,
}
#[derive(Debug, Clone)]
pub struct RangeElement {
pub start: u32,
pub end: u32,
pub rangetype: RangeType,
pub next: Option<Box<RangeElement>>,
}
impl RangeElement {
pub fn new() -> Self {
    // Match C rangeCreate(): start = -1, end = -1 (cast to uint32_t = u32::MAX),
    // rangetype is uninitialized in C; we use Empty as a safe default.
    RangeElement {
        start: u32::MAX,
        end: u32::MAX,
        rangetype: RangeType::Empty,
        next: None,
    }
}
pub fn add_single(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(num > 0);
    if start_of_list.is_none() {
        let mut e = Box::new(RangeElement::new());
        e.start = num;
        e.rangetype = RangeType::Single;
        return Some(e);
    }
    let mut head = start_of_list.unwrap();
    {
        let mut ptr: &mut RangeElement = &mut head;
        while ptr.next.is_some() {
            ptr = ptr.next.as_mut().unwrap();
        }
        let mut new_elem = Box::new(RangeElement::new());
        new_elem.start = num;
        new_elem.rangetype = RangeType::Single;
        ptr.next = Some(new_elem);
    }
    Some(head)
}
pub fn add_start_end(start: u32, end: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    assert!(start > 0);
    assert!(end >= start);
    if start_of_list.is_none() {
        let mut e = Box::new(RangeElement::new());
        e.start = start;
        if start == end {
            e.rangetype = RangeType::Single;
        } else {
            e.rangetype = RangeType::StartEnd;
            e.end = end;
        }
        return Some(e);
    }
    let mut head = start_of_list.unwrap();
    {
        let mut ptr: &mut RangeElement = &mut head;
        while ptr.next.is_some() {
            ptr = ptr.next.as_mut().unwrap();
        }
        let mut new_elem = Box::new(RangeElement::new());
        new_elem.start = start;
        if start == end {
            new_elem.rangetype = RangeType::Single;
        } else {
            new_elem.rangetype = RangeType::StartEnd;
            new_elem.end = end;
        }
        ptr.next = Some(new_elem);
    }
    Some(head)
}
pub fn add_greater_equal(num: u32, start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
    if start_of_list.is_none() {
        let mut e = Box::new(RangeElement::new());
        e.start = num;
        e.rangetype = RangeType::GreaterEqual;
        return Some(e);
    }
    let mut head = start_of_list.unwrap();
    {
        let mut ptr: &mut RangeElement = &mut head;
        while ptr.next.is_some() {
            ptr = ptr.next.as_mut().unwrap();
        }
        let mut new_elem = Box::new(RangeElement::new());
        new_elem.start = num;
        new_elem.rangetype = RangeType::GreaterEqual;
        ptr.next = Some(new_elem);
    }
    Some(head)
}
pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
    assert!(start_of_list.is_some());
    let mut cur: Option<&RangeElement> = start_of_list.as_deref();
    while let Some(ptr) = cur {
        match ptr.rangetype {
            RangeType::Single => {
                if ptr.start == num {
                    return true;
                }
            }
            RangeType::GreaterEqual => {
                if num >= ptr.start {
                    return true;
                }
            }
            RangeType::StartEnd => {
                if num >= ptr.start && num <= ptr.end {
                    return true;
                }
            }
            RangeType::Empty => {}
        }
        cur = ptr.next.as_deref();
    }
    false
}
pub fn to_string<'a>(&'a self, buf: &'a mut String, bufsize: usize) -> &'a str {
    let s = match self.rangetype {
        RangeType::Single => format!("[{}]", self.start),
        RangeType::GreaterEqual => format!("[{}-]", self.start),
        RangeType::StartEnd => format!("[{}-{}]", self.start, self.end),
        RangeType::Empty => "[]".to_string(),
    };
    // Emulate snprintf truncation: copy at most bufsize-1 bytes, leaving
    // room for nul terminator. Since Rust strings have no nul, just truncate.
    buf.clear();
    if bufsize == 0 {
        return buf.as_str();
    }
    let max = bufsize.saturating_sub(1);
    let truncated: String = s.chars().take(max).collect();
    // Actually snprintf truncates by bytes, not chars. Use bytes:
    buf.clear();
    let bytes = s.as_bytes();
    let take = std::cmp::min(bytes.len(), max);
    let truncated_bytes = &bytes[..take];
    if let Ok(s2) = std::str::from_utf8(truncated_bytes) {
        buf.push_str(s2);
    } else {
        buf.push_str(&truncated);
    }
    buf.as_str()
}
pub fn list_to_string<'a>(buf: &'a mut String, bufsize: usize, start_of_list: &'a Option<Box<RangeElement>>) -> &'a str {
    buf.clear();
    let mut cur: Option<&RangeElement> = start_of_list.as_deref();
    let mut bufidx: usize = 0;
    let tmpbufsize: usize = 256;
    while let Some(elem) = cur {
        let mut tmpbuf = String::new();
        elem.to_string(&mut tmpbuf, tmpbufsize);
        // Mimic C: tblen = sizeof(tmpbuf) = 256, which is wrong but matches the C code.
        // copylen = min(tblen, bufsize - bufidx) - but strncat will only copy up to nul.
        let tblen = tmpbufsize;
        let copylen = if (tblen + bufidx) >= bufsize {
            bufsize.saturating_sub(bufidx)
        } else {
            tblen
        };
        // strncat copies up to n bytes from src or until nul
        let src_bytes = tmpbuf.as_bytes();
        let to_copy = std::cmp::min(copylen, src_bytes.len());
        if to_copy > 0 {
            if let Ok(s) = std::str::from_utf8(&src_bytes[..to_copy]) {
                buf.push_str(s);
            }
        }
        bufidx += copylen;
        cur = elem.next.as_deref();
    }
    buf.as_str()
}
pub fn parse_int_ranges(text: &str) -> Option<Box<RangeElement>> {
    let flag_dash: i32 = 0x01;
    let mut state: i32 = 0;
    let mut numbuf = String::new();
    let mut rv: Option<Box<RangeElement>> = None;
    let mut start: u32 = 0;
    let mut end: u32;

    let bytes = text.as_bytes();
    let mut i: usize = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c >= '0' && c <= '9' {
            numbuf.push(c);
        } else if c == '-' {
            if state & flag_dash != 0 {
                eprintln!("error: too many dashes in range");
                std::process::exit((-1i32) as i32);
            }
            state |= flag_dash;
            if numbuf.is_empty() {
                eprintln!("error: range cannot start with '-'");
                std::process::exit((-1i32) as i32);
            }
            let parsed: i32 = numbuf.parse().unwrap_or(0);
            if parsed <= 0 {
                eprintln!("error: start range <= 0");
                std::process::exit((-1i32) as i32);
            }
            start = parsed as u32;
            numbuf.clear();
        } else if c == ',' {
            if numbuf.is_empty() {
                if state & flag_dash != 0 {
                    rv = RangeElement::add_greater_equal(start, rv);
                }
            } else if start > 0 {
                let parsed: i32 = numbuf.parse().unwrap_or(0);
                if parsed <= 0 || (start as i32) > parsed {
                    eprintln!("error: illegal start/end ranges");
                    std::process::exit((-1i32) as i32);
                }
                end = parsed as u32;
                rv = RangeElement::add_start_end(start, end, rv);
            } else {
                let parsed: i32 = numbuf.parse().unwrap_or(0);
                start = parsed as u32;
                rv = RangeElement::add_single(start, rv);
            }
            numbuf.clear();
            start = 0;
            state = 0;
        }
        i += 1;
    }
    // finish up
    if numbuf.is_empty() {
        if state & flag_dash != 0 {
            rv = RangeElement::add_greater_equal(start, rv);
        }
    } else if start > 0 {
        let parsed: i32 = numbuf.parse().unwrap_or(0);
        if parsed <= 0 || (start as i32) > parsed {
            eprintln!("error: illegal start/end range {}", text);
            std::process::exit((-1i32) as i32);
        }
        end = parsed as u32;
        rv = RangeElement::add_start_end(start, end, rv);
    } else {
        let parsed: i32 = numbuf.parse().unwrap_or(0);
        start = parsed as u32;
        rv = RangeElement::add_single(start, rv);
    }
    rv
}
}
}

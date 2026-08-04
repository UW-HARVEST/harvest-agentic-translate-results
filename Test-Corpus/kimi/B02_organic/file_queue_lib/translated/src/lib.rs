use std::ffi::{c_char, c_int, c_uint, CStr, CString};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::os::raw::c_void;
use std::ptr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const OS_MAXSTR: usize = 1024;
pub const MAX_FQUEUE: usize = 256;
pub const FQ_TIMEOUT: u64 = 5;

pub const CRALERT_MAIL_SET: c_int = 0x001;
pub const CRALERT_EXEC_SET: c_int = 0x002;
pub const CRALERT_READ_ALL: c_int = 0x004;
pub const CRALERT_READ_FAILED: c_int = 0x008;
pub const CRALERT_FP_SET: c_int = 0x010;

pub const ALERTS_DAILY: &str = "alerts.log";

const ALERT_BEGIN: &str = "** Alert";
const ALERT_BEGIN_SZ: usize = 8;
const RULE_BEGIN: &str = "Rule: ";
const RULE_BEGIN_SZ: usize = 6;
const SRCIP_BEGIN: &str = "Src IP: ";
const SRCIP_BEGIN_SZ: usize = 8;
const SRCPORT_BEGIN: &str = "Src Port: ";
const SRCPORT_BEGIN_SZ: usize = 10;
const DSTIP_BEGIN: &str = "Dst IP: ";
const DSTIP_BEGIN_SZ: usize = 8;
const DSTPORT_BEGIN: &str = "Dst Port: ";
const DSTPORT_BEGIN_SZ: usize = 10;
const USER_BEGIN: &str = "User: ";
const USER_BEGIN_SZ: usize = 6;
const ALERT_MAIL: &str = "mail";
const ALERT_MAIL_SZ: usize = 4;
const LOG_LIMIT: usize = 100;

#[repr(C)]
pub struct AlertData {
    pub rule: c_uint,
    pub level: c_uint,
    pub alertid: *mut c_char,
    pub date: *mut c_char,
    pub location: *mut c_char,
    pub comment: *mut c_char,
    pub group: *mut c_char,
    pub srcip: *mut c_char,
    pub srcport: c_int,
    pub dstip: *mut c_char,
    pub dstport: c_int,
    pub user: *mut c_char,
    pub filename: *mut c_char,
}

impl Default for AlertData {
    fn default() -> Self {
        AlertData {
            rule: 0,
            level: 0,
            alertid: ptr::null_mut(),
            date: ptr::null_mut(),
            location: ptr::null_mut(),
            comment: ptr::null_mut(),
            group: ptr::null_mut(),
            srcip: ptr::null_mut(),
            srcport: 0,
            dstip: ptr::null_mut(),
            dstport: 0,
            user: ptr::null_mut(),
            filename: ptr::null_mut(),
        }
    }
}

#[repr(C)]
pub struct FileQueue {
    pub last_change: i64,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut c_void,
    pub f_status_mtime: i64,
    pub f_status_size: i64,
}

impl Default for FileQueue {
    fn default() -> Self {
        FileQueue {
            last_change: 0,
            year: 0,
            day: 0,
            flags: 0,
            mon: [0; 4],
            file_name: [0; MAX_FQUEUE + 1],
            fp: ptr::null_mut(),
            f_status_mtime: 0,
            f_status_size: 0,
        }
    }
}

fn os_strdup(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

fn os_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

fn os_clearnl(s: &mut String) {
    if let Some(pos) =s.rfind('\n') {
        s.truncate(pos);
    }
}

fn file_sleep() {
    std::thread::sleep(Duration::from_secs(FQ_TIMEOUT));
}

fn get_file_queue(fileq: &mut FileQueue) {
    let name = if fileq.flags & CRALERT_FP_SET != 0 {
        "<stdin>"
    } else {
        ALERTS_DAILY
    };
    
    let bytes = name.as_bytes();
    let len = bytes.len().min(MAX_FQUEUE);
    fileq.file_name[..len].copy_from_slice(&bytes[..len].iter().map(|&b| b as c_char).collect::<Vec<_>>());
    fileq.file_name[len] = 0;
}

fn get_file_name(fileq: &FileQueue) -> String {
    let mut len = 0;
    while len < MAX_FQUEUE && fileq.file_name[len] != 0 {
        len += 1;
    }
    let bytes: Vec<u8> = fileq.file_name[..len].iter().map(|&c| c as u8).collect();
    String::from_utf8_lossy(&bytes).to_string()
}

fn get_mon_str(fileq: &FileQueue) -> String {
    let mut len = 0;
    while len < 3 && fileq.mon[len] != 0 {
        len += 1;
    }
    let bytes: Vec<u8> = fileq.mon[..len].iter().map(|&c| c as u8).collect();
    String::from_utf8_lossy(&bytes).to_string()
}

fn handle_queue(fileq: &mut FileQueue, flags: c_int) -> c_int {
    if flags & CRALERT_FP_SET == 0 {
        if !fileq.fp.is_null() {
            unsafe {
                let _ = Box::from_raw(fileq.fp as *mut BufReader<File>);
            }
            fileq.fp = ptr::null_mut();
        }
        
        let name = get_file_name(fileq);
        match File::open(&name) {
            Ok(file) => {
                let reader = Box::new(BufReader::new(file));
                fileq.fp = Box::into_raw(reader) as *mut c_void;
            }
            Err(_) => return 0,
        }
    }
    
    if flags & CRALERT_READ_ALL == 0 && !fileq.fp.is_null() {
        unsafe {
            let reader = &mut *(fileq.fp as *mut BufReader<File>);
            if let Err(_) = reader.seek(SeekFrom::End(0)) {
                return -1;
            }
        }
    }
    
    if !fileq.fp.is_null() {
        let name = get_file_name(fileq);
        match File::open(&name) {
            Ok(file) => {
                if let Ok(metadata) = file.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                            fileq.f_status_mtime = duration.as_secs() as i64;
                        }
                    }
                    fileq.f_status_size = metadata.len() as i64;
                    fileq.last_change = fileq.f_status_mtime;
                }
            }
            Err(_) => return -1,
        }
    }
    
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn Init_FileQueue(fileq: *mut FileQueue, p: *const libc::tm, flags: c_int) -> c_int {
    if fileq.is_null() || p.is_null() {
        return -1;
    }
    
    unsafe {
        let fileq = &mut *fileq;
        let p = &*p;
        
        if flags & CRALERT_FP_SET == 0 {
            fileq.fp = ptr::null_mut();
        }
        fileq.last_change = 0;
        fileq.flags = 0;
        fileq.day = p.tm_mday;
        fileq.year = p.tm_year + 1900;
        
        let s_month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                       "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        let mon = s_month[p.tm_mon as usize];
        let bytes = mon.as_bytes();
        fileq.mon[0] = bytes[0] as c_char;
        fileq.mon[1] = bytes[1] as c_char;
        fileq.mon[2] = bytes[2] as c_char;
        fileq.mon[3] = 0;
        
        fileq.file_name = [0; MAX_FQUEUE + 1];
        fileq.flags = flags;
        
        get_file_queue(fileq);
        
        if handle_queue(fileq, fileq.flags) < 0 {
            return -1;
        }
    }
    
    0
}

fn get_alert_data(flag: c_int, reader: &mut BufReader<File>) -> Option<Box<AlertData>> {
    let mut al_data = Box::new(AlertData::default());
    let mut _r = 0;
    let mut issyscheck = 0;
    let mut log_size = 0;
    
    let mut buffer = String::new();
    
    loop {
        buffer.clear();
        match reader.read_line(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        
        let mut str_line = buffer.clone();
        
        if str_line.starts_with(ALERT_BEGIN) {
            if _r == 2 {
                let pos = reader.stream_position().unwrap_or(0) as i64 - str_line.len() as i64;
                if pos >= 0 {
                    let _ = reader.seek(SeekFrom::Start(pos as u64));
                }
                return Some(al_data);
            }
            
            let p = &str_line[ALERT_BEGIN_SZ + 1..];
            if let Some(m) = p.find(':') {
                let z = m;
                let alertid = &p[..z];
                al_data.alertid = os_strdup(alertid);
                
                if let Some(space_pos) = p.find(' ') {
                    let p2 = &p[space_pos + 1..];
                    
                    if flag & CRALERT_MAIL_SET != 0 && !p2.starts_with(ALERT_MAIL) {
                        continue;
                    }
                    
                    if let Some(dash_pos) = p2.find('-') {
                        let mut group = p2[dash_pos + 1..].to_string();
                        while group.starts_with(' ') {
                            group.remove(0);
                        }
                        os_clearnl(&mut group);
                        if group.contains("syscheck") {
                            issyscheck = 1;
                        }
                        al_data.group = os_strdup(&group);
                    }
                }
            }
            
            _r = 1;
            continue;
        }
        
        if _r < 1 {
            continue;
        }
        
        if _r == 1 {
            os_clearnl(&mut str_line);
            
            if let Some(colon_pos) = str_line.find(':') {
                if let Some(space_pos) = str_line[colon_pos..].find(' ') {
                    let space_pos = colon_pos + space_pos;
                    let date = str_line[..space_pos].to_string();
                    let location = str_line[space_pos + 1..].to_string();
                    
                    if !al_data.date.is_null() || !al_data.location.is_null() {
                        return None;
                    }
                    
                    al_data.date = os_strdup(&date);
                    al_data.location = os_strdup(&location);
                    _r = 2;
                    log_size = 0;
                    continue;
                }
            }
        } else if _r == 2 {
            if str_line.starts_with(RULE_BEGIN) {
                os_clearnl(&mut str_line);
                let p = &str_line[RULE_BEGIN_SZ..];
                
                if let Ok(rule) = p.parse::<u32>() {
                    al_data.rule = rule;
                }
                
                if let Some(first_space) = p.find(' ') {
                    let p2 = &p[first_space + 1..];
                    if let Some(second_space) = p2.find(' ') {
                        let p3 = &p2[second_space + 1..];
                        if let Ok(level) = p3.parse::<u32>() {
                            al_data.level = level;
                        }
                        
                        if let Some(quote_pos) = p3.find('\'') {
                            let p4 = &p3[quote_pos + 1..];
                            let mut comment = p4.to_string();
                            if let Some(end_quote) = comment.rfind('\'') {
                                comment.truncate(end_quote);
                            }
                            al_data.comment = os_strdup(&comment);
                        }
                    }
                }
            } else if str_line.starts_with(SRCIP_BEGIN) {
                os_clearnl(&mut str_line);
                let p = &str_line[SRCIP_BEGIN_SZ..];
                al_data.srcip = os_strdup(p);
            } else if str_line.starts_with(SRCPORT_BEGIN) {
                os_clearnl(&mut str_line);
                let p = &str_line[SRCPORT_BEGIN_SZ..];
                if let Ok(port) = p.parse::<i32>() {
                    al_data.srcport = port;
                }
            } else if str_line.starts_with(DSTIP_BEGIN) {
                os_clearnl(&mut str_line);
                let p = &str_line[DSTIP_BEGIN_SZ..];
                al_data.dstip = os_strdup(p);
            } else if str_line.starts_with(DSTPORT_BEGIN) {
                os_clearnl(&mut str_line);
                let p = &str_line[DSTPORT_BEGIN_SZ..];
                if let Ok(port) = p.parse::<i32>() {
                    al_data.dstport = port;
                }
            } else if str_line.starts_with(USER_BEGIN) {
                os_clearnl(&mut str_line);
                let p = &str_line[USER_BEGIN_SZ..];
                al_data.user = os_strdup(p);
            } else if log_size < LOG_LIMIT {
                os_clearnl(&mut str_line);
                if issyscheck == 1 {
                    if str_line.starts_with("Integrity checksum changed for: '") {
                        let fname = &str_line[33..];
                        if fname.len() > 1 {
                            let fname = &fname[..fname.len() - 1];
                            al_data.filename = os_strdup(fname);
                        }
                    }
                    issyscheck = 0;
                }
            }
        }
    }
    
    if _r == 2 {
        Some(al_data)
    } else {
        None
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn GetAlertData(flag: c_int, fp: *mut libc::FILE) -> *mut AlertData {
    if fp.is_null() {
        return ptr::null_mut();
    }
    
    unsafe {
        let fd = libc::fileno(fp);
        let file = File::from_raw_fd(fd);
        let mut reader = BufReader::new(file);
        
        let result = get_alert_data(flag, &mut reader);
        
        let _ = reader.into_inner();
        
        match result {
            Some(data) => Box::into_raw(data),
            None => {
                libc::clearerr(fp);
                ptr::null_mut()
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn FreeAlertData(al_data: *mut AlertData) {
    if al_data.is_null() {
        return;
    }
    
    unsafe {
        let data = &mut *al_data;
        os_free(data.alertid);
        os_free(data.date);
        os_free(data.location);
        os_free(data.comment);
        os_free(data.group);
        os_free(data.srcip);
        os_free(data.dstip);
        os_free(data.user);
        os_free(data.filename);
        
        let _ = Box::from_raw(al_data);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn Read_FileMon(fileq: *mut FileQueue, p: *const libc::tm, timeout: c_uint) -> *mut AlertData {
    if fileq.is_null() || p.is_null() {
        return ptr::null_mut();
    }
    
    unsafe {
        let fileq = &mut *fileq;
        let p = &*p;
        
        if fileq.fp.is_null() {
            if handle_queue(fileq, 0) != 1 {
                file_sleep();
                return ptr::null_mut();
            }
        }
        
        if fileq.fp.is_null() {
            return ptr::null_mut();
        }
        
        let reader = &mut *(fileq.fp as *mut BufReader<File>);
        if let Some(al_data) = get_alert_data(fileq.flags, reader) {
            return Box::into_raw(al_data);
        }
        
        fileq.day = p.tm_mday;
        fileq.year = p.tm_year + 1900;
        
        let s_month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                       "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        let mon = s_month[p.tm_mon as usize];
        fileq.mon[0] = mon.as_bytes()[0] as c_char;
        fileq.mon[1] = mon.as_bytes()[1] as c_char;
        fileq.mon[2] = mon.as_bytes()[2] as c_char;
        fileq.mon[3] = 0;
        
        get_file_queue(fileq);
        
        if handle_queue(fileq, 0) != 1 {
            file_sleep();
            return ptr::null_mut();
        }
        
        let reader = &mut *(fileq.fp as *mut BufReader<File>);
        
        for _ in 0..timeout {
            if let Some(al_data) = get_alert_data(fileq.flags, reader) {
                return Box::into_raw(al_data);
            }
            file_sleep();
        }
    }
    
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(day: c_int, month: c_int, year: c_int, timeout: c_uint, flags: c_int) -> *mut AlertData {
    let mut time = libc::tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: day,
        tm_mon: month,
        tm_year: year,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: ptr::null(),
    };
    
    let mut fq = FileQueue::default();
    
    if Init_FileQueue(&mut fq, &time, flags) < 0 {
        eprintln!("File queue initialization failed");
        return ptr::null_mut();
    }
    
    let al_data = Read_FileMon(&mut fq, &time, timeout);
    
    if !fq.fp.is_null() {
        unsafe {
            let _ = Box::from_raw(fq.fp as *mut BufReader<File>);
        }
    }
    
    al_data
}

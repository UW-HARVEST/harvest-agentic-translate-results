// Translation of c_src/src/read-alert.c
//
// Provides:
//   - AlertData struct (mirroring `alert_data` in read-alert.h)
//   - get_alert_data() function (mirroring `GetAlertData`)
//   - free_alert_data() (no-op in safe Rust; Drop handles it)

use crate::shared::{os_clearnl, OS_MAXSTR};

// Flags
pub const CRALERT_MAIL_SET: i32 = 0x001;

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

const SYSCHECK_PREFIX: &str = "Integrity checksum changed for: '";
const SYSCHECK_PREFIX_SZ: usize = 33;

#[derive(Default, Debug, Clone)]
pub struct AlertData {
    pub rule: u32,
    pub level: u32,
    pub alertid: Option<String>,
    pub date: Option<String>,
    pub location: Option<String>,
    pub comment: Option<String>,
    pub group: Option<String>,
    pub srcip: Option<String>,
    pub srcport: i32,
    pub dstip: Option<String>,
    pub dstport: i32,
    pub user: Option<String>,
    pub filename: Option<String>,
}

/// Reader trait that mimics the parts of FILE* used by GetAlertData:
/// `fgets` (read up to a newline or buffer limit), `fseek` (rewind by a
/// number of bytes), `feof`, and `clearerr`.
pub trait AlertReader {
    /// Read up to `max - 1` bytes, stopping at the first newline (which is
    /// included in the returned slice if encountered). Returns None on EOF.
    fn fgets(&mut self, max: usize) -> Option<Vec<u8>>;
    /// Seek backwards by `n` bytes from the current position, like
    /// `fseek(fp, -n, SEEK_CUR)`. Returns true on success.
    fn rewind_bytes(&mut self, n: usize) -> bool;
    /// Whether the reader has reached the end of input.
    fn at_eof(&self) -> bool;
    /// Clear EOF/error state; matches `clearerr`.
    fn clear_err(&mut self);
}

/// Equivalent to `GetAlertData(int flag, FILE *fp)` in read-alert.c.
///
/// Returns `Some(AlertData)` if a complete alert was parsed, otherwise
/// `None`.  Reproduces the original control flow exactly, including the
/// `goto l_error` paths and the handling of the `_r` state machine.
pub fn get_alert_data<R: AlertReader>(flag: i32, fp: &mut R) -> Option<AlertData> {
    let mut al_data = AlertData::default();

    let mut _r: i32 = 0;
    let mut issyscheck: i32 = 0;
    let mut log_size: usize = 0;

    // Buffer mirrors `char str[OS_MAXSTR + 1]` with `str[OS_MAXSTR] = '\0'`.
    // We use OS_MAXSTR = 1024.
    let _max = OS_MAXSTR + 1;

    'outer: loop {
        // fgets(str, OS_MAXSTR, fp)
        let raw = match fp.fgets(OS_MAXSTR) {
            Some(b) => b,
            None => break 'outer,
        };
        // Convert to a UTF-8-lossy string for processing; we keep the raw
        // bytes for the seek-back length below to match C's strlen on the
        // exact buffer contents.
        let raw_len = raw.len();
        let str_lossy = String::from_utf8_lossy(&raw).into_owned();
        let mut s = str_lossy;

        // End of alert detection
        if s.starts_with(ALERT_BEGIN) {
            // End of the alert.
            if _r == 2 {
                // fseek(fp, -strlen(str), SEEK_CUR) -- C strlen ignores
                // embedded NULs but this buffer is a freshly read line so
                // raw_len matches.
                if fp.rewind_bytes(raw_len) {
                    return Some(al_data);
                } else {
                    return l_error(fp);
                }
            }

            // p = str + ALERT_BEGIN_SZ + 1;  -- skip "** Alert " prefix
            // Guard against short buffers.
            if s.len() <= ALERT_BEGIN_SZ + 1 {
                continue 'outer;
            }
            let p_start = ALERT_BEGIN_SZ + 1;
            let p_slice = &s[p_start..];

            // m = strstr(p, ":");
            let m_idx = match p_slice.find(':') {
                Some(idx) => idx,
                None => continue 'outer,
            };

            // z = strlen(p) - strlen(m); (length of the alertid token)
            let z = m_idx;
            // Allocate alertid as the first z bytes of p.
            al_data.alertid = Some(p_slice[..z].to_string());

            // Search for email flag: p = strchr(p, ' '); if (!p) continue;
            // p++;
            let space_off = match p_slice.find(' ') {
                Some(idx) => idx,
                None => continue 'outer,
            };
            // After p++ pointing past the space.
            let after_space = p_start + space_off + 1;

            // Check for email flag.
            if (flag & CRALERT_MAIL_SET) != 0 {
                // strncmp(ALERT_MAIL, p, ALERT_MAIL_SZ) != 0  -> continue
                if s.len() < after_space + ALERT_MAIL_SZ {
                    continue 'outer;
                }
                if &s[after_space..after_space + ALERT_MAIL_SZ] != ALERT_MAIL {
                    continue 'outer;
                }
            }

            // p = strchr(p, '-');  (search from current p position)
            if let Some(dash_off) = s[after_space..].find('-') {
                // p++ then skip leading spaces
                let mut group_start = after_space + dash_off + 1;
                let bytes = s.as_bytes();
                while group_start < bytes.len() && bytes[group_start] == b' ' {
                    group_start += 1;
                }
                let mut group = s[group_start..].to_string();
                os_clearnl(&mut group);
                let is_syscheck = group.contains("syscheck");
                al_data.group = Some(group);
                if is_syscheck {
                    issyscheck = 1;
                }
            }

            _r = 1;
            continue 'outer;
        }

        if _r < 1 {
            continue 'outer;
        }

        if _r == 1 {
            // Clear newline
            os_clearnl(&mut s);

            // p = strchr(str, ':');
            // if (p) { p = strchr(p, ' '); if (p) { *p = '\0'; p++; } else error }
            let (date_part, location_part) = if let Some(colon_idx) = s.find(':') {
                if let Some(rel_space) = s[colon_idx..].find(' ') {
                    let space_idx = colon_idx + rel_space;
                    let date = s[..space_idx].to_string();
                    let location = s[space_idx + 1..].to_string();
                    (date, location)
                } else {
                    eprint!("date of location not NULL: {}", errno_msg());
                    return l_error(fp);
                }
            } else {
                // p stayed null; al_data->date / location are NULL but p is also
                // null; the original code goes to l_error since !p.
                return l_error(fp);
            };

            if al_data.date.is_some() || al_data.location.is_some() {
                eprint!("date or location not NULL or p is NULL: {}", errno_msg());
                return l_error(fp);
            }

            al_data.date = Some(date_part);
            al_data.location = Some(location_part);
            _r = 2;
            log_size = 0;
            continue 'outer;
        } else if _r == 2 {
            // Rule begin
            if s.starts_with(RULE_BEGIN) {
                os_clearnl(&mut s);
                // p = str + RULE_BEGIN_SZ; al_data->rule = atoi(p);
                let p_after_rule = &s[RULE_BEGIN_SZ..];
                al_data.rule = c_atoi_u32(p_after_rule);

                // p = strchr(p, ' '); if (p) { p++; p = strchr(p, ' '); if (p) p++; }
                // Then check !p -> error.
                let space1 = p_after_rule.find(' ');
                let level_start_in_p = match space1 {
                    Some(s1) => {
                        let after_first = s1 + 1;
                        match p_after_rule[after_first..].find(' ') {
                            Some(rel) => Some(after_first + rel + 1),
                            None => None,
                        }
                    }
                    None => None,
                };
                let level_start_in_p = match level_start_in_p {
                    Some(v) => v,
                    None => return l_error(fp),
                };

                let p_for_level = &p_after_rule[level_start_in_p..];
                al_data.level = c_atoi_u32(p_for_level);

                // p = strchr(p, '\''); if (!p) error; p++; comment = strdup(p);
                let quote_off = match p_for_level.find('\'') {
                    Some(idx) => idx,
                    None => return l_error(fp),
                };
                let comment_start = quote_off + 1;
                let mut comment = p_for_level[comment_start..].to_string();

                // Must have closing '\''.
                match comment.rfind('\'') {
                    Some(end_idx) => {
                        comment.truncate(end_idx);
                    }
                    None => return l_error(fp),
                }
                al_data.comment = Some(comment);
            } else if s.starts_with(SRCIP_BEGIN) {
                os_clearnl(&mut s);
                let p = s[SRCIP_BEGIN_SZ..].to_string();
                al_data.srcip = Some(p);
            } else if s.starts_with(SRCPORT_BEGIN) {
                os_clearnl(&mut s);
                let p = &s[SRCPORT_BEGIN_SZ..];
                al_data.srcport = c_atoi_i32(p);
            } else if s.starts_with(DSTIP_BEGIN) {
                os_clearnl(&mut s);
                let p = s[DSTIP_BEGIN_SZ..].to_string();
                al_data.dstip = Some(p);
            } else if s.starts_with(DSTPORT_BEGIN) {
                os_clearnl(&mut s);
                let p = &s[DSTPORT_BEGIN_SZ..];
                al_data.dstport = c_atoi_i32(p);
            } else if s.starts_with(USER_BEGIN) {
                os_clearnl(&mut s);
                let p = s[USER_BEGIN_SZ..].to_string();
                al_data.user = Some(p);
            } else if log_size < LOG_LIMIT {
                os_clearnl(&mut s);
                if issyscheck == 1 {
                    if s.len() >= SYSCHECK_PREFIX_SZ
                        && &s[..SYSCHECK_PREFIX_SZ] == SYSCHECK_PREFIX
                    {
                        let mut filename = s[SYSCHECK_PREFIX_SZ..].to_string();
                        // strdup of (str + 33). Then trim the last byte
                        // (the trailing single quote) if non-empty.
                        if !filename.is_empty() {
                            // C: filename[strlen-1] = '\0' -> drop last byte.
                            // Operate on bytes since the original is byte-based.
                            let mut bytes = filename.into_bytes();
                            bytes.pop();
                            filename = String::from_utf8_lossy(&bytes).into_owned();
                        }
                        al_data.filename = Some(filename);
                    }
                    issyscheck = 0;
                }
                // log array is commented out in C source; mirror that.
                let _ = log_size; // keep in scope
            }
        }
    }

    // We reached the end of the alert and the information is saved.
    if fp.at_eof() && _r == 2 {
        return Some(al_data);
    }

    l_error(fp)
}

fn l_error<R: AlertReader>(fp: &mut R) -> Option<AlertData> {
    fp.clear_err();
    None
}

/// `atoi` semantics: skip leading whitespace, optional sign, then read
/// decimal digits until a non-digit.
fn c_atoi_i32(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let mut acc: i64 = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        acc = acc.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if neg {
        acc = acc.wrapping_neg();
    }
    acc as i32
}

fn c_atoi_u32(s: &str) -> u32 {
    c_atoi_i32(s) as u32
}

fn errno_msg() -> &'static str {
    // perror() in C prepends a string and prints the system error.
    // We don't have a real errno here; emit a plain newline-terminated tag.
    ""
}

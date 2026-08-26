use regex::Regex;

/// Equivalent of the C `os_data` struct. Fields are `Option<String>` to
/// represent the C library's `NULL` pointers when a piece of data could not
/// be extracted from the uname string.
#[derive(Debug, Default, Clone)]
pub struct OsData {
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_major: Option<String>,
    pub os_minor: Option<String>,
    pub os_codename: Option<String>,
    pub os_platform: Option<String>,
    pub os_build: Option<String>,
    pub os_uname: Option<String>,
    pub os_arch: Option<String>,
}

impl OsData {
    pub fn new() -> Self {
        OsData::default()
    }
}

/// Looks for the OS architecture in a string.
///
/// Mirrors the C `get_os_arch` function: walks through a fixed list of
/// architecture names in order and returns the first one found via
/// substring search.
fn get_os_arch(os_header: &str) -> Option<String> {
    const ARCHS: &[&str] = &[
        "x86_64", "i386", "i686", "sparc", "amd64", "i86pc", "ia64", "AIX",
        "armv6", "armv7", "aarch64", "arm64",
    ];
    for arch in ARCHS {
        if os_header.contains(arch) {
            return Some((*arch).to_string());
        }
    }
    None
}

/// Mirrors the C `w_regexec` function. Returns the first capture group
/// (group 1) of `pattern` against `string`, anchored to start of string
/// (the C code passes patterns starting with `^`).
///
/// Returns `Some(captured_string)` if the pattern matched and group 1 is
/// non-empty, or `None` otherwise.
fn regex_capture1(pattern: &str, string: &str) -> Option<String> {
    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Couldn't compile regular expression '{}'", pattern);
            return None;
        }
    };
    let caps = re.captures(string)?;
    let m = caps.get(1)?;
    Some(m.as_str().to_string())
}

/// Parses an OS uname string. Mirrors the behavior of the C
/// `parse_uname_string` function exactly, including the in-place
/// truncation logic (which affects which substring `get_os_arch` is called
/// with).
pub fn parse_uname_string(uname: &str, osd: &mut OsData) {
    // Mimic the C code which mutates the input buffer in-place. We
    // simulate that by using `working` as a substitute for `uname`'s
    // truncated state when needed.
    if let Some(idx) = uname.find(" [Ver: ") {
        // [Ver: os_major.os_minor.os_build]
        let name_part = &uname[..idx];
        // Skip past " [Ver: " (7 chars).
        let after = &uname[idx + 7..];
        // Strip the trailing ']' (last char).
        let str_tmp: &str = if !after.is_empty() {
            &after[..after.len() - 1]
        } else {
            after
        };

        osd.os_name = Some(name_part.to_string());

        // Get os_major
        if let Some(s) = regex_capture1(r"^([0-9]+)\.*", str_tmp) {
            osd.os_major = Some(s);
        }

        // Get os_minor
        if let Some(s) = regex_capture1(r"^[0-9]+\.([0-9]+)\.*", str_tmp) {
            osd.os_minor = Some(s);
        }

        // Get os_build (one or more dotted numeric segments)
        if let Some(s) = regex_capture1(r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", str_tmp) {
            osd.os_build = Some(s);
        }

        osd.os_version = Some(str_tmp.to_string());
        osd.os_platform = Some("windows".to_string());
    } else {
        // Track the working "uname" buffer state for the get_os_arch call
        // at the end. The C code truncates at " [" if present.
        let arch_input: String;

        if let Some(idx) = uname.find(" [") {
            arch_input = uname[..idx].to_string();
            // Move past " [" (2 chars).
            let after = &uname[idx + 2..];
            // os_name = strdup(after)
            let mut os_name = after.to_string();

            // Look for ": " inside os_name.
            if let Some(cidx) = os_name.find(": ") {
                let (name_only, rest) = os_name.split_at(cidx);
                let name_only = name_only.to_string();
                // skip ": "
                let version_with_bracket = &rest[2..];
                // Strip trailing ']'
                let mut os_version = if !version_with_bracket.is_empty() {
                    version_with_bracket[..version_with_bracket.len() - 1].to_string()
                } else {
                    version_with_bracket.to_string()
                };

                // Check for " (" -> codename
                let mut codename: Option<String> = None;
                if let Some(pidx) = os_version.find(" (") {
                    let (ver_only, paren_part) = os_version.split_at(pidx);
                    let ver_only = ver_only.to_string();
                    let cn_with_paren = &paren_part[2..];
                    // Strip trailing ')'
                    let cn = if !cn_with_paren.is_empty() {
                        cn_with_paren[..cn_with_paren.len() - 1].to_string()
                    } else {
                        cn_with_paren.to_string()
                    };
                    codename = Some(cn);
                    os_version = ver_only;
                }

                // Get os_major
                if let Some(s) = regex_capture1(r"^([0-9]+)\.*", &os_version) {
                    osd.os_major = Some(s);
                }
                // Get os_minor
                if let Some(s) = regex_capture1(r"^[0-9]+\.([0-9]+)\.*", &os_version) {
                    osd.os_minor = Some(s);
                }

                os_name = name_only;
                osd.os_version = Some(os_version);
                osd.os_codename = codename;
            } else {
                // No ": " inside. Strip last char (the ']') from os_name.
                if !os_name.is_empty() {
                    os_name.truncate(os_name.len() - 1);
                }
            }

            // os_name|os_platform
            if let Some(pidx) = os_name.find('|') {
                let (n, plat) = os_name.split_at(pidx);
                let n = n.to_string();
                let plat = plat[1..].to_string();
                osd.os_platform = Some(plat);
                osd.os_name = Some(n);
            } else {
                osd.os_name = Some(os_name);
            }
        } else {
            arch_input = uname.to_string();
        }

        if let Some(arch) = get_os_arch(&arch_input) {
            osd.os_arch = Some(arch);
        }
    }
}

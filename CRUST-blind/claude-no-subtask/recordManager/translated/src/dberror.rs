/* Module-wide constants */
use std::sync::Mutex;

pub const PAGE_SIZE: i32 = 4096;

/* Holder for error messages */
static RC_MESSAGE: Mutex<Option<String>> = Mutex::new(None);

/* Return code definitions */
#[derive(Debug, PartialEq, Eq)]
pub enum RC {
    Ok = 0,
    FileNotFound = 1,
    FileHandleNotInit = 2,
    WriteFailed = 3,
    ReadNonExistingPage = 4,
    // Record Manager Errors
    RmCompareValueOfDifferentDatatype = 200,
    RmExprResultIsNotBoolean = 201,
    RmBooleanExprArgIsNotBoolean = 202,
    RmNoMoreTuples = 203,
    RmNoPrintForDatatype = 204,
    RmUnknownDatatype = 205,
    // Index Manager Errors
    ImKeyNotFound = 300,
    ImKeyAlreadyExists = 301,
    ImNToLarge = 302,
    ImNoMoreEntries = 303,
    // General Errors
    MemoryAllocationFail = 401,
    BufferpoolInUse = 402,
    CloseFailed = 403,
    Error = 404,
    BufferpoolFull = 405,
    ReadFailed = 406,
    InvalidHeader = 407,
    SeekFailed = 408,
    DestroyFailed = 409,
    RecordNotFound = 410,
    GeneralError = 411,
    ShutdownWithoutInit = 420,
    LoggingSetupFailure = 430,
}

pub fn throw(rc: RC, message: &str) -> i32 {
    let code = rc as i32;
    if let Ok(mut guard) = RC_MESSAGE.lock() {
        *guard = Some(message.to_string());
    }
    code
}

pub fn check(code: i32) {
    if code != 0 {
        let guard = RC_MESSAGE.lock();
        let msg = guard
            .as_ref()
            .ok()
            .and_then(|g| g.as_ref().cloned())
            .unwrap_or_default();
        if !msg.is_empty() {
            println!("ERROR: Operation returned error: EC ({}), \"{}\"", code, msg);
        } else {
            println!("ERROR: Operation returned error: EC ({})", code);
        }
        std::process::exit(1);
    }
}

pub fn print_error(error: RC) {
    let code = error as i32;
    let guard = RC_MESSAGE.lock();
    let msg = guard
        .as_ref()
        .ok()
        .and_then(|g| g.as_ref().cloned());
    match msg {
        Some(m) if !m.is_empty() => println!("EC ({}), \"{}\"", code, m),
        _ => println!("EC ({})", code),
    }
}

pub fn error_message(error: RC) -> String {
    let code = error as i32;
    let guard = RC_MESSAGE.lock();
    let msg = guard
        .as_ref()
        .ok()
        .and_then(|g| g.as_ref().cloned());
    match msg {
        Some(m) if !m.is_empty() => format!("EC ({}), \"{}\"\n", code, m),
        _ => format!("EC ({})\n", code),
    }
}

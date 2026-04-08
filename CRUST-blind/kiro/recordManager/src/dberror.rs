/* Module-wide constants */
pub const PAGE_SIZE: i32 = 4096;
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

static mut RC_MESSAGE: Option<String> = None;

pub fn throw(rc: RC, message: &str) -> i32 {
    unsafe {
        RC_MESSAGE = Some(message.to_string());
    }
    rc as i32
}

pub fn check(code: i32) {
    if code != 0 {
        let message = error_message_from_code(code);
        println!("[check] ERROR: Operation returned error: {}", message);
        std::process::exit(1);
    }
}

pub fn print_error(error: RC) {
    unsafe {
        if let Some(ref msg) = RC_MESSAGE {
            println!("EC ({}), \"{}\"", error as i32, msg);
        } else {
            println!("EC ({})", error as i32);
        }
    }
}

pub fn error_message(error: RC) -> String {
    unsafe {
        if let Some(ref msg) = RC_MESSAGE {
            format!("EC ({}), \"{}\"\n", error as i32, msg)
        } else {
            format!("EC ({})\n", error as i32)
        }
    }
}

fn error_message_from_code(code: i32) -> String {
    unsafe {
        if let Some(ref msg) = RC_MESSAGE {
            format!("EC ({}), \"{}\"\n", code, msg)
        } else {
            format!("EC ({})\n", code)
        }
    }
}

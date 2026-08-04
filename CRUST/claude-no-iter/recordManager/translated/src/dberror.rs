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

use std::cell::RefCell;

thread_local! {
    static RC_MESSAGE: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn throw(rc: RC, message: &str) -> i32 {
    RC_MESSAGE.with(|m| {
        *m.borrow_mut() = Some(message.to_string());
    });
    rc as i32
}

pub fn check(code: i32) {
    if code != RC::Ok as i32 {
        let message = error_message_from_code(code);
        eprintln!("ERROR: Operation returned error: {}", message);
        std::process::exit(1);
    }
}

fn error_message_from_code(code: i32) -> String {
    RC_MESSAGE.with(|m| {
        let borrowed = m.borrow();
        if let Some(ref msg) = *borrowed {
            format!("EC ({}), \"{}\"\n", code, msg)
        } else {
            format!("EC ({})\n", code)
        }
    })
}

pub fn print_error(error: RC) {
    let code = error as i32;
    let formatted = error_message_from_code(code);
    print!("{}", formatted);
}

pub fn error_message(error: RC) -> String {
    let code = error as i32;
    error_message_from_code(code)
}

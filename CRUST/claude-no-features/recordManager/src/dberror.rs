/* Module-wide constants */
pub const PAGE_SIZE: i32 = 4096;
/* Return code definitions */
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

impl RC {
    pub fn code(&self) -> i32 {
        match self {
            RC::Ok => 0,
            RC::FileNotFound => 1,
            RC::FileHandleNotInit => 2,
            RC::WriteFailed => 3,
            RC::ReadNonExistingPage => 4,
            RC::RmCompareValueOfDifferentDatatype => 200,
            RC::RmExprResultIsNotBoolean => 201,
            RC::RmBooleanExprArgIsNotBoolean => 202,
            RC::RmNoMoreTuples => 203,
            RC::RmNoPrintForDatatype => 204,
            RC::RmUnknownDatatype => 205,
            RC::ImKeyNotFound => 300,
            RC::ImKeyAlreadyExists => 301,
            RC::ImNToLarge => 302,
            RC::ImNoMoreEntries => 303,
            RC::MemoryAllocationFail => 401,
            RC::BufferpoolInUse => 402,
            RC::CloseFailed => 403,
            RC::Error => 404,
            RC::BufferpoolFull => 405,
            RC::ReadFailed => 406,
            RC::InvalidHeader => 407,
            RC::SeekFailed => 408,
            RC::DestroyFailed => 409,
            RC::RecordNotFound => 410,
            RC::GeneralError => 411,
            RC::ShutdownWithoutInit => 420,
            RC::LoggingSetupFailure => 430,
        }
    }
}

use std::sync::Mutex;

static RC_MESSAGE: Mutex<Option<String>> = Mutex::new(None);

pub fn throw(rc: RC, message: &str) -> i32 {
    if let Ok(mut guard) = RC_MESSAGE.lock() {
        *guard = Some(message.to_string());
    }
    rc.code()
}

pub fn check(code: i32) {
    if code != RC::Ok.code() {
        let message = error_message_for_code(code);
        println!("ERROR: Operation returned error: {}", message);
        std::process::exit(1);
    }
}

pub fn print_error(error: RC) {
    let guard = RC_MESSAGE.lock();
    match guard {
        Ok(g) => match &*g {
            Some(msg) => println!("EC ({}), \"{}\"", error.code(), msg),
            None => println!("EC ({})", error.code()),
        },
        Err(_) => println!("EC ({})", error.code()),
    }
}

pub fn error_message(error: RC) -> String {
    error_message_for_code(error.code())
}

fn error_message_for_code(code: i32) -> String {
    let guard = RC_MESSAGE.lock();
    match guard {
        Ok(g) => match &*g {
            Some(msg) => format!("EC ({}), \"{}\"\n", code, msg),
            None => format!("EC ({})\n", code),
        },
        Err(_) => format!("EC ({})\n", code),
    }
}

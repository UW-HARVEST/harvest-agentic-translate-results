use std::cell::RefCell;

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
    pub fn as_i32(&self) -> i32 {
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

thread_local! {
    static RC_MESSAGE: RefCell<Option<String>> = RefCell::new(None);
}

pub fn throw(rc: RC, message: &str) -> i32 {
    RC_MESSAGE.with(|m| {
        *m.borrow_mut() = Some(message.to_string());
    });
    rc.as_i32()
}

pub fn check(code: i32) {
    if code != RC::Ok.as_i32() {
        // Try to find an RC with the matching code
        let rc = match code {
            0 => RC::Ok,
            1 => RC::FileNotFound,
            2 => RC::FileHandleNotInit,
            3 => RC::WriteFailed,
            4 => RC::ReadNonExistingPage,
            200 => RC::RmCompareValueOfDifferentDatatype,
            201 => RC::RmExprResultIsNotBoolean,
            202 => RC::RmBooleanExprArgIsNotBoolean,
            203 => RC::RmNoMoreTuples,
            204 => RC::RmNoPrintForDatatype,
            205 => RC::RmUnknownDatatype,
            300 => RC::ImKeyNotFound,
            301 => RC::ImKeyAlreadyExists,
            302 => RC::ImNToLarge,
            303 => RC::ImNoMoreEntries,
            401 => RC::MemoryAllocationFail,
            402 => RC::BufferpoolInUse,
            403 => RC::CloseFailed,
            404 => RC::Error,
            405 => RC::BufferpoolFull,
            406 => RC::ReadFailed,
            407 => RC::InvalidHeader,
            408 => RC::SeekFailed,
            409 => RC::DestroyFailed,
            410 => RC::RecordNotFound,
            411 => RC::GeneralError,
            420 => RC::ShutdownWithoutInit,
            430 => RC::LoggingSetupFailure,
            _ => RC::Error,
        };
        let message = error_message(rc);
        eprintln!("ERROR: Operation returned error: {}", message);
        std::process::exit(1);
    }
}

pub fn print_error(error: RC) {
    let msg = RC_MESSAGE.with(|m| m.borrow().clone());
    match msg {
        Some(m) => println!("EC ({}), \"{}\"", error.as_i32(), m),
        None => println!("EC ({})", error.as_i32()),
    }
}

pub fn error_message(error: RC) -> String {
    let msg = RC_MESSAGE.with(|m| m.borrow().clone());
    match msg {
        Some(m) => format!("EC ({}), \"{}\"\n", error.as_i32(), m),
        None => format!("EC ({})\n", error.as_i32()),
    }
}

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

impl Clone for RC {
    fn clone(&self) -> Self {
        match self {
            RC::Ok => RC::Ok,
            RC::FileNotFound => RC::FileNotFound,
            RC::FileHandleNotInit => RC::FileHandleNotInit,
            RC::WriteFailed => RC::WriteFailed,
            RC::ReadNonExistingPage => RC::ReadNonExistingPage,
            RC::RmCompareValueOfDifferentDatatype => RC::RmCompareValueOfDifferentDatatype,
            RC::RmExprResultIsNotBoolean => RC::RmExprResultIsNotBoolean,
            RC::RmBooleanExprArgIsNotBoolean => RC::RmBooleanExprArgIsNotBoolean,
            RC::RmNoMoreTuples => RC::RmNoMoreTuples,
            RC::RmNoPrintForDatatype => RC::RmNoPrintForDatatype,
            RC::RmUnknownDatatype => RC::RmUnknownDatatype,
            RC::ImKeyNotFound => RC::ImKeyNotFound,
            RC::ImKeyAlreadyExists => RC::ImKeyAlreadyExists,
            RC::ImNToLarge => RC::ImNToLarge,
            RC::ImNoMoreEntries => RC::ImNoMoreEntries,
            RC::MemoryAllocationFail => RC::MemoryAllocationFail,
            RC::BufferpoolInUse => RC::BufferpoolInUse,
            RC::CloseFailed => RC::CloseFailed,
            RC::Error => RC::Error,
            RC::BufferpoolFull => RC::BufferpoolFull,
            RC::ReadFailed => RC::ReadFailed,
            RC::InvalidHeader => RC::InvalidHeader,
            RC::SeekFailed => RC::SeekFailed,
            RC::DestroyFailed => RC::DestroyFailed,
            RC::RecordNotFound => RC::RecordNotFound,
            RC::GeneralError => RC::GeneralError,
            RC::ShutdownWithoutInit => RC::ShutdownWithoutInit,
            RC::LoggingSetupFailure => RC::LoggingSetupFailure,
        }
    }
}

pub fn throw(rc: RC, message: &str) -> i32 {
    let code = rc_to_i32(&rc);
    eprintln!("[ERROR {}] {}", code, message);
    code
}

pub fn check(code: i32) {
    if code != 0 {
        let msg = error_message_from_code(code);
        eprintln!("ERROR: Operation returned error: {}", msg);
        std::process::exit(1);
    }
}

pub fn print_error(error: RC) {
    let code = rc_to_i32(&error);
    println!("EC ({})", code);
}

pub fn error_message(error: RC) -> String {
    let code = rc_to_i32(&error);
    error_message_from_code(code)
}

fn error_message_from_code(code: i32) -> String {
    format!("EC ({})\n", code)
}

fn rc_to_i32(rc: &RC) -> i32 {
    match rc {
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

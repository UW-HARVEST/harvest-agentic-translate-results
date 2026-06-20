use std::sync::{Mutex, OnceLock};

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

fn rc_message() -> &'static Mutex<Option<String>> {
    static RC_MESSAGE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    RC_MESSAGE.get_or_init(|| Mutex::new(None))
}

pub fn throw(rc: RC, message: &str) -> i32 {
    if let Ok(mut slot) = rc_message().lock() {
        *slot = Some(message.to_string());
    }
    rc as i32
}

pub fn check(code: i32) {
    if code != RC::Ok as i32 {
        let error = match code {
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
        eprintln!("{}", error_message(error));
        std::process::exit(1);
    }
}

pub fn print_error(error: RC) {
    print!("{}", error_message(error));
}

pub fn error_message(error: RC) -> String {
    match rc_message().lock() {
        Ok(slot) => match slot.as_ref() {
            Some(message) => format!("EC ({}), \"{}\"\n", error as i32, message),
            None => format!("EC ({})\n", error as i32),
        },
        Err(_) => format!("EC ({})\n", error as i32),
    }
}

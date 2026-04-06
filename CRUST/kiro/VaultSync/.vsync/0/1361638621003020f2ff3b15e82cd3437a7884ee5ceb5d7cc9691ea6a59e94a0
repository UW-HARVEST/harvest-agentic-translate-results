use crate::logger::{logger, LOGGING_TAG};
use crate::vm::{
    self, add_changes, commit as vm_commit, init_repo, load_author, load_repository, make_commit,
    rollback, Author, Commit, Repository,
};
use std::path::Path;

pub const ARG_INIT: &str = "init";
pub const ARG_HELP: &str = "--help";
pub const ARG_HELP_SC: &str = "-h";
pub const ARG_ADD_CHANGES: &str = "add";
pub const ARG_COMMIT: &str = "commit";
pub const ARG_ROLLBACK: &str = "rollback";

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let argc = args.len() as i32;
    if argc < 2 {
        invalid_args();
        return;
    } else if argc < 3 {
        if args[1] == ARG_INIT {
            init_repository();
        } else if args[1] == ARG_COMMIT {
            commit_changes();
        } else if args[1] == ARG_HELP || args[1] == ARG_HELP_SC {
            getting_help();
        } else {
            invalid_args();
        }
        return;
    } else if argc < 4 && args[1] == ARG_ROLLBACK {
        rollback_changes(&args[2]);
        return;
    }

    if args[1] == ARG_ADD_CHANGES {
        let strs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        track_changes(argc, &strs);
        return;
    }

    invalid_args();
}

pub fn getting_help() {
    println!("Usage: vaultsync [OPTIONS] COMMAND\n");
    println!("Options:");
    println!("  -h, --help     Show help message and exit\n");
    println!("Commands:");
    println!("  init           Initialize a repository");
    println!("  commit         Make a commit");
    println!("  add [files]    Add files to be tracked in the next commit\n");
    println!("For more information, see 'man vaultsync'");
}

pub fn init_repository() {
    let author = match std::panic::catch_unwind(|| load_author()) {
        Ok(a) => a,
        Err(_) => {
            logger(LOGGING_TAG::ERROR_TAG, "Can not getting the author, check the default config at ~/.vsync_rc");
            return;
        }
    };

    match init_repo(&author) {
        Ok(_) => logger(LOGGING_TAG::INFO_TAG, "Initialization done"),
        Err(_) => {}
    }
}

pub fn track_changes(n: i32, files: &Vec<&str>) {
    let paths: Vec<&Path> = files[2..].iter().map(|s| Path::new(*s)).collect();

    let repo = load_repository();
    match add_changes(&repo, &paths) {
        Ok(_) => logger(LOGGING_TAG::INFO_TAG, "Files has been tracked successfully"),
        Err(_) => {}
    }
}

pub fn commit_changes() {
    let author = match std::panic::catch_unwind(|| load_author()) {
        Ok(a) => a,
        Err(_) => {
            logger(LOGGING_TAG::ERROR_TAG, "Can not getting the author, check the default config at ~/.vsync_rc");
            return;
        }
    };

    let repo = load_repository();
    let commit = Commit {
        hash: String::new(),
        author: author.clone(),
        parent_hash: String::new(),
    };

    match make_commit(&repo, &author, &commit) {
        Ok(_) => logger(LOGGING_TAG::INFO_TAG, "The commit has been done successfully"),
        Err(_) => {}
    }
}

pub fn rollback_changes(hash: &str) {
    let repo = load_repository();
    match rollback(&repo, hash) {
        Ok(_) => logger(LOGGING_TAG::INFO_TAG, "The rollback has been successfully done"),
        Err(_) => {}
    }
}

fn invalid_args() {
    logger(LOGGING_TAG::INFO_TAG, "Invalid arguments, check --help");
}

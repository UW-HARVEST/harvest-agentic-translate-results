use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use std::fs;
use std::io::{BufRead, BufReader, Write, Read};

use crate::logger::{logger, LOGGING_TAG};

pub const MAX_UNAME: usize = 50;
pub const MAX_MAIL: usize = 100;
pub const CONFIG_FILE: &str = "/config";
pub const REPOSITORY_FILE: &str = "/repository";
pub const LOG_FILE: &str = "/log";
pub const MAX_REPOSITORY_FILE_INIT: usize = 1024;
pub const MAX_CWD: usize = 4096;
pub const MAX_REP_NAME: usize = 80;

#[derive(Clone)]
pub struct RepoValue {
    pub raw_path: String,
    pub hash: String,
}

#[derive(Clone)]
pub struct Author {
    pub username: String,
    pub mail: String,
}

pub const TRACKED_DIR: &str = "tracked";
pub const HASH_LEN: usize = 2 * 32 + 1;

#[derive(Clone)]
pub struct Commit {
    pub hash: String,
    pub author: Author,
    pub parent_hash: String,
}

#[derive(Clone)]
pub struct Repository {
    pub name: String,
    pub author: Author,
    pub dir: String,
    pub last_commit: Option<Commit>,
}

fn create_hash_string() -> String {
    use sha2::{Sha256, Digest};
    use rand::Rng;

    let charset = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut rng = rand::thread_rng();
    let mut random_data = [0u8; 16];
    for b in random_data.iter_mut() {
        *b = charset[rng.gen::<u8>() as usize % charset.len()];
    }
    let hash_bytes = Sha256::digest(&random_data);
    hash_bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn copy_file(src: &str, dst: &str) -> Result<(), String> {
    let data = fs::read(src).map_err(|e| e.to_string())?;
    fs::write(dst, data).map_err(|e| e.to_string())
}

fn create_directories(path: &str) -> Result<(), String> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn populate_hashmap_from_file(
    map: &mut HashMap<String, RepoValue>,
    raw_path: &str,
    filename: &str,
) -> Result<(), String> {
    let file = fs::File::open(filename)
        .map_err(|_| "Can not populate a hashmap from such as this file".to_string())?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    // skip first two lines
    lines.next();
    lines.next();
    for line in lines {
        let line = line.map_err(|e| e.to_string())?;
        if let Some(space_pos) = line.find(' ') {
            let path = &line[..space_pos];
            let hash = &line[space_pos + 1..];
            let src_file_path = format!("{}/{}", raw_path, hash);
            map.insert(
                path.to_string(),
                RepoValue {
                    raw_path: src_file_path,
                    hash: hash.to_string(),
                },
            );
        }
    }
    Ok(())
}

pub fn init_repo(author: &Author) -> Result<Repository, String> {
    let dir = std::env::current_dir()
        .map_err(|_| "[init_repo] Can not get the current working directory".to_string())?
        .to_string_lossy()
        .to_string();

    let first_commit = Commit {
        hash: "0".to_string(),
        parent_hash: "-".to_string(),
        author: author.clone(),
    };

    let repo = Repository {
        name: "foo".to_string(),
        author: author.clone(),
        dir: dir.clone(),
        last_commit: Some(first_commit.clone()),
    };

    let dotdir_path = format!("{}/.vsync", dir);
    fs::create_dir(&dotdir_path)
        .map_err(|_| "[init_repo] Can not create .vsync dir".to_string())?;

    write_repository_file(&repo)
        .map_err(|_| "[init_repo] Can not create repository file".to_string())?;

    let mut map = HashMap::new();
    make_init_map_repo(&repo, &mut map, &dir)
        .map_err(|_| "[init_repo] can not create the hashmap for the first commit".to_string())?;

    create_commit(&repo, &first_commit, &map)
        .map_err(|_| "[init_repo] can not create the first commit".to_string())?;

    Ok(repo)
}

pub fn write_repository_file(repo: &Repository) -> Result<(), String> {
    let repo_file_path = format!("{}/.vsync/repository", repo.dir);
    let mut file = fs::File::create(&repo_file_path)
        .map_err(|_| "[write_repository_file] Can not open the repository file".to_string())?;
    let commit = repo.last_commit.as_ref().unwrap();
    write!(
        file,
        "{}\n{}\n{}\n{} {}",
        repo.dir, commit.hash, repo.name, repo.author.username, repo.author.mail
    )
    .map_err(|e| e.to_string())
}

pub fn load_repository() -> Repository {
    let dir = std::env::current_dir()
        .expect("Can not get cwd")
        .to_string_lossy()
        .to_string();

    let repo_file_path = format!("{}/.vsync/repository", dir);
    let content = fs::read_to_string(&repo_file_path)
        .expect("[load_repository] Can not load the repository file");
    let lines: Vec<&str> = content.lines().collect();

    // line 0: dir, line 1: last commit hash, line 2: name, line 3: author
    let last_commit_hash = lines[1];

    let last_commit_path = format!("{}/.vsync/{}/commit", dir, last_commit_hash);
    let commit_content = fs::read_to_string(&last_commit_path)
        .expect("[load_repository] Can not load the last commit");
    let commit_lines: Vec<&str> = commit_content.lines().collect();

    let parent_hash = commit_lines[0];
    let author_line = commit_lines[1];
    let space = author_line.find(' ').expect("[load_repository] Can not load the author of last commit");
    let commit_author = Author {
        username: author_line[..space].to_string(),
        mail: author_line[space + 1..].to_string(),
    };

    let last_commit = Commit {
        hash: last_commit_hash.to_string(),
        author: commit_author,
        parent_hash: parent_hash.to_string(),
    };

    let repo_name = lines[2];
    let repo_author_line = lines[3];
    let space2 = repo_author_line.find(' ').expect("[load_repository] Can not load the author of repository");
    let repo_author = Author {
        username: repo_author_line[..space2].to_string(),
        mail: repo_author_line[space2 + 1..].to_string(),
    };

    Repository {
        name: repo_name.to_string(),
        author: repo_author,
        dir,
        last_commit: Some(last_commit),
    }
}

pub fn load_author() -> Author {
    let config_path = std::env::var("VSYNC_CONFIG_PATH")
        .expect("[load_author] The global config file does not existed");
    let content = fs::read_to_string(&config_path)
        .expect("[load_author] Can not read config file");
    let line = content.lines().next().expect("[load_author] Empty config");
    let space = line.find(' ').expect("[load_author] Invalid config format");
    Author {
        username: line[..space].to_string(),
        mail: line[space + 1..].trim_end_matches('\n').to_string(),
    }
}

// commit.h
pub fn make_init_map_repo(
    repo: &Repository,
    map: &mut HashMap<String, RepoValue>,
    path: &str,
) -> Result<(), String> {
    let entries = fs::read_dir(path)
        .map_err(|_| "[make_init_map_repo] Can not open the init commit dir".to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "." || name == ".." || name == ".vsync" {
            continue;
        }
        let fullpath = format!("{}/{}", path, name);
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        if ft.is_dir() {
            make_init_map_repo(repo, map, &fullpath)?;
        } else {
            let hash_string = create_hash_string();
            map.insert(
                fullpath.clone(),
                RepoValue {
                    raw_path: fullpath,
                    hash: hash_string,
                },
            );
        }
    }
    Ok(())
}

pub fn add_changes(repo: &Repository, files: &Vec<&Path>) -> Result<(), String> {
    let vsync_path = format!("{}/.vsync", repo.dir);
    if !Path::new(&vsync_path).is_dir() {
        logger(LOGGING_TAG::ERROR_TAG, "[add_changes] Can't find .vsync");
        return Err("[add_changes] Can't find .vsync".to_string());
    }

    let track_dir_path = format!("{}/.vsync/tracked", repo.dir);
    if !Path::new(&track_dir_path).is_dir() {
        fs::create_dir(&track_dir_path)
            .map_err(|_| "Can not create a tracking dir".to_string())?;
    }

    let mut map: HashMap<String, RepoValue> = HashMap::new();
    let track_file_path = format!("{}/track", track_dir_path);
    if Path::new(&track_file_path).exists() {
        populate_hashmap_from_file(&mut map, &track_dir_path, &track_file_path)?;
    }

    for file_path in files {
        let path_str = file_path.to_string_lossy().to_string();
        if !file_path.exists() {
            logger(LOGGING_TAG::ERROR_TAG, "Can not adding changes that not existed... check the added files");
            return Err("Can not adding changes that not existed".to_string());
        }

        // remove old tracked version if exists
        if let Some(old_val) = map.remove(&path_str) {
            let old_file = format!("{}/{}", track_dir_path, old_val.hash);
            fs::remove_file(&old_file)
                .map_err(|_| "[add_changes] Can not adding changes".to_string())?;
        }

        let hash_string = create_hash_string();
        map.insert(
            path_str.clone(),
            RepoValue {
                raw_path: path_str,
                hash: hash_string,
            },
        );
    }

    make_changes(repo, &mut map)
        .map_err(|_| "[add_changes] Can not persist changes".to_string())?;

    Ok(())
}

pub fn make_changes(repo: &Repository, map: &mut HashMap<String, RepoValue>) -> Result<(), String> {
    let track_dir_path = format!("{}/.vsync/tracked", repo.dir);
    let track_file_path = format!("{}/track", track_dir_path);

    let mut track_file = fs::File::create(&track_file_path)
        .map_err(|_| "[make_changes] Can not open track file".to_string())?;

    writeln!(track_file, "-").map_err(|e| e.to_string())?;
    writeln!(track_file, "-").map_err(|e| e.to_string())?;

    for (path, value) in map.iter() {
        let destination_path = format!("{}/{}", track_dir_path, value.hash);
        if value.raw_path != destination_path {
            copy_file(&value.raw_path, &destination_path)
                .map_err(|_| "[make_changes] Can not copy file".to_string())?;
        }
        writeln!(track_file, "{} {}", path, value.hash).map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn create_commit(
    repo: &Repository,
    commit: &Commit,
    map: &HashMap<String, RepoValue>,
) -> Result<(), String> {
    let commit_path = format!("{}/.vsync/{}", repo.dir, commit.hash);
    fs::create_dir(&commit_path)
        .map_err(|_| "[create_commit] Can not create the commit dir".to_string())?;

    let commit_file_path = format!("{}/commit", commit_path);
    let mut commit_file = fs::File::create(&commit_file_path)
        .map_err(|_| "[create_commit] Can not create the commit file".to_string())?;

    writeln!(commit_file, "{}", commit.parent_hash).map_err(|e| e.to_string())?;
    writeln!(commit_file, "{} {}", commit.author.username, commit.author.mail)
        .map_err(|e| e.to_string())?;

    for (path, value) in map.iter() {
        let destination_path = format!("{}/{}", commit_path, value.hash);
        copy_file(&value.raw_path, &destination_path)
            .map_err(|_| "[create_commit] can not copy file".to_string())?;
        writeln!(commit_file, "{} {}", path, value.hash).map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn make_commit(repo: &Repository, author: &Author, commit: &Commit) -> Result<(), String> {
    let mut new_commit = Commit {
        hash: create_hash_string(),
        parent_hash: repo.last_commit.as_ref().unwrap().hash.clone(),
        author: author.clone(),
    };

    // load changes
    let mut changes_map: HashMap<String, RepoValue> = HashMap::new();
    let track_dir_path = format!("{}/.vsync/tracked", repo.dir);
    let track_file_path = format!("{}/track", track_dir_path);
    if Path::new(&track_file_path).exists() {
        populate_hashmap_from_file(&mut changes_map, &track_dir_path, &track_file_path)?;
    }

    // load last commit
    let last_commit_hash = &repo.last_commit.as_ref().unwrap().hash;
    let last_commit_dir = format!("{}/.vsync/{}", repo.dir, last_commit_hash);
    let last_commit_file = format!("{}/commit", last_commit_dir);

    let mut last_commit_map: HashMap<String, RepoValue> = HashMap::new();
    populate_hashmap_from_file(&mut last_commit_map, &last_commit_dir, &last_commit_file)?;

    // merge changes into last commit map
    for (path, value) in changes_map.iter() {
        last_commit_map.insert(path.clone(), value.clone());
    }

    create_commit(repo, &new_commit, &last_commit_map)?;

    // update repo
    let mut updated_repo = repo.clone();
    updated_repo.last_commit = Some(new_commit);

    write_repository_file(&updated_repo)?;
    delete_tracked_dir(&updated_repo)?;

    Ok(())
}

pub fn delete_tracked_dir(repo: &Repository) -> Result<(), String> {
    let tracked_dir_path = format!("{}/.vsync/tracked", repo.dir);
    if !Path::new(&tracked_dir_path).is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(&tracked_dir_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        fs::remove_file(entry.path()).map_err(|_| "[delete_tracked_dir]".to_string())?;
    }

    fs::remove_dir(&tracked_dir_path).map_err(|_| "[delete_tracked_dir]".to_string())
}

pub fn rollback(repo: &Repository, commit_hash: &str) -> Result<(), String> {
    let commit_dir_path = format!("{}/.vsync/{}", repo.dir, commit_hash);
    if !Path::new(&commit_dir_path).is_dir() {
        logger(LOGGING_TAG::ERROR_TAG, "[rollback] Commit not found");
        return Err("[rollback] Commit not found".to_string());
    }

    let commit_file_path = format!("{}/commit", commit_dir_path);
    if !Path::new(&commit_file_path).exists() {
        return Err("[rollback] Can not found the commit file".to_string());
    }

    let mut commit_map: HashMap<String, RepoValue> = HashMap::new();
    populate_hashmap_from_file(&mut commit_map, &commit_dir_path, &commit_file_path)?;

    reset_repo_dir(&repo.dir, &repo.dir)?;

    make_rollback_commit(&mut commit_map)?;

    let commit = load_commit(repo, commit_hash)?;

    let mut updated_repo = repo.clone();
    updated_repo.last_commit = Some(commit);
    write_repository_file(&updated_repo)?;

    Ok(())
}

pub fn reset_repo_dir(path: &str, root_path: &str) -> Result<(), String> {
    let entries = fs::read_dir(path).map_err(|_| "Can not open dir".to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "." || name == ".." || name == ".vsync" {
            continue;
        }
        let full_path = format!("{}/{}", path, name);
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        if ft.is_dir() {
            reset_repo_dir(&full_path, root_path)?;
        } else {
            fs::remove_file(&full_path).map_err(|e| e.to_string())?;
        }
    }

    if path != root_path {
        fs::remove_dir(path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn make_rollback_commit(map: &mut HashMap<String, RepoValue>) -> Result<(), String> {
    for (path, value) in map.iter() {
        create_directories(path)?;
        copy_file(&value.raw_path, path)
            .map_err(|_| "[make_rollback_commit] can not copy file".to_string())?;
    }
    Ok(())
}

pub fn load_commit(repo: &Repository, commit_hash: &str) -> Result<Commit, String> {
    let commit_file_path = format!("{}/.vsync/{}/commit", repo.dir, commit_hash);
    let content = fs::read_to_string(&commit_file_path)
        .map_err(|_| "[load_commit] Commit not found".to_string())?;
    let lines: Vec<&str> = content.lines().collect();

    let parent_hash = lines[0];
    let author_line = lines[1];
    let space = author_line.find(' ')
        .ok_or("[load_commit] Author of commit not found".to_string())?;

    Ok(Commit {
        hash: commit_hash.to_string(),
        author: Author {
            username: author_line[..space].to_string(),
            mail: author_line[space + 1..].to_string(),
        },
        parent_hash: parent_hash.to_string(),
    })
}

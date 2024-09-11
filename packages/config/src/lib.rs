#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]

use std::path::PathBuf;

#[cfg(feature = "db")]
pub mod db;

pub fn get_config_dir_path() -> Option<PathBuf> {
    home::home_dir().map(|home| home.join(".local").join("moosicbox"))
}

pub fn get_profiles_dir_path() -> Option<PathBuf> {
    get_config_dir_path().map(|x| x.join("profiles"))
}

pub fn get_profile_dir_path(profile: &str) -> Option<PathBuf> {
    get_profiles_dir_path().map(|x| x.join(profile))
}

pub fn get_profile_db_dir_path(profile: &str) -> Option<PathBuf> {
    get_profile_dir_path(profile).map(|x| x.join("db"))
}

pub fn get_cache_dir_path() -> Option<PathBuf> {
    get_config_dir_path().map(|config| config.join("cache"))
}

pub fn make_config_dir_path() -> Option<PathBuf> {
    if let Some(path) = get_config_dir_path() {
        if path.is_dir() || std::fs::create_dir_all(&path).is_ok() {
            return Some(path);
        }
    }

    None
}

pub fn make_profile_dir_path(profile: &str) -> Option<PathBuf> {
    if let Some(path) = get_profile_dir_path(profile) {
        if path.is_dir() || std::fs::create_dir_all(&path).is_ok() {
            return Some(path);
        }
    }

    None
}

pub fn make_profile_db_dir_path(profile: &str) -> Option<PathBuf> {
    if let Some(path) = get_profile_db_dir_path(profile) {
        if path.is_dir() || std::fs::create_dir_all(&path).is_ok() {
            return Some(path);
        }
    }

    None
}

pub fn make_cache_dir_path() -> Option<PathBuf> {
    if let Some(path) = get_cache_dir_path() {
        if path.is_dir() || std::fs::create_dir_all(&path).is_ok() {
            return Some(path);
        }
    }

    None
}

#[cfg(feature = "test")]
pub fn get_tests_dir_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "moosicbox_tests_{}",
        rand::Rng::gen::<usize>(&mut rand::thread_rng())
    ))
}

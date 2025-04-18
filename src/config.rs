// Copyright (C) 2025  Tassilo Horn <tsdh@gnu.org>
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for
// more details.
//
// You should have received a copy of the GNU General Public License along with
// this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    fs::{DirBuilder, OpenOptions},
    io::{Read, Write},
    path::Path,
    sync::{LazyLock, RwLock},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

static CONFIG: LazyLock<RwLock<Config>> =
    LazyLock::new(|| RwLock::new(load_config()));

pub fn with_config<F, R>(f: F) -> R
where
    F: FnOnce(&Config) -> R,
{
    f(&CONFIG.read().unwrap())
}

pub fn get_config_file_path() -> Box<Path> {
    let proj_dirs = ProjectDirs::from("", "", "nirius").expect("");
    let user_config_dir = proj_dirs.config_dir();
    if !user_config_dir.exists() {
        let sys_path = "/etc/xdg/nirius/config.toml".to_string();
        let sys_config_file = Path::new(sys_path.as_str());
        if sys_config_file.exists() {
            return sys_config_file.into();
        }
        DirBuilder::new()
            .recursive(true)
            .create(user_config_dir)
            .unwrap();
    }
    user_config_dir.join("config.toml").into_boxed_path()
}

pub fn save_config(cfg: Config) {
    let path = get_config_file_path();
    let content = toml::to_string_pretty::<Config>(&cfg)
        .expect("Cannot serialize config.");
    let mut file = OpenOptions::new()
        .read(false)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

pub fn load_config() -> Config {
    let path = get_config_file_path();
    if !path.exists() {
        save_config(Config::default());
        log::debug!("Created new config in {}.", path.to_string_lossy());
    }

    load_config_file(&path)
}

pub fn load_config_file(config_file: &Path) -> Config {
    if !config_file.exists() {
        panic!(
            "Config file {} does not exist.",
            config_file.to_string_lossy()
        );
    } else {
        log::debug!("Loading config from {}.", config_file.to_string_lossy());
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(config_file)
        .unwrap();
    let mut buf: String = String::new();
    file.read_to_string(&mut buf).unwrap();
    match toml::from_str::<Config>(&buf) {
        Ok(cfg) => cfg,
        Err(err) => {
            log::error!("Invalid config: {err}");
            log::error!("Using default configuration.");
            Config::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Config {}

impl Default for Config {
    fn default() -> Self {
        Config {}
    }
}

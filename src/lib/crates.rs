use std::collections::HashMap;
use anyhow::Result;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use either::Either;
use itertools::Itertools;
use regex::Regex;
use serde::Deserialize;

pub struct CratesIndex {
    path: PathBuf,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CrateMeta {
    pub name: String,
    pub path: PathBuf,
}

impl CrateMeta {
    pub fn detail(&self) -> Result<Vec<Crate>> {
        let lines = fs::read_to_string(&self.path)?;
        Ok(lines
            .trim()
            .lines()
            .map(|line| serde_json::from_str(line))
            .try_collect()?)
    }
}

#[derive(Debug, Deserialize)]
pub struct Crate {
    pub name: String,
    #[serde(rename = "vers")]
    pub version: String,
    pub features: HashMap<String, Vec<String>>,
    pub yanked: bool,
}

impl Default for CratesIndex {
    fn default() -> Self {
        Self {
            path: home::cargo_home()
                .unwrap()
                .join("registry")
                .join("index")
                .read_dir()
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path(),
        }
    }
}

impl CratesIndex {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn crates_with_prefix(&self, prefix: &str) -> io::Result<Vec<CrateMeta>> {
        _crates_with_prefix(&self.path, &regexify(prefix), prefix)
    }
    pub fn crate_(&self, name: &str) -> io::Result<Option<CrateMeta>> {
        _crate_exact(&self.path, name, name)
    }
}

fn regexify(partial_name: &str) -> Regex {
    Regex::new(
        format!(
            "^{}",
            partial_name.replace("-", "?[-_]").replace("_", "?[-_]")
        )
        .as_str(),
    )
    .unwrap()
}

fn expand_path_domain(part: &str) -> impl Iterator<Item = String> {
    part.chars()
        .map(|char| {
            if char == '_' || char == '-' {
                Either::Left(['_', '-'].into_iter())
            } else {
                Either::Right([char].into_iter())
            }
        })
        .multi_cartesian_product()
        .map(|chars| chars.into_iter().join(""))
}

pub fn _crate_exact(
    path: &Path,
    prefix: &str,
    remaining_prefix: &str,
) -> io::Result<Option<CrateMeta>> {
    if !path.is_dir() {
        return Ok(None);
    }

    let crate_ = path
        .read_dir()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_str().unwrap() == prefix)
        .map(|entry| CrateMeta {
            name: entry.file_name().into_string().unwrap(),
            path: entry.path(),
        })
        .next();

    if let Some(crate_) = crate_ {
        Ok(Some(crate_))
    } else {
        match remaining_prefix.len() {
            0 => Ok(None),
            1 => _crate_exact(path, prefix, ""),
            _ => {
                let (h1, t1) = remaining_prefix.split_at(1);
                if let Some(crate_) = _crate_exact(&path.join(h1), prefix, t1)? {
                    return Ok(Some(crate_));
                }

                let (h2, t2) = remaining_prefix.split_at(2);
                _crate_exact(&path.join(h2), prefix, t2)
            }
        }
    }
}

pub fn _crates_with_prefix(
    path: &Path,
    matcher: &Regex,
    remaining_prefix: &str,
) -> io::Result<Vec<CrateMeta>> {
    if !path.is_dir() {
        return Ok(vec![]);
    }

    let path_iter = path
        .read_dir()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false));
    let mut crates = if remaining_prefix.is_empty() {
        Either::Left(path_iter)
    } else {
        Either::Right(
            path_iter.filter(|entry| matcher.is_match(entry.file_name().to_str().unwrap())),
        )
    }
    .map(|entry| CrateMeta {
        name: entry.file_name().into_string().unwrap(),
        path: entry.path(),
    })
    .collect_vec();

    match remaining_prefix.len() {
        0 => {
            for path in path
                .read_dir()?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            {
                crates.extend(_crates_with_prefix(&path.path(), matcher, "")?);
            }
            Ok(crates)
        }
        1 => {
            let parts = expand_path_domain(remaining_prefix).collect_vec();
            for subpath in path
                .read_dir()?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            {
                let subpath = subpath.path();
                for part in &parts {
                    if subpath
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with(part)
                    {
                        crates.extend(_crates_with_prefix(&subpath, matcher, "")?);
                    }
                }
            }
            Ok(crates)
        }
        _ => {
            let (h1, t1) = remaining_prefix.split_at(1);
            let (h2, t2) = remaining_prefix.split_at(2);
            for path in expand_path_domain(h1).map(|h1| path.join(h1)) {
                crates.extend(_crates_with_prefix(&path, matcher, t1)?);
            }
            for path in expand_path_domain(h2).map(|h2| path.join(h2)) {
                crates.extend(_crates_with_prefix(&path, matcher, t2)?);
            }
            Ok(crates)
        }
    }
}

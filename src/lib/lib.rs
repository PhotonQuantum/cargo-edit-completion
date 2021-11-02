use anyhow::{anyhow, bail, Result};
use either::Either;
use itertools::Itertools;
use semver::{Comparator, Prerelease, Version};

use crate::crates::CratesIndex;

pub mod crates;

fn satisfied_versions(index: &CratesIndex, crate_name: &str, req: &str) -> Result<Vec<Version>> {
    let crate_ = index
        .crate_(crate_name)?
        .ok_or_else(|| anyhow!("unable to find crate"))?;
    let versions = crate_.detail()?;
    Ok(versions
        .iter()
        .filter(|version| version.version.starts_with(req) && !version.yanked)
        .map(|version| Version::parse(version.version.as_str()).unwrap()) // TODO error handling
        .rev()
        .collect())
}

type Field = Either<u64, Prerelease>;

#[derive(Debug, Copy, Clone)]
enum FieldType {
    Major,
    Minor,
    Patch,
    Pre,
}

fn first_unfilled_field(req: Option<&Comparator>) -> Option<FieldType> {
    if let Some(req) = req {
        if req.minor.is_none() {
            return Some(FieldType::Minor);
        }
        if req.patch.is_none() {
            return Some(FieldType::Patch);
        }
        if req.pre.is_empty() {
            return Some(FieldType::Pre);
        }
        None
    } else {
        Some(FieldType::Major)
    }
}

fn extract_field(version: &Version, field: FieldType) -> Field {
    match field {
        FieldType::Major => Field::Left(version.major),
        FieldType::Minor => Field::Left(version.minor),
        FieldType::Patch => Field::Left(version.patch),
        FieldType::Pre => Field::Right(version.pre.clone()),
    }
}

fn query_prefix(req: &Comparator) -> String {
    let mut output = req.major.to_string();
    if let Some(minor) = req.minor {
        output.push('.');
        output.push_str(&*minor.to_string());
    }
    if let Some(patch) = req.patch {
        output.push('.');
        output.push_str(&*patch.to_string());
    }
    if !req.pre.is_empty() {
        output.push('-');
        output.push_str(req.pre.as_str());
    }
    output
}

fn complete_version(
    index: &CratesIndex,
    crate_name: &str,
    partial_ver: &str,
) -> Result<Vec<String>> {
    let query_prefix = partial_ver.trim().trim_start_matches(&['>', '<', '=', '~', '^'][..]).trim_start_matches('=');

    let versions = satisfied_versions(index, crate_name, &query_prefix)?;

    Ok(versions.into_iter().filter_map(|version|version.to_string().strip_prefix(&partial_ver).map(|s|s.to_string())).collect())
}

fn complete_crate_name(index: &CratesIndex, partial_name: &str) -> Result<Vec<String>> {
    Ok(index
        .crates_with_prefix(partial_name)?
        .into_iter()
        .map(|crate_| crate_.name)
        .collect())
}

pub fn complete_crate(index: &CratesIndex, partial_command: &str) -> Result<Vec<String>> {
    if let Some((name, vers)) = partial_command.split_once("@") {
        let last_ver = vers.rsplit(',').next().unwrap_or_default();
        Ok(complete_version(index, name, last_ver)?
            .into_iter()
            .map(|part| format!("{}{}", partial_command, part))
            .collect())
    } else {
        Ok(complete_crate_name(index, partial_command)?)
    }
}

pub fn complete_feature(
    index: &CratesIndex,
    crate_name: &str,
    version: &str,
) -> Result<Vec<String>> {
    let crate_ = index
        .crate_(crate_name)?
        .ok_or_else(|| anyhow!("missing crate"))?;
    Ok(crate_
        .detail()?
        .into_iter()
        .filter(|ver| ver.version.starts_with(version))
        .last()
        .ok_or_else(|| anyhow!("missing version"))?
        .features
        .keys()
        .cloned()
        .collect())
}

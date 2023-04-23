use std::borrow::Cow;
use std::env::consts::{ARCH, OS};
use std::fmt;
use std::str::FromStr;

use anyhow::{anyhow, Error};
use serde::{de, Deserialize, Serialize};

mod indygreg_python {
    use super::PythonVersion;
    include!("downloads.inc");
}

const DEFAULT_KIND: &str = "cpython";

/// Internal descriptor for a python version.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone)]
pub struct PythonVersion {
    pub kind: &'static str,
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl PythonVersion {
    /// Returns the latest version for this OS.
    pub fn latest_cpython() -> PythonVersion {
        get_download_url("3", OS, ARCH)
            .expect("unsupported platform")
            .0
    }
}

impl Serialize for PythonVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PythonVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = Cow::<'_, str>::deserialize(deserializer)?;
        PythonVersion::from_str(&s).map_err(|err| de::Error::custom(err.to_string()))
    }
}

impl FromStr for PythonVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_version(s)
            .map(|(kind, major, minor, patch)| PythonVersion {
                kind: if kind == "cpython" {
                    "cpython"
                } else {
                    "unknown"
                },
                major,
                minor: minor.unwrap_or(0),
                patch: patch.unwrap_or(0),
            })
            .ok_or_else(|| anyhow!("invalid version"))
    }
}

impl fmt::Display for PythonVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}@{}.{}.{}",
            self.kind, self.major, self.minor, self.patch
        )
    }
}

fn parse_version(version: &str) -> Option<(&str, u8, Option<u8>, Option<u8>)> {
    let (kind, version) = match version.split_once('@') {
        Some(rv) => rv,
        None => (DEFAULT_KIND, version),
    };
    let mut iter = version.split('.').flat_map(|x| x.parse().ok());
    let a = iter.next()?;
    Some((kind, a, iter.next(), iter.next()))
}

fn matches_version(ref_version: (&str, u8, Option<u8>, Option<u8>), v: PythonVersion) -> bool {
    match ref_version {
        (kind, major, Some(minor), Some(patch)) => {
            (v.kind, v.major, v.minor, v.patch) == (kind, major, minor, patch)
        }
        (kind, major, Some(minor), None) => v.kind == kind && v.major == major && v.minor == minor,
        (kind, major, None, None | Some(_)) => v.kind == kind && v.major == major,
    }
}

/// Given a version, platform and architecture returns the download URL.
pub fn get_download_url(
    version: &str,
    platform: &str,
    arch: &str,
) -> Option<(PythonVersion, &'static str)> {
    let parsed_version = parse_version(version)?;
    for (it_version, it_arch, it_platform, it_url) in indygreg_python::CPYTHON_VERSIONS {
        if platform == *it_platform
            && arch == *it_arch
            && matches_version(parsed_version, *it_version)
        {
            return Some((*it_version, it_url));
        }
    }
    None
}

#[test]
fn test_get_download_url() {
    let url = get_download_url("3.8.14", "macos", "aarch64");
    assert_eq!(url, Some((PythonVersion { kind: "cpython", major: 3, minor: 8, patch: 14 }, "https://github.com/indygreg/python-build-standalone/releases/download/20221002/cpython-3.8.14%2B20221002-aarch64-apple-darwin-debug-full.tar.zst")));
    let url = get_download_url("3.8", "macos", "aarch64");
    assert_eq!(url, Some((PythonVersion { kind: "cpython", major: 3, minor: 8, patch: 16 }, "https://github.com/indygreg/python-build-standalone/releases/download/20221220/cpython-3.8.16%2B20221220-aarch64-apple-darwin-debug-full.tar.zst")));
    let url = get_download_url("3", "macos", "aarch64");
    assert_eq!(url, Some((PythonVersion { kind: "cpython", major: 3, minor: 11, patch: 1}, "https://github.com/indygreg/python-build-standalone/releases/download/20230116/cpython-3.11.1%2B20230116-aarch64-apple-darwin-debug-full.tar.zst")));
}

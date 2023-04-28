use std::borrow::Cow;
use std::env::consts::{ARCH, OS};
use std::fmt;
use std::str::FromStr;

use anyhow::{anyhow, Error};
use pep440_rs::Version;
use serde::{de, Deserialize, Serialize};

mod indygreg_python {
    use super::PythonVersion;
    include!("downloads.inc");
}

const DEFAULT_KIND: &str = "cpython";

/// Internal descriptor for a python version.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct PythonVersion {
    pub kind: Cow<'static, str>,
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub suffix: Option<Cow<'static, str>>,
}

impl PythonVersion {
    /// Returns the latest version for this OS.
    pub fn latest_cpython() -> PythonVersion {
        get_download_url(
            &PythonVersionRequest {
                kind: None,
                major: 3,
                minor: None,
                patch: None,
                suffix: None,
            },
            OS,
            ARCH,
        )
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
        let req: PythonVersionRequest = s.parse()?;
        Ok(PythonVersion {
            kind: match req.kind {
                None => Cow::Borrowed(DEFAULT_KIND),
                Some(other) => other,
            },
            major: req.major,
            minor: req.minor.unwrap_or(0),
            patch: req.patch.unwrap_or(0),
            suffix: req.suffix,
        })
    }
}

impl TryFrom<PythonVersionRequest> for PythonVersion {
    type Error = Error;

    fn try_from(req: PythonVersionRequest) -> Result<Self, Self::Error> {
        Ok(PythonVersion {
            kind: match req.kind {
                None => Cow::Borrowed(DEFAULT_KIND),
                Some(other) => other,
            },
            major: req.major,
            minor: req.minor.ok_or_else(|| anyhow!("missing minor version"))?,
            patch: req.patch.ok_or_else(|| anyhow!("missing patch version"))?,
            suffix: req.suffix,
        })
    }
}

impl fmt::Display for PythonVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}@{}.{}.{}",
            self.kind, self.major, self.minor, self.patch
        )?;
        if let Some(ref suffix) = self.suffix {
            write!(f, ".{}", suffix)?;
        }
        Ok(())
    }
}

/// Internal descriptor for a python version request.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct PythonVersionRequest {
    pub kind: Option<Cow<'static, str>>,
    pub major: u8,
    pub minor: Option<u8>,
    pub patch: Option<u8>,
    pub suffix: Option<Cow<'static, str>>,
}

impl PythonVersionRequest {
    /// Returns a simplified format of the version request.
    pub fn format_simple(&self) -> String {
        use std::fmt::Write;
        let mut rv = format!("{}", self.major);
        if let Some(minor) = self.minor {
            write!(rv, ".{}", minor).unwrap();
            if let Some(patch) = self.patch {
                write!(rv, ".{}", patch).unwrap();
            }
        }
        rv
    }
}

impl From<PythonVersion> for PythonVersionRequest {
    fn from(value: PythonVersion) -> Self {
        PythonVersionRequest {
            kind: Some(value.kind),
            major: value.major,
            minor: Some(value.minor),
            patch: Some(value.patch),
            suffix: value.suffix,
        }
    }
}

impl From<Version> for PythonVersionRequest {
    fn from(value: Version) -> Self {
        PythonVersionRequest {
            kind: None,
            major: value.release.first().map(|x| *x as _).unwrap_or(3),
            minor: value.release.get(1).map(|x| *x as _),
            patch: value.release.get(2).map(|x| *x as _),
            suffix: None,
        }
    }
}

impl FromStr for PythonVersionRequest {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (kind, version) = match s.split_once('@') {
            Some((kind, version)) => (Some(kind), version),
            None => (None, s),
        };
        let mut iter = version.split('.');
        let major = iter
            .next()
            .and_then(|x| x.parse::<u8>().ok())
            .ok_or_else(|| anyhow!("invalid version"))?;
        let minor = iter.next().and_then(|x| x.parse::<u8>().ok());
        let patch = iter.next().and_then(|x| x.parse::<u8>().ok());
        let suffix = iter.next().map(|x| Cow::Owned(x.to_string()));

        Ok(PythonVersionRequest {
            kind: kind.map(|x| x.to_string().into()),
            major,
            minor,
            patch,
            suffix,
        })
    }
}

impl fmt::Display for PythonVersionRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref kind) = self.kind {
            write!(f, "{}@", kind)?;
        }
        write!(f, "{}", self.major)?;
        if let Some(ref minor) = self.minor {
            write!(f, ".{}", minor)?;
            if let Some(ref patch) = self.patch {
                write!(f, ".{}", patch)?;
            }
        }
        Ok(())
    }
}

fn matches_version(req: &PythonVersionRequest, v: &PythonVersion) -> bool {
    if req.kind.as_deref().unwrap_or(DEFAULT_KIND) != v.kind {
        return false;
    }
    if req.major != v.major {
        return false;
    }
    if let Some(minor) = req.minor {
        if minor != v.minor {
            return false;
        }
    }
    if let Some(patch) = req.patch {
        if patch != v.patch {
            return false;
        }
    }
    if let Some(ref suffix) = req.suffix {
        if Some(suffix) != v.suffix.as_ref() {
            return false;
        }
    }
    true
}

/// Given a version, platform and architecture returns the download URL.
pub fn get_download_url(
    requested_version: &PythonVersionRequest,
    platform: &str,
    arch: &str,
) -> Option<(PythonVersion, &'static str)> {
    for (it_version, it_arch, it_platform, it_url) in indygreg_python::CPYTHON_VERSIONS {
        if platform == *it_platform
            && arch == *it_arch
            && matches_version(requested_version, it_version)
        {
            return Some((it_version.clone(), it_url));
        }
    }
    None
}

/// Returns an iterator over downloadable installations.
pub fn iter_downloadable<'s>(
    platform: &'s str,
    arch: &'s str,
) -> impl Iterator<Item = PythonVersion> + 's {
    indygreg_python::CPYTHON_VERSIONS.iter().filter_map(
        move |(version, it_arch, it_platform, _)| {
            if *it_arch == arch && *it_platform == platform {
                Some(version.clone())
            } else {
                None
            }
        },
    )
}

#[test]
fn test_get_download_url() {
    let url = get_download_url(&"3.8.14".parse().unwrap(), "macos", "aarch64");
    assert_eq!(url, Some((PythonVersion { kind: "cpython".into(), major: 3, minor: 8, patch: 14, suffix: None }, "https://github.com/indygreg/python-build-standalone/releases/download/20221002/cpython-3.8.14%2B20221002-aarch64-apple-darwin-pgo-full.tar.zst")));
    let url = get_download_url(&"3.8".parse().unwrap(), "macos", "aarch64");
    assert_eq!(url, Some((PythonVersion { kind: "cpython".into(), major: 3, minor: 8, patch: 16, suffix: None }, "https://github.com/indygreg/python-build-standalone/releases/download/20221220/cpython-3.8.16%2B20221220-aarch64-apple-darwin-pgo-full.tar.zst")));
    let url = get_download_url(&"3".parse().unwrap(), "macos", "aarch64");
    assert_eq!(url, Some((PythonVersion { kind: "cpython".into(), major: 3, minor: 11, patch: 1, suffix: None }, "https://github.com/indygreg/python-build-standalone/releases/download/20230116/cpython-3.11.1%2B20230116-aarch64-apple-darwin-pgo-full.tar.zst")));
}

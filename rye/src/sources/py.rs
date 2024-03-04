use std::borrow::Cow;
use std::env::consts::{ARCH, OS};
use std::fmt;
use std::str::FromStr;

use anyhow::{anyhow, Error};
use pep440_rs::Version;
use serde::{de, Deserialize, Serialize};

mod downloads {
    use super::PythonVersion;
    include!("generated/python_downloads.inc");
}

const DEFAULT_NAME: &str = "cpython";

/// Internal descriptor for a python version.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct PythonVersion {
    pub name: Cow<'static, str>,
    pub arch: Cow<'static, str>,
    pub os: Cow<'static, str>,
    pub environment: Option<Cow<'static, str>>,
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub suffix: Option<Cow<'static, str>>,
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
            name: match req.name() {
                DEFAULT_NAME => Cow::Borrowed(DEFAULT_NAME),
                other => Cow::Owned(other.to_string()),
            },
            arch: match req.arch() {
                ARCH => Cow::Borrowed(ARCH),
                other => Cow::Owned(other.to_string()),
            },
            os: match req.os() {
                OS => Cow::Borrowed(OS),
                other => Cow::Owned(other.to_string()),
            },
            environment: req
                .environment()
                .map(|environment| Cow::Owned(environment.to_string())),
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
            name: match req.name() {
                DEFAULT_NAME => Cow::Borrowed(DEFAULT_NAME),
                other => Cow::Owned(other.to_string()),
            },
            arch: match req.arch() {
                ARCH => Cow::Borrowed(ARCH),
                other => Cow::Owned(other.to_string()),
            },
            os: match req.os() {
                OS => Cow::Borrowed(OS),
                other => Cow::Owned(other.to_string()),
            },
            environment: req
                .environment()
                .map(|environment| Cow::Owned(environment.to_string())),
            major: req.major,
            minor: req.minor.ok_or_else(|| anyhow!("missing minor version"))?,
            patch: req.patch.ok_or_else(|| anyhow!("missing patch version"))?,
            suffix: req.suffix,
        })
    }
}

impl fmt::Display for PythonVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if self.arch != ARCH || self.os != OS {
            write!(f, "-{}", self.arch)?;
            if self.os != OS {
                write!(f, "-{}", self.os)?;
            }
            if let Some(environment) = &self.environment {
                write!(f, "-{}", environment)?;
            }
        }
        write!(f, "@{}.{}.{}", self.major, self.minor, self.patch)?;

        if let Some(ref suffix) = self.suffix {
            write!(f, ".{}", suffix)?;
        }
        Ok(())
    }
}

impl From<PythonVersion> for Version {
    fn from(value: PythonVersion) -> Self {
        Version {
            epoch: 0,
            release: vec![value.major as u64, value.minor as u64, value.patch as u64],
            pre: None,
            post: None,
            dev: None,
            local: None,
        }
    }
}

impl From<PythonVersionRequest> for Version {
    fn from(value: PythonVersionRequest) -> Self {
        Version {
            epoch: 0,
            release: vec![
                value.major as u64,
                value.minor.unwrap_or_default() as u64,
                value.patch.unwrap_or_default() as u64,
            ],
            pre: None,
            post: None,
            dev: None,
            local: None,
        }
    }
}

impl PythonVersion {
    /// Returns a simplified format of the version request.
    pub fn format_simple(&self) -> String {
        use std::fmt::Write;
        let mut rv = format!("{}", self.major);
        write!(rv, ".{}", self.minor).unwrap();
        write!(rv, ".{}", self.patch).unwrap();
        rv
    }
}

/// Internal descriptor for a python version request.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct PythonVersionRequest {
    pub name: Option<Cow<'static, str>>,
    pub arch: Option<Cow<'static, str>>,
    pub os: Option<Cow<'static, str>>,
    pub environment: Option<Cow<'static, str>>,
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

    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or(DEFAULT_NAME)
    }

    pub fn arch(&self) -> &str {
        self.arch.as_deref().unwrap_or(ARCH)
    }

    pub fn os(&self) -> &str {
        self.os.as_deref().unwrap_or(OS)
    }

    pub fn environment(&self) -> Option<&str> {
        self.environment.as_deref()
    }
}

impl From<PythonVersion> for PythonVersionRequest {
    fn from(value: PythonVersion) -> Self {
        PythonVersionRequest {
            name: Some(value.name),
            arch: Some(value.arch),
            os: Some(value.os),
            environment: value.environment,
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
            name: None,
            arch: None,
            os: None,
            environment: None,
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
            Some((kind, version)) => (kind, version),
            None => ("", s),
        };
        let mut iter = version.split('.');
        let major = iter
            .next()
            .and_then(|x| x.parse::<u8>().ok())
            .ok_or_else(|| anyhow!("invalid syntax for version"))?;
        let minor = iter.next().and_then(|x| x.parse::<u8>().ok());
        let patch = iter.next().and_then(|x| x.parse::<u8>().ok());
        let suffix = iter.next().map(|x| Cow::Owned(x.to_string()));
        if iter.next().is_some() {
            return Err(anyhow!("unexpected garbage after version"));
        }

        let mut iter = kind.splitn(4, '-');

        Ok(PythonVersionRequest {
            name: match iter.next() {
                None | Some("") => None,
                Some(DEFAULT_NAME) => Some(Cow::Borrowed(DEFAULT_NAME)),
                Some(other) => Some(Cow::Owned(other.to_string())),
            },
            arch: iter.next().map(|x| x.to_string().into()),
            os: iter.next().map(|x| x.to_string().into()),
            environment: iter.next().map(|x| x.to_string().into()),
            major,
            minor,
            patch,
            suffix,
        })
    }
}

impl fmt::Display for PythonVersionRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref name) = self.name {
            write!(f, "{}", name)?;
            if let Some(ref arch) = self.arch {
                write!(f, "-{}", arch)?;
                if let Some(ref os) = self.os {
                    write!(f, "-{}", os)?;
                }
                if let Some(ref environment) = self.environment {
                    write!(f, "-{}", environment)?;
                }
            }
            write!(f, "@")?;
        }
        write!(f, "{}", self.major)?;
        if let Some(ref minor) = self.minor {
            write!(f, ".{}", minor)?;
            if let Some(ref patch) = self.patch {
                write!(f, ".{}", patch)?;
                if let Some(ref suffix) = self.suffix {
                    write!(f, ".{}", suffix)?;
                }
            }
        }
        Ok(())
    }
}

fn default_environment(os: &str) -> Option<&str> {
    match os {
        "linux" => Some("gnu"),
        _ => None,
    }
}

pub fn matches_version(req: &PythonVersionRequest, v: &PythonVersion) -> bool {
    if req.name.as_deref().unwrap_or(DEFAULT_NAME) != v.name {
        return false;
    }
    if req.arch.as_deref().unwrap_or(ARCH) != v.arch {
        return false;
    }
    if req.os.as_deref().unwrap_or(OS) != v.os {
        return false;
    }
    if req
        .environment
        .as_deref()
        .or(default_environment(req.os.as_deref().unwrap_or(OS)))
        != v.environment.as_deref()
    {
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
) -> Option<(PythonVersion, &'static str, Option<&'static str>)> {
    for (it_version, it_url, it_sha256) in downloads::PYTHON_VERSIONS {
        if matches_version(requested_version, it_version) {
            return Some((it_version.clone(), it_url, *it_sha256));
        }
    }
    None
}

/// Returns an iterator over downloadable installations.
pub fn iter_downloadable<'s>(
    os: &'s str,
    arch: &'s str,
) -> impl Iterator<Item = PythonVersion> + 's {
    downloads::PYTHON_VERSIONS
        .iter()
        .filter_map(move |(version, _, _)| {
            if version.arch == arch && version.os == os {
                Some(version.clone())
            } else {
                None
            }
        })
}

#[test]
fn test_parse_version_request() {
    let request: PythonVersionRequest = "cpython-x86_64-linux-musl@3.12.1".parse().unwrap();
    assert_eq!(
        request,
        PythonVersionRequest {
            name: Some(Cow::Owned("cpython".into())),
            arch: Some("x86_64".into()),
            os: Some("linux".into()),
            environment: Some("musl".into()),
            major: 3,
            minor: Some(12),
            patch: Some(1),
            suffix: None,
        },
    );

    let request: PythonVersionRequest = "cpython-x86_64-linux-gnu@3.12.1".parse().unwrap();
    assert_eq!(
        request,
        PythonVersionRequest {
            name: Some(Cow::Owned("cpython".into())),
            arch: Some("x86_64".into()),
            os: Some("linux".into()),
            environment: Some("gnu".into()),
            major: 3,
            minor: Some(12),
            patch: Some(1),
            suffix: None,
        },
    );

    let request: PythonVersionRequest = "cpython-aarch64-macos@3.12.1".parse().unwrap();
    assert_eq!(
        request,
        PythonVersionRequest {
            name: Some(Cow::Owned("cpython".into())),
            arch: Some("aarch64".into()),
            os: Some("macos".into()),
            environment: None,
            major: 3,
            minor: Some(12),
            patch: Some(1),
            suffix: None,
        },
    );
}

#[test]
fn test_version_match() {
    let request: PythonVersionRequest = "cpython-aarch64-macos@3.12.1".parse().unwrap();
    assert!(matches_version(
        &request,
        &PythonVersion {
            name: "cpython".into(),
            arch: "aarch64".into(),
            os: "macos".into(),
            environment: None,
            major: 3,
            minor: 12,
            patch: 1,
            suffix: None,
        }
    ));

    let request: PythonVersionRequest = "cpython-x86_64-linux-musl@3.12.1".parse().unwrap();
    assert!(matches_version(
        &request,
        &PythonVersion {
            name: "cpython".into(),
            arch: "x86_64".into(),
            os: "linux".into(),
            environment: Some("musl".into()),
            major: 3,
            minor: 12,
            patch: 1,
            suffix: None,
        }
    ));

    let request: PythonVersionRequest = "cpython-x86_64-linux@3.12.1".parse().unwrap();
    assert!(matches_version(
        &request,
        &PythonVersion {
            name: "cpython".into(),
            arch: "x86_64".into(),
            os: "linux".into(),
            environment: Some("gnu".into()),
            major: 3,
            minor: 12,
            patch: 1,
            suffix: None,
        }
    ));
}

#[test]
fn test_get_download_url() {
    {
        let url = get_download_url(&"cpython-aarch64-macos@3.8.14".parse().unwrap());
        assert_eq!(url, Some((PythonVersion { name: "cpython".into(), arch: "aarch64".into(), os: "macos".into(), environment: None, major: 3, minor: 8, patch: 14, suffix: None }, "https://github.com/indygreg/python-build-standalone/releases/download/20221002/cpython-3.8.14%2B20221002-aarch64-apple-darwin-pgo%2Blto-full.tar.zst", Some("d17a3fcc161345efa2ec0b4ab9c9ed6c139d29128f2e34bb636338a484aa7b72"))));
    }
    {
        let url = get_download_url(&"cpython-x86_64-linux-musl@3.12.1".parse().unwrap());
        assert_eq!(url, Some((PythonVersion { name: "cpython".into(), arch: "x86_64".into(), os: "linux".into(), environment: Some("musl".into()), major: 3, minor: 12, patch: 1, suffix: None }, "https://github.com/indygreg/python-build-standalone/releases/download/20240107/cpython-3.12.1%2B20240107-x86_64-unknown-linux-musl-lto-full.tar.zst", Some("c4b07a02d8f0986b56e010a67132e5eeba1def4991c6c06ed184f831a484a06f"))));
    }
}

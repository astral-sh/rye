use anyhow::{anyhow, Error};
use std::borrow::Cow;
use std::env::consts::{ARCH, OS};

mod downloads {
    use super::UvDownload;
    include!("generated/uv_downloads.inc");
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct UvDownload {
    pub arch: Cow<'static, str>,
    pub os: Cow<'static, str>,
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub suffix: Option<Cow<'static, str>>,
    pub url: Cow<'static, str>,
    pub sha256: Cow<'static, str>,
}

impl std::fmt::Display for UvDownload {
    // The format of the version string is: "uv-<arch>-<os>@<major>.<minor>.<patch>.<suffix>"
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "uv")?;
        if self.arch != ARCH || self.os != OS {
            write!(f, "-{}", self.arch)?;
            if self.os != OS {
                write!(f, "-{}", self.os)?;
            }
        }
        write!(f, "@{}.{}.{}", self.major, self.minor, self.patch)?;

        if let Some(ref suffix) = self.suffix {
            write!(f, ".{}", suffix)?;
        }
        Ok(())
    }
}

impl UvDownload {
    // See [`UvDownload::fmt`] for the format of the version string.
    pub fn version(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// This is the request for the version of uv to download.
// At the moment, we only support requesting the current architecture and OS.
// We only have one version included in the binary, so we do not need to request
// versions just yet. However, this implementation is designed to be extensible.
pub struct UvRequest {
    pub arch: Option<Cow<'static, str>>,
    pub os: Option<Cow<'static, str>>,
}

impl Default for UvRequest {
    fn default() -> Self {
        Self {
            arch: Some(ARCH.into()),
            os: Some(OS.into()),
        }
    }
}

impl TryFrom<UvRequest> for UvDownload {
    type Error = Error;

    // Searches our list of downloads for the current architecture and OS.
    // Note: We do not need to search for versions just yet, since we only have one of
    // uv at a time.
    fn try_from(v: UvRequest) -> Result<Self, Self::Error> {
        downloads::UV_DOWNLOADS
            .iter()
            .rev()
            .find(|d| {
                (v.arch.is_none() || v.arch.as_ref().unwrap() == &d.arch)
                    && (v.os.is_none() || v.os.as_ref().unwrap() == &d.os)
            })
            .cloned()
            .ok_or_else(|| anyhow!("No matching download found"))
    }
}

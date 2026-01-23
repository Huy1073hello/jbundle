use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildSystem {
    DepsEdn,
    Leiningen,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetOs {
    Linux,
    MacOs,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetArch {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone)]
pub struct Target {
    pub os: TargetOs,
    pub arch: TargetArch,
}

impl Target {
    pub fn current() -> Self {
        let os = if cfg!(target_os = "macos") {
            TargetOs::MacOs
        } else {
            TargetOs::Linux
        };
        let arch = if cfg!(target_arch = "aarch64") {
            TargetArch::Aarch64
        } else {
            TargetArch::X86_64
        };
        Self { os, arch }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "linux-x64" => Some(Self { os: TargetOs::Linux, arch: TargetArch::X86_64 }),
            "linux-aarch64" => Some(Self { os: TargetOs::Linux, arch: TargetArch::Aarch64 }),
            "macos-x64" => Some(Self { os: TargetOs::MacOs, arch: TargetArch::X86_64 }),
            "macos-aarch64" => Some(Self { os: TargetOs::MacOs, arch: TargetArch::Aarch64 }),
            _ => None,
        }
    }

    pub fn adoptium_os(&self) -> &'static str {
        match self.os {
            TargetOs::Linux => "linux",
            TargetOs::MacOs => "mac",
        }
    }

    pub fn adoptium_arch(&self) -> &'static str {
        match self.arch {
            TargetArch::X86_64 => "x64",
            TargetArch::Aarch64 => "aarch64",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub input: PathBuf,
    pub output: PathBuf,
    pub java_version: u8,
    pub target: Target,
    pub jvm_args: Vec<String>,
}

impl BuildConfig {
    pub fn cache_dir() -> PathBuf {
        dirs::home_dir()
            .expect("cannot determine home directory")
            .join(".clj-pack")
            .join("cache")
    }
}

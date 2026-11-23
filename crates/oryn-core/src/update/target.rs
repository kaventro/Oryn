use std::path::{Path, PathBuf};

use super::install::{BinaryInstaller, BundleInstaller, Installer, TarballInstaller};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Windows,
    Linux,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
    Aarch64,
    X86_64,
    Other,
}

impl Arch {
    fn name(self) -> Option<&'static str> {
        match self {
            Arch::Aarch64 => Some("aarch64"),
            Arch::X86_64 => Some("x86_64"),
            Arch::Other => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    pub fn current() -> Self {
        let os = match std::env::consts::OS {
            "macos" => Os::MacOs,
            "windows" => Os::Windows,
            "linux" => Os::Linux,
            _ => Os::Other,
        };
        let arch = match std::env::consts::ARCH {
            "aarch64" => Arch::Aarch64,
            "x86_64" => Arch::X86_64,
            _ => Arch::Other,
        };
        Self { os, arch }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Method {
    Bundle { app: PathBuf },
    Binary { exe: PathBuf },
    Tarball { exe: PathBuf, member: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub asset: String,
    pub checksums: String,
    pub method: Method,
}

impl Target {
    pub fn installer(&self) -> Box<dyn Installer> {
        match &self.method {
            Method::Bundle { app } => Box::new(BundleInstaller::new(app.clone())),
            Method::Binary { exe } => Box::new(BinaryInstaller::new(exe.clone())),
            Method::Tarball { exe, member } => {
                Box::new(TarballInstaller::new(exe.clone(), member.clone()))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    Automatic(Target),
    Manual(&'static str),
}

pub fn plan(platform: Platform, exe: &Path, version: &str) -> Plan {
    let Some(arch) = platform.arch.name() else {
        return Plan::Manual("Oryn can't update itself on this processor.");
    };
    match platform.os {
        Os::MacOs => mac_plan(exe, arch, version),
        Os::Windows if platform.arch == Arch::X86_64 => Plan::Automatic(Target {
            asset: format!("Oryn_{version}_x64_portable.exe"),
            checksums: "SHA256SUMS-windows.txt".into(),
            method: Method::Binary {
                exe: exe.to_path_buf(),
            },
        }),
        Os::Linux => linux_plan(exe, arch, version),
        Os::Windows | Os::Other => Plan::Manual("Oryn can't update itself on this system."),
    }
}

fn mac_plan(exe: &Path, arch: &str, version: &str) -> Plan {
    let Some(app) = bundle_of(exe) else {
        return Plan::Manual("Oryn updates itself only when it runs from Oryn.app.");
    };
    if app.to_string_lossy().contains("/AppTranslocation/") {
        return Plan::Manual(
            "macOS is running Oryn from a temporary copy. Move Oryn to Applications, open it from there and try again.",
        );
    }
    Plan::Automatic(Target {
        asset: format!("Oryn_{version}_{arch}.app.zip"),
        checksums: "SHA256SUMS-macos.txt".into(),
        method: Method::Bundle { app },
    })
}

fn linux_plan(exe: &Path, arch: &str, version: &str) -> Plan {
    let managed = ["/usr/", "/nix/", "/snap/", "/opt/"];
    if managed.iter().any(|root| exe.starts_with(root)) {
        return Plan::Manual("Oryn was installed by your package manager. Update it from there.");
    }
    let folder = format!("Oryn_{version}_linux_{arch}");
    Plan::Automatic(Target {
        asset: format!("{folder}.tar.gz"),
        checksums: "SHA256SUMS-linux.txt".into(),
        method: Method::Tarball {
            exe: exe.to_path_buf(),
            member: format!("{folder}/oryn"),
        },
    })
}

fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    let shaped = macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && app.extension()? == "app";
    shaped.then(|| app.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAC_ARM: Platform = Platform {
        os: Os::MacOs,
        arch: Arch::Aarch64,
    };

    #[test]
    fn mac_updates_the_whole_bundle() {
        let exe = Path::new("/Applications/Oryn.app/Contents/MacOS/oryn");
        assert_eq!(
            plan(MAC_ARM, exe, "0.0.11"),
            Plan::Automatic(Target {
                asset: "Oryn_0.0.11_aarch64.app.zip".into(),
                checksums: "SHA256SUMS-macos.txt".into(),
                method: Method::Bundle {
                    app: PathBuf::from("/Applications/Oryn.app")
                },
            })
        );
    }

    #[test]
    fn mac_needs_a_bundle_outside_translocation() {
        let bare = Path::new("/Users/dev/oryn/target/release/oryn");
        assert!(matches!(plan(MAC_ARM, bare, "1.0.0"), Plan::Manual(_)));
        let moved =
            Path::new("/private/var/folders/x/AppTranslocation/ABC/d/Oryn.app/Contents/MacOS/oryn");
        assert!(matches!(plan(MAC_ARM, moved, "1.0.0"), Plan::Manual(_)));
    }

    #[test]
    fn windows_swaps_in_the_portable_exe() {
        let exe = Path::new(r"C:\Users\dev\AppData\Local\Programs\Oryn\Oryn.exe");
        let windows = Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        };
        let Plan::Automatic(target) = plan(windows, exe, "0.0.11") else {
            panic!("expected an automatic plan");
        };
        assert_eq!(target.asset, "Oryn_0.0.11_x64_portable.exe");
        assert_eq!(target.checksums, "SHA256SUMS-windows.txt");
        let arm = Platform {
            os: Os::Windows,
            arch: Arch::Aarch64,
        };
        assert!(matches!(plan(arm, exe, "0.0.11"), Plan::Manual(_)));
    }

    #[test]
    fn linux_uses_the_tarball_unless_a_package_manager_owns_oryn() {
        let linux = Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        };
        let Plan::Automatic(target) = plan(linux, Path::new("/home/dev/apps/oryn"), "0.0.11")
        else {
            panic!("expected an automatic plan");
        };
        assert_eq!(target.asset, "Oryn_0.0.11_linux_x86_64.tar.gz");
        assert_eq!(
            target.method,
            Method::Tarball {
                exe: PathBuf::from("/home/dev/apps/oryn"),
                member: "Oryn_0.0.11_linux_x86_64/oryn".into(),
            }
        );
        assert!(matches!(
            plan(linux, Path::new("/usr/bin/oryn"), "0.0.11"),
            Plan::Manual(_)
        ));
    }
}

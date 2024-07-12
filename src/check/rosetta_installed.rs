use nix_rs::{
    env::{MacOSArch, OS},
    info,
};
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::traits::{Check, CheckResult, Checkable};

/// Check if rosetta 2 is installed
///
/// Applies to ARM macs.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct RosettaInstalled {
    enable: bool,
    required: bool,
}

impl Checkable for RosettaInstalled {
    fn check(
        &self,
        nix_info: &info::NixInfo,
        _: Option<&nix_rs::flake::url::FlakeUrl>,
    ) -> Vec<Check> {
        let mut checks = vec![];
        if let (
            true,
            OS::MacOS {
                nix_darwin: _,
                arch: MacOSArch::Arm64(_),
            },
        ) = (self.enable, &nix_info.nix_env.os)
        {
            let check = Check {
                title: "Rosetta Installed".to_string(),
                info: "".to_string(),
                result: if is_rosetta_installed() {
                    CheckResult::Green
                } else {
                    CheckResult::Red {
                        msg: "Rosetta 2 is not installed".to_string(),
                        suggestion: "Install Rosetta 2 by running: `softwareupdate --install-rosetta --agree-to-license`".to_string(),
                    }
                },
                required: self.required,
            };
            checks.push(check);
        };
        checks
    }
}

fn is_rosetta_installed() -> bool {
    let output = Command::new("pkgutil")
        .arg("--pkg-info")
        .arg("com.apple.pkg.RosettaUpdateAuto")
        .output()
        .expect("Failed to execute command");

    output.status.success()
}

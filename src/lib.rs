//! Health checks for the user's Nix install

pub mod check;
pub mod report;
pub mod traits;

use colored::Colorize;
use std::vec::IntoIter;

use check::direnv::Direnv;
use nix_rs::command::NixCmd;
use nix_rs::flake::eval::nix_eval_attr_json;
use nix_rs::flake::url::FlakeUrl;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use self::check::{
    caches::Caches, flake_enabled::FlakeEnabled, max_jobs::MaxJobs, min_nix_version::MinNixVersion,
    rosetta::Rosetta, trusted_users::TrustedUsers,
};

/// Nix Health check information for user's install
///
/// Each field represents an individual check which satisfies the [Checkable] trait.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct NixHealth {
    pub max_jobs: MaxJobs,
    pub caches: Caches,
    pub flake_enabled: FlakeEnabled,
    pub nix_version: MinNixVersion,
    pub system: check::system::System,
    pub trusted_users: TrustedUsers,
    pub rosetta: Rosetta,
    pub direnv: Direnv,
}

impl<'a> IntoIterator for &'a NixHealth {
    type Item = &'a dyn traits::Checkable;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    /// Return an iterator to iterate on the fields of [NixHealth]
    fn into_iter(self) -> Self::IntoIter {
        let items: Vec<Self::Item> = vec![
            &self.rosetta,
            &self.nix_version,
            &self.flake_enabled,
            &self.system,
            &self.max_jobs,
            &self.caches,
            &self.trusted_users,
            &self.direnv,
        ];
        items.into_iter()
    }
}

impl NixHealth {
    /// Create [NixHealth] using configuration from the given flake
    ///
    /// Fallback to using the default health check config if the flake doesn't
    /// override it.
    pub async fn from_flake(url: &FlakeUrl) -> Result<Self, nix_rs::command::NixCmdError> {
        nix_eval_attr_json(
            &NixCmd::default(),
            &url.with_fully_qualified_root_attr("nix-health"),
            true,
        )
        .await
    }

    /// Run all checks and collect the results
    #[instrument(skip_all)]
    pub fn run_checks(
        &self,
        nix_info: &nix_rs::info::NixInfo,
        flake_url: Option<FlakeUrl>,
    ) -> Vec<traits::Check> {
        NixHealth::run_checks_with(self.into_iter(), nix_info, flake_url)
    }

    pub fn run_checks_with(
        items: IntoIter<&dyn traits::Checkable>,
        nix_info: &nix_rs::info::NixInfo,
        flake_url: Option<FlakeUrl>,
    ) -> Vec<traits::Check> {
        tracing::info!("🩺 Running health checks");
        items
            .flat_map(|c| c.check(nix_info, flake_url.as_ref()))
            .collect()
    }

    pub fn print_report_returning_exit_code(checks: &[traits::Check], quiet: bool) -> i32 {
        let mut res = AllChecksResult::new();
        for check in checks {
            match &check.result {
                traits::CheckResult::Green => {
                    if !quiet {
                        println!("{}", format!("✅ {}", check.title).green().bold());
                        println!("   {}", check.info.blue());
                    }
                }
                traits::CheckResult::Red { msg, suggestion } => {
                    res.register_failure(check.required);
                    if check.required {
                        println!("{}", format!("❌ {}", check.title).red().bold());
                    } else {
                        println!("{}", format!("🟧 {}", check.title).yellow().bold());
                    }
                    println!("   {}", check.info.blue());
                    println!("   {}", msg.yellow());
                    println!("   {}", suggestion);
                }
            }
        }
        res.report()
    }

    pub fn schema() -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&NixHealth::default())
    }
}

/// A convenient type to aggregate check failures, and summary report at end.
enum AllChecksResult {
    Pass,
    PassSomeFail,
    Fail,
}

impl AllChecksResult {
    fn new() -> Self {
        AllChecksResult::Pass
    }

    fn register_failure(&mut self, required: bool) {
        if required {
            *self = AllChecksResult::Fail;
        } else if matches!(self, AllChecksResult::Pass) {
            *self = AllChecksResult::PassSomeFail;
        }
    }

    fn report(self) -> i32 {
        match self {
            AllChecksResult::Pass => {
                println!("{}", "✅ All checks passed".green().bold());
                0
            }
            AllChecksResult::PassSomeFail => {
                println!(
                    "{}, {}",
                    "✅ Required checks passed".green().bold(),
                    "but some non-required checks failed".yellow().bold()
                );
                0
            }
            AllChecksResult::Fail => {
                println!("{}", "❌ Some required checks failed".red().bold());
                1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::check::{caches::Caches, min_nix_version::MinNixVersion};

    #[test]
    fn test_json_deserialize_empty() {
        let json = r#"{}"#;
        let v: super::NixHealth = serde_json::from_str(json).unwrap();
        assert_eq!(v.nix_version, MinNixVersion::default());
        assert_eq!(v.caches, Caches::default());
        println!("{:?}", v);
    }

    #[test]
    fn test_json_deserialize_nix_version() {
        let json = r#"{ "nix-version": { "min-required": "2.17.0" } }"#;
        let v: super::NixHealth = serde_json::from_str(json).unwrap();
        assert_eq!(v.nix_version.min_required.to_string(), "2.17.0");
        assert_eq!(v.caches, Caches::default());
    }

    #[test]
    fn test_json_deserialize_caches() {
        let json = r#"{ "caches": { "required": ["https://foo.cachix.org"] } }"#;
        let v: super::NixHealth = serde_json::from_str(json).unwrap();
        assert_eq!(
            v.caches.required,
            vec![url::Url::parse("https://foo.cachix.org").unwrap()]
        );
        assert_eq!(v.nix_version, MinNixVersion::default());
    }
}

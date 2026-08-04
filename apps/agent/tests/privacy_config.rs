#![cfg(windows)]

use std::{fs, path::PathBuf};

use pastral_agent::PrivacyPolicyConfig;
use pastral_agent_core::{SourceAdmissionDecision, SourceObservation};
use pastral_domain::ClipEventId;

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pastral-agent-privacy-config-test-{}",
            ClipEventId::new_v4()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn default_policy_is_created_once_and_fails_closed_for_unresolved_sources() {
    let root = TestRoot::new();

    let first = PrivacyPolicyConfig::load_or_create(root.path()).unwrap();
    let second = PrivacyPolicyConfig::load_or_create(root.path()).unwrap();

    assert_eq!(first, second);
    assert!(first.source_policy().deny_unresolved_source());
    assert_eq!(
        first
            .source_policy()
            .evaluate(&SourceObservation::unavailable()),
        SourceAdmissionDecision::DenyUnresolved
    );
    assert_eq!(
        first.source_policy().denied_executable_names(),
        &[
            "1password.exe".to_owned(),
            "bitwarden.exe".to_owned(),
            "keepass.exe".to_owned(),
            "keepassxc.exe".to_owned(),
        ]
    );
    assert_eq!(
        fs::read_to_string(root.path().join("privacy-policy.txt")).unwrap(),
        concat!(
            "version=1\n",
            "deny_unresolved_source=true\n",
            "deny_process=1password.exe\n",
            "deny_process=bitwarden.exe\n",
            "deny_process=keepass.exe\n",
            "deny_process=keepassxc.exe\n",
        )
    );
}

#[test]
fn custom_exact_names_are_normalized_case_insensitively() {
    let root = TestRoot::new();
    fs::write(
        root.path().join("privacy-policy.txt"),
        concat!(
            "version=1\n",
            "deny_unresolved_source=false\n",
            "deny_process=CustomVault.EXE\n",
            "deny_process=customvault.exe\n",
        ),
    )
    .unwrap();

    let config = PrivacyPolicyConfig::load_or_create(root.path()).unwrap();
    assert!(!config.source_policy().deny_unresolved_source());
    assert_eq!(
        config.source_policy().denied_executable_names(),
        &["customvault.exe".to_owned()]
    );
    assert_eq!(
        config
            .source_policy()
            .evaluate(&SourceObservation::from_executable_name("CUSTOMVAULT.EXE").unwrap()),
        SourceAdmissionDecision::DenyExecutable
    );
}

#[test]
fn malformed_existing_policy_fails_closed_without_replacement() {
    for content in [
        "version=1\ndeny_process=keepass.exe\n",
        "version=2\ndeny_unresolved_source=true\n",
        "version=1\ndeny_unresolved_source=maybe\n",
        "version=1\ndeny_unresolved_source=true\nunknown=value\n",
        "version=1\ndeny_unresolved_source=true\ndeny_process=C:\\Vault\\vault.exe\n",
    ] {
        let root = TestRoot::new();
        let path = root.path().join("privacy-policy.txt");
        fs::write(&path, content).unwrap();

        assert!(PrivacyPolicyConfig::load_or_create(root.path()).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), content);
    }
}

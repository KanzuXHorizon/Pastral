use pastral_agent_core::{
    MAX_SECRET_SCAN_BYTES, SensitiveClass, SourceAdmissionDecision, SourceAdmissionPolicy,
    SourceConfidence, SourceObservation, detect_high_confidence_secret,
};

#[test]
fn unresolved_source_is_denied_when_policy_requires_resolution() {
    let policy = SourceAdmissionPolicy::new(true, Vec::<String>::new()).unwrap();
    let observation = SourceObservation::unavailable();

    assert_eq!(observation.confidence(), SourceConfidence::Unavailable);
    assert_eq!(
        policy.evaluate(&observation),
        SourceAdmissionDecision::DenyUnresolved
    );
}

#[test]
fn resolved_unlisted_source_is_allowed() {
    let policy = SourceAdmissionPolicy::new(true, ["keepassxc.exe"]).unwrap();
    let observation = SourceObservation::from_executable_name("notepad.exe").unwrap();

    assert_eq!(observation.confidence(), SourceConfidence::ProcessImage);
    assert_eq!(observation.executable_name(), Some("notepad.exe"));
    assert_eq!(
        policy.evaluate(&observation),
        SourceAdmissionDecision::Allow
    );
}

#[test]
fn exact_executable_match_is_case_insensitive() {
    let policy = SourceAdmissionPolicy::new(false, ["KeePassXC.EXE", "bitwarden.exe"]).unwrap();

    assert_eq!(
        policy.evaluate(&SourceObservation::from_executable_name("keepassxc.exe").unwrap()),
        SourceAdmissionDecision::DenyExecutable
    );
    assert_eq!(
        policy.evaluate(&SourceObservation::from_executable_name("keepassxc-helper.exe").unwrap()),
        SourceAdmissionDecision::Allow
    );
}

#[test]
fn path_like_or_empty_executable_names_are_rejected() {
    assert!(SourceObservation::from_executable_name("").is_err());
    assert!(SourceObservation::from_executable_name("C:\\Apps\\safe.exe").is_err());
    assert!(SourceObservation::from_executable_name("folder/safe.exe").is_err());
    assert!(SourceAdmissionPolicy::new(false, ["D:\\KeePass\\KeePass.exe"]).is_err());
}

#[test]
fn duplicate_denied_names_are_normalized_deterministically() {
    let policy = SourceAdmissionPolicy::new(
        false,
        [
            "KeePass.exe",
            "keepass.exe",
            "Bitwarden.exe",
            "BITWARDEN.EXE",
        ],
    )
    .unwrap();

    assert_eq!(
        policy.denied_executable_names(),
        &["bitwarden.exe".to_owned(), "keepass.exe".to_owned()]
    );
}

#[test]
fn private_key_envelopes_are_detected_at_high_confidence() {
    for marker in [
        concat!("-----BEGIN ", "PRIVATE KEY-----"),
        concat!("-----BEGIN ENCRYPTED ", "PRIVATE KEY-----"),
        concat!("-----BEGIN RSA ", "PRIVATE KEY-----"),
        concat!("-----BEGIN EC ", "PRIVATE KEY-----"),
        concat!("-----BEGIN DSA ", "PRIVATE KEY-----"),
        concat!("-----BEGIN OPENSSH ", "PRIVATE KEY-----"),
        concat!("-----BEGIN PGP ", "PRIVATE KEY BLOCK-----"),
    ] {
        let text = format!("header\n{marker}\nbody");
        assert_eq!(
            detect_high_confidence_secret(&text),
            Some(SensitiveClass::PrivateKeyMaterial),
            "marker was not detected: {marker}"
        );
    }
}

#[test]
fn detector_limit_is_fail_closed_and_ordinary_text_is_allowed() {
    assert_eq!(detect_high_confidence_secret("ordinary note"), None);
    let oversized = "x".repeat(MAX_SECRET_SCAN_BYTES + 1);
    assert_eq!(
        detect_high_confidence_secret(&oversized),
        Some(SensitiveClass::DetectorLimitExceeded)
    );
}

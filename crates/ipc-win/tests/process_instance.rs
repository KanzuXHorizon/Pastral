#![cfg(windows)]

use pastral_ipc_win::{LocalProcessInstance, acquire_local_process_instance, random_bytes};

fn unique_name(label: &str) -> String {
    let random = random_bytes::<16>().expect("system RNG must be available");
    let suffix = random
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        r"Local\Pastral.Test.{label}.{}.{suffix}",
        std::process::id()
    )
}

#[test]
fn first_local_instance_owns_name_second_is_rejected_and_drop_releases() {
    let name = unique_name("lifetime");
    let first = acquire_local_process_instance(&name).unwrap();
    assert!(matches!(first, LocalProcessInstance::Acquired(_)));

    let second = acquire_local_process_instance(&name).unwrap();
    assert!(matches!(second, LocalProcessInstance::AlreadyRunning));

    drop(first);
    let third = acquire_local_process_instance(&name).unwrap();
    assert!(matches!(third, LocalProcessInstance::Acquired(_)));
}

#[test]
fn local_instance_name_is_strict_and_bounded() {
    for invalid in [
        "",
        "Pastral.NoNamespace",
        r"Global\Pastral.Test",
        "Local\\",
        "Local\\Pastral\0Injected",
    ] {
        assert!(
            acquire_local_process_instance(invalid).is_err(),
            "{invalid:?}"
        );
    }

    let oversized = format!(r"Local\Pastral.Test.{}", "x".repeat(256));
    assert!(acquire_local_process_instance(&oversized).is_err());
}

use std::{fs, path::PathBuf, sync::Arc, thread};

use pastral_ipc_win::{
    IDENTITY_FILE_NAME, SECRET_FILE_NAME, TransportError, TransportIdentity, derive_pipe_name,
    load_or_create_transport_material, load_transport_material,
};
use uuid::Uuid;

fn temp_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("pastral-ipc-win-{label}-{}", Uuid::new_v4()))
}

fn secret_bytes(material: &pastral_ipc_win::TransportMaterial) -> [u8; 32] {
    material.secret().expose(|bytes| *bytes)
}

#[test]
fn material_is_created_once_and_reused_exactly() {
    let root = temp_root("stable");
    let first = load_or_create_transport_material(&root).unwrap();
    let second = load_or_create_transport_material(&root).unwrap();

    assert_eq!(first.identity(), second.identity());
    assert_eq!(first.identity().secret_version(), 1);
    assert!(!first.identity().instance_id().is_zero());
    assert_eq!(secret_bytes(&first), secret_bytes(&second));
    assert_ne!(secret_bytes(&first), [0; 32]);

    let identity_text = fs::read_to_string(root.join(IDENTITY_FILE_NAME)).unwrap();
    assert_eq!(identity_text.lines().count(), 3);
    assert!(identity_text.starts_with("version=1\ninstance_id="));
    assert!(identity_text.ends_with("\nsecret_version=1\n"));
    assert!(root.join(SECRET_FILE_NAME).is_file());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn existing_only_material_load_never_creates_missing_root() {
    let root = temp_root("existing-only");
    assert!(!root.exists());

    assert!(matches!(
        load_transport_material(&root),
        Err(TransportError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        })
    ));
    assert!(!root.exists());

    let created = load_or_create_transport_material(&root).unwrap();
    let loaded = load_transport_material(&root).unwrap();
    assert_eq!(created.identity(), loaded.identity());
    assert_eq!(secret_bytes(&created), secret_bytes(&loaded));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn canonical_pipe_name_uses_only_version_session_and_instance_uuid() {
    let root = temp_root("name");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join(IDENTITY_FILE_NAME),
        "version=1\ninstance_id=550e8400-e29b-41d4-a716-446655440000\nsecret_version=1\n",
    )
    .unwrap();

    let identity = TransportIdentity::load_or_create(&root).unwrap();
    let name = derive_pipe_name(&identity, 3).unwrap();
    assert_eq!(
        name.as_str(),
        r"\\.\pipe\Pastral-v1-s3-550e8400-e29b-41d4-a716-446655440000"
    );
    assert!(name.as_wide_nul().last() == Some(&0));
    assert!(name.as_wide_nul().len() <= 129);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_existing_identity_fails_closed_without_replacement() {
    let root = temp_root("bad-identity");
    fs::create_dir_all(&root).unwrap();
    let path = root.join(IDENTITY_FILE_NAME);
    let original = b"version=1\ninstance_id=not-a-uuid\nsecret_version=1\n";
    fs::write(&path, original).unwrap();

    assert!(matches!(
        TransportIdentity::load_or_create(&root),
        Err(TransportError::InvalidIdentity(_))
    ));
    assert_eq!(fs::read(&path).unwrap(), original);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_existing_secret_fails_closed_without_replacement() {
    let root = temp_root("bad-secret");
    let identity = TransportIdentity::load_or_create(&root).unwrap();
    assert_eq!(identity.secret_version(), 1);
    let path = root.join(SECRET_FILE_NAME);
    let original = b"not-a-dpapi-envelope";
    fs::write(&path, original).unwrap();

    assert!(matches!(
        load_or_create_transport_material(&root),
        Err(TransportError::InvalidSecretEnvelope(_)) | Err(TransportError::Windows { .. })
    ));
    assert_eq!(fs::read(&path).unwrap(), original);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_material_publication_converges_on_one_identity_and_secret() {
    let root = Arc::new(temp_root("concurrent"));
    let handles = (0..8)
        .map(|_| {
            let root = Arc::clone(&root);
            thread::spawn(move || {
                let material = load_or_create_transport_material(&root).unwrap();
                (*material.identity(), secret_bytes(&material))
            })
        })
        .collect::<Vec<_>>();

    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    for result in &results[1..] {
        assert_eq!(result, &results[0]);
    }

    fs::remove_dir_all(root.as_ref()).unwrap();
}

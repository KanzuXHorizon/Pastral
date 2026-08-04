use std::fs;

use pastral_ipc_win::{
    TransportError, build_logon_sid_pipe_security, create_first_pipe_server,
    current_token_identity, derive_pipe_name, load_or_create_transport_material,
};
use uuid::Uuid;

#[test]
fn first_instance_creation_detects_name_squatting_and_recovers_after_close() {
    let root = std::env::temp_dir().join(format!("pastral-pipe-create-{}", Uuid::new_v4()));
    let material = load_or_create_transport_material(&root).unwrap();
    let identity = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), identity.session_id()).unwrap();
    let security = build_logon_sid_pipe_security(&identity).unwrap();

    let first = create_first_pipe_server(&name, &security).unwrap();
    assert!(matches!(
        create_first_pipe_server(&name, &security),
        Err(TransportError::Windows { .. })
    ));
    drop(first);

    let reopened = create_first_pipe_server(&name, &security).unwrap();
    drop(reopened);
    fs::remove_dir_all(root).unwrap();
}

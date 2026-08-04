use std::{
    fs,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use pastral_ipc_auth::{AuthError, Nonce, NonceReplayCache};
use pastral_ipc_core::{Capability, CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits};
use pastral_ipc_win::{
    IDENTITY_FILE_NAME, PipeFrameStream, TransportError, build_logon_sid_pipe_security,
    client_handshake, client_handshake_with_nonce_for_test, create_first_pipe_server,
    current_token_identity, derive_pipe_name, load_or_create_transport_material, open_pipe_client,
    server_handshake, server_handshake_with_nonce_for_test,
};
use uuid::Uuid;

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(2)
}

#[test]
fn mutual_handshake_authenticates_kernel_peers_and_health_capability() {
    let root = std::env::temp_dir().join(format!("pastral-handshake-{}", Uuid::new_v4()));
    let material = load_or_create_transport_material(&root).unwrap();
    let identity = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), identity.session_id()).unwrap();
    let security = build_logon_sid_pipe_security(&identity).unwrap();
    let mut server = create_first_pipe_server(&name, &security).unwrap();
    let client_name = name.clone();
    let client_root = root.clone();

    let client_thread = thread::spawn(move || {
        let client = open_pipe_client(&client_name, deadline()).unwrap();
        let peer = client.peer_identity().unwrap();
        let stream = PipeFrameStream::from_client(client, FrameLimits::default());
        let material = load_or_create_transport_material(&client_root).unwrap();
        let authenticated = client_handshake(stream, &material, peer, deadline()).unwrap();
        (
            authenticated.peer().process_id(),
            authenticated.selected_minor(),
            authenticated.capabilities().to_vec(),
        )
    });

    server.connect(deadline()).unwrap();
    let peer = server.peer_identity().unwrap();
    let stream = PipeFrameStream::from_server(server, FrameLimits::default());
    let mut replay = NonceReplayCache::new(64).unwrap();
    let authenticated = server_handshake(stream, &material, peer, &mut replay, deadline()).unwrap();

    assert_eq!(authenticated.peer().process_id(), std::process::id());
    assert_eq!(authenticated.selected_minor(), 0);
    assert_eq!(authenticated.capabilities(), &[Capability::Health]);
    assert_eq!(
        client_thread.join().unwrap(),
        (std::process::id(), 0, vec![Capability::Health])
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wrong_installation_secret_is_rejected_before_authenticated_connection_exists() {
    let server_root = std::env::temp_dir().join(format!("pastral-server-{}", Uuid::new_v4()));
    let client_root = std::env::temp_dir().join(format!("pastral-client-{}", Uuid::new_v4()));
    let server_material = load_or_create_transport_material(&server_root).unwrap();
    fs::create_dir_all(&client_root).unwrap();
    fs::copy(
        server_root.join(IDENTITY_FILE_NAME),
        client_root.join(IDENTITY_FILE_NAME),
    )
    .unwrap();
    let identity = current_token_identity().unwrap();
    let name = derive_pipe_name(server_material.identity(), identity.session_id()).unwrap();
    let security = build_logon_sid_pipe_security(&identity).unwrap();
    let mut server = create_first_pipe_server(&name, &security).unwrap();
    let client_name = name.clone();

    let client_thread = thread::spawn(move || {
        let client = open_pipe_client(&client_name, deadline()).unwrap();
        let peer = client.peer_identity().unwrap();
        let stream = PipeFrameStream::from_client(client, FrameLimits::default());
        let wrong = load_or_create_transport_material(&client_root).unwrap();
        client_handshake(stream, &wrong, peer, deadline())
            .err()
            .unwrap()
    });

    server.connect(deadline()).unwrap();
    let peer = server.peer_identity().unwrap();
    let stream = PipeFrameStream::from_server(server, FrameLimits::default());
    let mut replay = NonceReplayCache::new(64).unwrap();
    assert_eq!(
        server_handshake(stream, &server_material, peer, &mut replay, deadline()).err(),
        Some(TransportError::Authentication(AuthError::ProofMismatch))
    );
    assert!(matches!(
        client_thread.join().unwrap(),
        TransportError::Disconnected | TransportError::Windows { .. }
    ));
    fs::remove_dir_all(server_root).unwrap();
}

#[test]
fn control_frame_before_authentication_is_rejected() {
    let root = std::env::temp_dir().join(format!("pastral-control-first-{}", Uuid::new_v4()));
    let material = load_or_create_transport_material(&root).unwrap();
    let identity = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), identity.session_id()).unwrap();
    let security = build_logon_sid_pipe_security(&identity).unwrap();
    let mut server = create_first_pipe_server(&name, &security).unwrap();
    let client_name = name.clone();

    let client_thread = thread::spawn(move || {
        let client = open_pipe_client(&client_name, deadline()).unwrap();
        let mut stream = PipeFrameStream::from_client(client, FrameLimits::default());
        let hello = stream.read_frame(deadline()).unwrap();
        assert_eq!(hello.header().kind(), FrameKind::HelloProto);
        let header = FrameHeader::new(
            FrameKind::ControlProto,
            0,
            0,
            CorrelationId::new_v4(),
            FrameLimits::default(),
        )
        .unwrap();
        stream
            .write_frame(&Frame::new(header, Vec::new()).unwrap(), deadline())
            .unwrap();
    });

    server.connect(deadline()).unwrap();
    let peer = server.peer_identity().unwrap();
    let stream = PipeFrameStream::from_server(server, FrameLimits::default());
    let mut replay = NonceReplayCache::new(64).unwrap();
    assert_eq!(
        server_handshake(stream, &material, peer, &mut replay, deadline()).err(),
        Some(TransportError::Protocol("expected client hello frame"))
    );
    client_thread.join().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn repeated_nonce_transcript_is_rejected_by_shared_replay_cache() {
    let root = std::env::temp_dir().join(format!("pastral-replay-{}", Uuid::new_v4()));
    let server_nonce = Nonce::from_bytes([0x11; 32]).unwrap();
    let client_nonce = Nonce::from_bytes([0x22; 32]).unwrap();
    let mut replay = NonceReplayCache::new(64).unwrap();

    for attempt in 0..2 {
        let material = load_or_create_transport_material(&root).unwrap();
        let identity = current_token_identity().unwrap();
        let name = derive_pipe_name(material.identity(), identity.session_id()).unwrap();
        let security = build_logon_sid_pipe_security(&identity).unwrap();
        let mut server = create_first_pipe_server(&name, &security).unwrap();
        let client_name = name.clone();
        let client_root = root.clone();
        let (result_tx, result_rx) = mpsc::channel();

        let client_thread = thread::spawn(move || {
            let client = open_pipe_client(&client_name, deadline()).unwrap();
            let peer = client.peer_identity().unwrap();
            let stream = PipeFrameStream::from_client(client, FrameLimits::default());
            let material = load_or_create_transport_material(&client_root).unwrap();
            let result = client_handshake_with_nonce_for_test(
                stream,
                &material,
                peer,
                client_nonce,
                deadline(),
            );
            result_tx.send(result.is_ok()).unwrap();
        });

        server.connect(deadline()).unwrap();
        let peer = server.peer_identity().unwrap();
        let stream = PipeFrameStream::from_server(server, FrameLimits::default());
        let result = server_handshake_with_nonce_for_test(
            stream,
            &material,
            peer,
            &mut replay,
            server_nonce,
            deadline(),
        );
        if attempt == 0 {
            assert!(result.is_ok());
            assert!(result_rx.recv().unwrap());
        } else {
            assert_eq!(
                result.err(),
                Some(TransportError::Authentication(AuthError::ReplayDetected))
            );
            assert!(!result_rx.recv().unwrap());
        }
        client_thread.join().unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}

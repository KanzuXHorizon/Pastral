use std::{
    fs,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use pastral_ipc_core::{CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits};
use pastral_ipc_win::{
    PipeFrameStream, build_logon_sid_pipe_security, create_first_pipe_server,
    current_token_identity, derive_pipe_name, load_or_create_transport_material, open_pipe_client,
};
use uuid::Uuid;

fn frame(body: &[u8]) -> Frame {
    let limits = FrameLimits::default();
    let header = FrameHeader::new(
        FrameKind::ControlProto,
        u32::try_from(body.len()).unwrap(),
        0,
        CorrelationId::new_v4(),
        limits,
    )
    .unwrap();
    Frame::new(header, body.to_vec()).unwrap()
}

#[test]
fn connected_endpoints_report_kernel_pid_session_and_exchange_frames() {
    let root = std::env::temp_dir().join(format!("pastral-pipe-stream-{}", Uuid::new_v4()));
    let material = load_or_create_transport_material(&root).unwrap();
    let identity = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), identity.session_id()).unwrap();
    let security = build_logon_sid_pipe_security(&identity).unwrap();
    let mut server = create_first_pipe_server(&name, &security).unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();

    let client_name = name.clone();
    let client_thread = thread::spawn(move || {
        let client =
            open_pipe_client(&client_name, Instant::now() + Duration::from_secs(2)).unwrap();
        let peer = client.peer_identity().unwrap();
        ready_tx
            .send((peer.process_id(), peer.session_id()))
            .unwrap();
        let mut stream = PipeFrameStream::from_client(client, FrameLimits::default());
        stream
            .write_frame(&frame(b"first"), Instant::now() + Duration::from_secs(2))
            .unwrap();
        stream
            .write_frame(&frame(b"second"), Instant::now() + Duration::from_secs(2))
            .unwrap();
        release_rx.recv().unwrap();
    });

    server
        .connect(Instant::now() + Duration::from_secs(2))
        .unwrap();
    let server_peer = server.peer_identity().unwrap();
    assert_eq!(server_peer.process_id(), std::process::id());
    assert_eq!(server_peer.session_id(), identity.session_id());
    assert_eq!(
        ready_rx.recv().unwrap(),
        (std::process::id(), identity.session_id())
    );

    let mut stream = PipeFrameStream::from_server(server, FrameLimits::default());
    assert_eq!(
        stream
            .read_frame(Instant::now() + Duration::from_secs(2))
            .unwrap()
            .body(),
        b"first"
    );
    assert_eq!(
        stream
            .read_frame(Instant::now() + Duration::from_secs(2))
            .unwrap()
            .body(),
        b"second"
    );

    release_tx.send(()).unwrap();
    client_thread.join().unwrap();
    fs::remove_dir_all(root).unwrap();
}

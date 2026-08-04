use std::{
    fs,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use pastral_ipc_core::FrameLimits;
use pastral_ipc_win::{
    PipeFrameStream, TransportError, build_logon_sid_pipe_security, create_first_pipe_server,
    current_token_identity, derive_pipe_name, load_or_create_transport_material, open_pipe_client,
};
use uuid::Uuid;

fn setup(
    label: &str,
) -> (
    std::path::PathBuf,
    pastral_ipc_win::PipeName,
    pastral_ipc_win::PipeSecurity,
) {
    let root = std::env::temp_dir().join(format!("pastral-pipe-{label}-{}", Uuid::new_v4()));
    let material = load_or_create_transport_material(&root).unwrap();
    let identity = current_token_identity().unwrap();
    let name = derive_pipe_name(material.identity(), identity.session_id()).unwrap();
    let security = build_logon_sid_pipe_security(&identity).unwrap();
    (root, name, security)
}

#[test]
fn server_connect_without_client_times_out_and_remains_closeable() {
    let (root, name, security) = setup("connect-timeout");
    let mut server = create_first_pipe_server(&name, &security).unwrap();
    assert!(matches!(
        server.connect(Instant::now() + Duration::from_millis(40)),
        Err(TransportError::Timeout("connect named pipe"))
    ));
    drop(server);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn silent_connected_client_causes_bounded_read_timeout() {
    let (root, name, security) = setup("read-timeout");
    let mut server = create_first_pipe_server(&name, &security).unwrap();
    let (release_tx, release_rx) = mpsc::channel();
    let client_name = name.clone();
    let client_thread = thread::spawn(move || {
        let client =
            open_pipe_client(&client_name, Instant::now() + Duration::from_secs(2)).unwrap();
        release_rx.recv().unwrap();
        drop(client);
    });

    server
        .connect(Instant::now() + Duration::from_secs(2))
        .unwrap();
    let mut stream = PipeFrameStream::from_server(server, FrameLimits::default());
    assert!(matches!(
        stream.read_frame(Instant::now() + Duration::from_millis(50)),
        Err(TransportError::Timeout("ReadFile"))
    ));

    release_tx.send(()).unwrap();
    client_thread.join().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn disconnected_client_is_reported_distinctly() {
    let (root, name, security) = setup("disconnect");
    let mut server = create_first_pipe_server(&name, &security).unwrap();
    let client_name = name.clone();
    let client_thread = thread::spawn(move || {
        let client =
            open_pipe_client(&client_name, Instant::now() + Duration::from_secs(2)).unwrap();
        drop(client);
    });

    server
        .connect(Instant::now() + Duration::from_secs(2))
        .unwrap();
    client_thread.join().unwrap();
    let mut stream = PipeFrameStream::from_server(server, FrameLimits::default());
    assert_eq!(
        stream.read_frame(Instant::now() + Duration::from_secs(1)),
        Err(TransportError::Disconnected)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn opening_nonexistent_pipe_is_bounded() {
    let (root, name, _security) = setup("open-timeout");
    let started = Instant::now();
    assert!(matches!(
        open_pipe_client(&name, started + Duration::from_millis(40)),
        Err(TransportError::Timeout("open named-pipe client"))
    ));
    assert!(started.elapsed() < Duration::from_secs(1));
    fs::remove_dir_all(root).unwrap();
}

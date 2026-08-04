use std::{collections::BTreeSet, time::Instant};

use pastral_ipc_auth::{
    AuthenticationProof, HandshakeTranscript, Nonce, NonceReplayCache, PeerTranscriptIdentity,
    ProofRole, compute_proof, verify_proof,
};
use pastral_ipc_core::{
    Capability, ClientHelloDto, CorrelationId, Frame, FrameHeader, FrameKind, FrameLimits,
    ServerAcceptedDto, ServerHelloDto,
};
use pastral_ipc_schema::{
    decode_client_hello, decode_server_accepted, decode_server_hello, encode_client_hello,
    encode_server_accepted, encode_server_hello, schema_sha256,
};

use crate::{
    PipeFrameStream, TransportError, TransportMaterial, ValidatedPeer, current_token_identity,
    random_bytes,
};

const PROTOCOL_MAJOR: u32 = 1;
const MIN_MINOR: u32 = 0;
const MAX_MINOR: u32 = 0;
const SELECTED_MINOR: u32 = 0;
const HEALTH_CAPABILITIES: [Capability; 1] = [Capability::Health];

pub struct AuthenticatedServerConnection {
    stream: PipeFrameStream,
    peer: ValidatedPeer,
    selected_minor: u32,
    capabilities: Vec<Capability>,
}

impl AuthenticatedServerConnection {
    #[must_use]
    pub const fn peer(&self) -> &ValidatedPeer {
        &self.peer
    }

    #[must_use]
    pub const fn selected_minor(&self) -> u32 {
        self.selected_minor
    }

    #[must_use]
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    #[must_use]
    pub fn into_stream(self) -> PipeFrameStream {
        self.stream
    }
}

pub struct AuthenticatedClientConnection {
    stream: PipeFrameStream,
    peer: ValidatedPeer,
    selected_minor: u32,
    capabilities: Vec<Capability>,
}

impl AuthenticatedClientConnection {
    #[must_use]
    pub const fn peer(&self) -> &ValidatedPeer {
        &self.peer
    }

    #[must_use]
    pub const fn selected_minor(&self) -> u32 {
        self.selected_minor
    }

    #[must_use]
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    #[must_use]
    pub fn into_stream(self) -> PipeFrameStream {
        self.stream
    }
}

pub fn server_handshake(
    stream: PipeFrameStream,
    material: &TransportMaterial,
    peer: ValidatedPeer,
    replay_cache: &mut NonceReplayCache,
    deadline: Instant,
) -> Result<AuthenticatedServerConnection, TransportError> {
    server_handshake_with_capabilities(
        stream,
        material,
        peer,
        replay_cache,
        &HEALTH_CAPABILITIES,
        deadline,
    )
}

pub fn server_handshake_with_capabilities(
    stream: PipeFrameStream,
    material: &TransportMaterial,
    peer: ValidatedPeer,
    replay_cache: &mut NonceReplayCache,
    capabilities: &[Capability],
    deadline: Instant,
) -> Result<AuthenticatedServerConnection, TransportError> {
    let capabilities = normalize_capabilities(capabilities)?;
    server_handshake_with_nonce(
        stream,
        material,
        peer,
        replay_cache,
        random_nonce()?,
        &capabilities,
        deadline,
    )
}

#[doc(hidden)]
pub fn server_handshake_with_nonce_for_test(
    stream: PipeFrameStream,
    material: &TransportMaterial,
    peer: ValidatedPeer,
    replay_cache: &mut NonceReplayCache,
    server_nonce: Nonce,
    deadline: Instant,
) -> Result<AuthenticatedServerConnection, TransportError> {
    server_handshake_with_nonce(
        stream,
        material,
        peer,
        replay_cache,
        server_nonce,
        &HEALTH_CAPABILITIES,
        deadline,
    )
}

pub fn client_handshake(
    stream: PipeFrameStream,
    material: &TransportMaterial,
    peer: ValidatedPeer,
    deadline: Instant,
) -> Result<AuthenticatedClientConnection, TransportError> {
    client_handshake_with_capabilities(stream, material, peer, &HEALTH_CAPABILITIES, deadline)
}

pub fn client_handshake_with_capabilities(
    stream: PipeFrameStream,
    material: &TransportMaterial,
    peer: ValidatedPeer,
    capabilities: &[Capability],
    deadline: Instant,
) -> Result<AuthenticatedClientConnection, TransportError> {
    let capabilities = normalize_capabilities(capabilities)?;
    client_handshake_with_nonce(
        stream,
        material,
        peer,
        random_nonce()?,
        &capabilities,
        deadline,
    )
}

#[doc(hidden)]
pub fn client_handshake_with_nonce_for_test(
    stream: PipeFrameStream,
    material: &TransportMaterial,
    peer: ValidatedPeer,
    client_nonce: Nonce,
    deadline: Instant,
) -> Result<AuthenticatedClientConnection, TransportError> {
    client_handshake_with_nonce(
        stream,
        material,
        peer,
        client_nonce,
        &HEALTH_CAPABILITIES,
        deadline,
    )
}

fn server_handshake_with_nonce(
    mut stream: PipeFrameStream,
    material: &TransportMaterial,
    peer: ValidatedPeer,
    replay_cache: &mut NonceReplayCache,
    server_nonce: Nonce,
    capabilities: &[Capability],
    deadline: Instant,
) -> Result<AuthenticatedServerConnection, TransportError> {
    let hello = ServerHelloDto::new(
        PROTOCOL_MAJOR,
        MIN_MINOR,
        MAX_MINOR,
        *server_nonce.as_bytes(),
        material.identity().instance_id(),
        capabilities.iter().copied(),
    )
    .map_err(|_| TransportError::Protocol("server hello DTO is invalid"))?;
    let body = encode_server_hello(&hello)
        .map_err(|_| TransportError::Protocol("server hello encoding failed"))?;
    stream.write_frame(
        &hello_frame(body, CorrelationId::initial_server_hello())?,
        deadline,
    )?;

    let client_frame = stream.read_frame(deadline)?;
    if client_frame.header().kind() != FrameKind::HelloProto {
        return Err(TransportError::Protocol("expected client hello frame"));
    }
    let client = decode_client_hello(client_frame.body())
        .map_err(|_| TransportError::Protocol("client hello decoding failed"))?;
    if client.protocol_major() != PROTOCOL_MAJOR
        || client.minor_range() != (MIN_MINOR, MAX_MINOR)
        || client.echoed_server_nonce() != server_nonce.as_bytes()
    {
        return Err(TransportError::Protocol(
            "client hello negotiation mismatch",
        ));
    }
    if !capabilities_are_subset(client.capabilities(), capabilities) {
        return Err(TransportError::Protocol(
            "client requested unsupported capabilities",
        ));
    }
    let client_nonce =
        Nonce::from_bytes(*client.client_nonce()).map_err(TransportError::Authentication)?;
    let transcript = transcript(
        material,
        &server_nonce,
        &client_nonce,
        &peer,
        LocalRole::Server,
        capabilities,
        client.capabilities(),
    )?;
    let proof = AuthenticationProof::from_bytes(*client.authentication_proof());
    verify_proof(material.secret(), &transcript, ProofRole::Client, &proof)
        .map_err(TransportError::Authentication)?;
    replay_cache
        .record(
            &server_nonce,
            &client_nonce,
            peer.process_id(),
            peer.session_id(),
        )
        .map_err(TransportError::Authentication)?;

    let server_proof = compute_proof(material.secret(), &transcript, ProofRole::Server);
    let accepted = ServerAcceptedDto::new(
        SELECTED_MINOR,
        client.capabilities().iter().copied(),
        *server_proof.as_bytes(),
    )
    .map_err(|_| TransportError::Protocol("server accepted DTO is invalid"))?;
    let accepted_body = encode_server_accepted(&accepted)
        .map_err(|_| TransportError::Protocol("server accepted encoding failed"))?;
    stream.write_frame(
        &hello_frame(accepted_body, client_frame.header().correlation())?,
        deadline,
    )?;

    Ok(AuthenticatedServerConnection {
        stream,
        peer,
        selected_minor: SELECTED_MINOR,
        capabilities: client.capabilities().to_vec(),
    })
}

fn client_handshake_with_nonce(
    mut stream: PipeFrameStream,
    material: &TransportMaterial,
    peer: ValidatedPeer,
    client_nonce: Nonce,
    capabilities: &[Capability],
    deadline: Instant,
) -> Result<AuthenticatedClientConnection, TransportError> {
    let server_frame = stream.read_frame(deadline)?;
    if server_frame.header().kind() != FrameKind::HelloProto
        || !server_frame.header().correlation().is_zero()
    {
        return Err(TransportError::Protocol(
            "expected initial server hello frame",
        ));
    }
    let server = decode_server_hello(server_frame.body())
        .map_err(|_| TransportError::Protocol("server hello decoding failed"))?;
    if server.protocol_major() != PROTOCOL_MAJOR
        || server.minor_range() != (MIN_MINOR, MAX_MINOR)
        || !capabilities_are_subset(capabilities, server.capabilities())
        || server.instance_id() != material.identity().instance_id()
    {
        return Err(TransportError::Protocol(
            "server hello negotiation mismatch",
        ));
    }
    let server_nonce =
        Nonce::from_bytes(*server.server_nonce()).map_err(TransportError::Authentication)?;
    let transcript = transcript(
        material,
        &server_nonce,
        &client_nonce,
        &peer,
        LocalRole::Client,
        server.capabilities(),
        capabilities,
    )?;
    let client_proof = compute_proof(material.secret(), &transcript, ProofRole::Client);
    let client = ClientHelloDto::new(
        PROTOCOL_MAJOR,
        MIN_MINOR,
        MAX_MINOR,
        *client_nonce.as_bytes(),
        *server_nonce.as_bytes(),
        capabilities.iter().copied(),
        *client_proof.as_bytes(),
    )
    .map_err(|_| TransportError::Protocol("client hello DTO is invalid"))?;
    let correlation = CorrelationId::new_v4();
    let body = encode_client_hello(&client)
        .map_err(|_| TransportError::Protocol("client hello encoding failed"))?;
    stream.write_frame(&hello_frame(body, correlation)?, deadline)?;

    let accepted_frame = stream.read_frame(deadline)?;
    if accepted_frame.header().kind() != FrameKind::HelloProto
        || accepted_frame.header().correlation() != correlation
    {
        return Err(TransportError::Protocol("expected server accepted frame"));
    }
    let accepted = decode_server_accepted(accepted_frame.body())
        .map_err(|_| TransportError::Protocol("server accepted decoding failed"))?;
    if accepted.selected_minor() != SELECTED_MINOR
        || accepted.accepted_capabilities() != capabilities
    {
        return Err(TransportError::Protocol(
            "server accepted negotiation mismatch",
        ));
    }
    let server_proof = AuthenticationProof::from_bytes(*accepted.authentication_proof());
    verify_proof(
        material.secret(),
        &transcript,
        ProofRole::Server,
        &server_proof,
    )
    .map_err(TransportError::Authentication)?;

    Ok(AuthenticatedClientConnection {
        stream,
        peer,
        selected_minor: SELECTED_MINOR,
        capabilities: capabilities.to_vec(),
    })
}

#[derive(Clone, Copy)]
enum LocalRole {
    Server,
    Client,
}

fn transcript(
    material: &TransportMaterial,
    server_nonce: &Nonce,
    client_nonce: &Nonce,
    peer: &ValidatedPeer,
    local_role: LocalRole,
    requested_capabilities: &[Capability],
    accepted_capabilities: &[Capability],
) -> Result<HandshakeTranscript, TransportError> {
    let current = current_token_identity()?;
    let current_identity = PeerTranscriptIdentity::new(
        current.process_id(),
        current.session_id(),
        current.integrity_rid(),
    );
    let peer_identity =
        PeerTranscriptIdentity::new(peer.process_id(), peer.session_id(), peer.integrity_rid());
    let (server_identity, client_identity) = match local_role {
        LocalRole::Server => (current_identity, peer_identity),
        LocalRole::Client => (peer_identity, current_identity),
    };
    let server_identity = server_identity.map_err(TransportError::Authentication)?;
    let client_identity = client_identity.map_err(TransportError::Authentication)?;
    HandshakeTranscript::new(
        schema_sha256(),
        PROTOCOL_MAJOR,
        MIN_MINOR,
        MAX_MINOR,
        MIN_MINOR,
        MAX_MINOR,
        SELECTED_MINOR,
        *server_nonce,
        *client_nonce,
        material.identity().instance_id(),
        server_identity,
        client_identity,
        requested_capabilities.iter().copied(),
        accepted_capabilities.iter().copied(),
    )
    .map_err(TransportError::Authentication)
}

fn capabilities_are_subset(requested: &[Capability], supported: &[Capability]) -> bool {
    requested
        .iter()
        .all(|capability| supported.contains(capability))
}

fn normalize_capabilities(capabilities: &[Capability]) -> Result<Vec<Capability>, TransportError> {
    let unique = capabilities.iter().copied().collect::<BTreeSet<_>>();
    if unique.is_empty() || unique.len() != capabilities.len() {
        return Err(TransportError::Protocol("capability set is invalid"));
    }
    Ok(unique.into_iter().collect())
}

fn random_nonce() -> Result<Nonce, TransportError> {
    for _ in 0..4 {
        if let Ok(nonce) = Nonce::from_bytes(random_bytes::<32>()?) {
            return Ok(nonce);
        }
    }
    Err(TransportError::Authentication(
        pastral_ipc_auth::AuthError::InvalidNonce,
    ))
}

fn hello_frame(body: Vec<u8>, correlation: CorrelationId) -> Result<Frame, TransportError> {
    let limits = FrameLimits::default();
    let header = FrameHeader::new(
        FrameKind::HelloProto,
        u32::try_from(body.len())
            .map_err(|_| TransportError::SizeLimit("hello body exceeds u32"))?,
        0,
        correlation,
        limits,
    )
    .map_err(|_| TransportError::Protocol("hello frame header is invalid"))?;
    Frame::new(header, body).map_err(|_| TransportError::Protocol("hello frame is invalid"))
}

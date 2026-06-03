//! Full peer mesh for a squad — manages all peer connections for one local player.

use std::collections::HashMap;

use crate::{
    error::WebRtcError,
    peer::{PeerConnection, PeerRole},
};

/// Manages all WebRTC peer connections for a single local player.
///
/// For a 4-player squad, the local player maintains 3 connections:
/// one to their duo partner and two to the other duo's members.
pub struct PeerMesh {
    /// Map from remote user ID → peer connection.
    peers: HashMap<String, PeerConnection>,
}

impl PeerMesh {
    /// Create an empty mesh.
    pub fn new() -> Self {
        Self { peers: HashMap::new() }
    }

    /// Add a peer connection to the mesh.
    pub fn add_peer(
        &mut self,
        remote_id: impl Into<String>,
        role: PeerRole,
    ) -> Result<tokio::sync::mpsc::Receiver<Vec<f32>>, WebRtcError> {
        let id = remote_id.into();
        let (conn, audio_rx) = PeerConnection::new(id.clone(), role)?;
        self.peers.insert(id, conn);
        Ok(audio_rx)
    }

    /// Returns a reference to a peer connection by remote user ID.
    pub fn get(&self, remote_id: &str) -> Option<&PeerConnection> {
        self.peers.get(remote_id)
    }

    /// Returns a mutable reference to a peer connection by remote user ID.
    pub fn get_mut(&mut self, remote_id: &str) -> Option<&mut PeerConnection> {
        self.peers.get_mut(remote_id)
    }

    /// Returns the number of active peer connections.
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    /// Returns true if there are no active peer connections.
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }
}

impl Default for PeerMesh {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_add_and_count() {
        let mut mesh = PeerMesh::new();
        assert!(mesh.is_empty());

        let _rx = mesh.add_peer("@alice:example.org", PeerRole::Offerer).unwrap();
        let _rx = mesh.add_peer("@bob:example.org", PeerRole::Answerer).unwrap();

        assert_eq!(mesh.len(), 2);
        assert!(mesh.get("@alice:example.org").is_some());
        assert!(mesh.get("@unknown:example.org").is_none());
    }
}

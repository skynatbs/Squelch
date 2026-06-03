/// Matrix event types used for WebRTC signaling.
pub const SDP_OFFER: &str = "io.squelch.sdp_offer";
pub const SDP_ANSWER: &str = "io.squelch.sdp_answer";
pub const ICE_CANDIDATE: &str = "io.squelch.ice_candidate";
pub const CALL_MEMBER: &str = "io.squelch.call_member";

/// Sent by the squad leader to all members to initiate collective room cleanup.
/// All clients leave the Matrix room and clear their local room ID on receipt.
/// Only accepted from the current leader — clients ignore it from non-leaders.
pub const DISBAND: &str = "io.squelch.disband";

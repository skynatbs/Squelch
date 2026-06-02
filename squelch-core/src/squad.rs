//! Squad model — members, duos, leader assignment.

use serde::{Deserialize, Serialize};

/// Unique identifier for a squad member (their Matrix user ID).
pub type MemberId = String;

/// A duo is a pair of members who always hear each other (open mic).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Duo {
    pub members: [MemberId; 2],
}

/// Role of a member within the squad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Regular player — access to duo channel only.
    Member,
    /// Squad leader — access to duo channel + leader net (PTT).
    Leader,
}

/// A single squad member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub id:   MemberId,
    pub role: Role,
}

/// The full squad state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Squad {
    pub members: Vec<Member>,
    pub duos:    Vec<Duo>,
}

impl Squad {
    /// Returns true if the given member is a leader.
    pub fn is_leader(&self, id: &str) -> bool {
        self.members.iter().any(|m| m.id == id && m.role == Role::Leader)
    }

    /// Returns the duo partner of a member, if any.
    pub fn duo_partner<'a>(&'a self, id: &str) -> Option<&'a MemberId> {
        for duo in &self.duos {
            if duo.members[0] == id { return Some(&duo.members[1]); }
            if duo.members[1] == id { return Some(&duo.members[0]); }
        }
        None
    }

    /// Transfer leadership from one member to another.
    /// Returns false if `from` is not currently a leader.
    pub fn transfer_leadership(&mut self, from: &str, to: &str) -> bool {
        let is_leader = self.members.iter().any(|m| m.id == from && m.role == Role::Leader);
        if !is_leader { return false; }
        for m in &mut self.members {
            if m.id == from { m.role = Role::Member; }
            if m.id == to   { m.role = Role::Leader; }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_squad() -> Squad {
        Squad {
            members: vec![
                Member { id: "alice".into(), role: Role::Leader },
                Member { id: "bob".into(),   role: Role::Member },
                Member { id: "carol".into(), role: Role::Leader },
                Member { id: "dave".into(),  role: Role::Member },
            ],
            duos: vec![
                Duo { members: ["alice".into(), "bob".into()] },
                Duo { members: ["carol".into(), "dave".into()] },
            ],
        }
    }

    #[test]
    fn leader_detection() {
        let s = make_squad();
        assert!(s.is_leader("alice"));
        assert!(s.is_leader("carol"));
        assert!(!s.is_leader("bob"));
        assert!(!s.is_leader("dave"));
    }

    #[test]
    fn duo_partner() {
        let s = make_squad();
        assert_eq!(s.duo_partner("alice"), Some(&"bob".to_owned()));
        assert_eq!(s.duo_partner("bob"),   Some(&"alice".to_owned()));
        assert_eq!(s.duo_partner("carol"), Some(&"dave".to_owned()));
    }

    #[test]
    fn transfer_leadership() {
        let mut s = make_squad();
        assert!(s.transfer_leadership("alice", "bob"));
        assert!(!s.is_leader("alice"));
        assert!(s.is_leader("bob"));
    }

    #[test]
    fn transfer_leadership_not_leader_fails() {
        let mut s = make_squad();
        assert!(!s.transfer_leadership("dave", "bob")); // dave is not a leader
    }
}

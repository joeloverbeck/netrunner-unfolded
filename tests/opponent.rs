use netrunner_unfolded::{
    game::Player,
    play::{Play, SaveOptions},
    policies::Policy,
};
use std::time::Duration;

#[test]
fn opponent_status_is_pure_and_cancelled_or_stale_proposals_cannot_commit() {
    let mut play = Play::introductory_seeded(42, None, Player::Runner);
    play.admit();
    assert_eq!(play.opponent_policy(), Policy::CorpV1);
    let before = play.observation();
    for _ in 0..10 {
        assert!(play.opponent_pending());
    }
    assert_eq!(play.observation(), before);
    let proposal = play
        .prepare_opponent(Duration::from_secs(1))
        .unwrap()
        .unwrap()
        .compute();
    play.cancel_opponent();
    assert!(
        play.commit_opponent(proposal)
            .unwrap_err()
            .contains("stale or cancelled")
    );
    assert_eq!(play.observation(), before);
    let proposal = play
        .prepare_opponent(Duration::from_secs(1))
        .unwrap()
        .unwrap()
        .compute();
    let duplicate = proposal.clone();
    play.commit_opponent(proposal).unwrap();
    let committed = play.observation();
    assert!(play.commit_opponent(duplicate).is_err());
    assert_eq!(play.observation(), committed);
}

#[test]
fn failed_save_preserves_game_and_policy_and_reopen_keeps_the_selected_opponent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.json");
    let options = SaveOptions {
        path: path.clone(),
        recover_from: None,
    };
    let mut play = Play::introductory_seeded(42, Some(options.clone()), Player::Runner);
    play.admit();
    let proposal = play
        .prepare_opponent(Duration::from_secs(1))
        .unwrap()
        .unwrap()
        .compute();
    let before = play.observation();
    let bytes = std::fs::read(&path).unwrap();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(dir.path().join("game.json.lock"))
        .unwrap();
    lock.lock().unwrap();
    assert!(play.commit_opponent(proposal).unwrap_err().contains("busy"));
    assert_eq!(play.observation(), before);
    assert_eq!(play.opponent_policy(), Policy::CorpV1);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    drop(lock);
    drop(play);
    let mut reopened = Play::introductory_seeded(999, Some(options), Player::Corp);
    reopened.admit();
    assert_eq!(reopened.opponent_policy(), Policy::CorpV1);
    assert_eq!(reopened.observation(), before);
    reopened.respond().unwrap();
    assert_ne!(reopened.observation(), before);
}

#[test]
fn proposal_is_bound_to_its_match_and_zero_budget_fallback_is_observable() {
    use netrunner_unfolded::play::OpponentOutcome;
    let mut play = Play::introductory_seeded(3, None, Player::Runner);
    let mut other = Play::introductory_seeded(42, None, Player::Runner);
    play.admit();
    other.admit();
    let proposal = play
        .prepare_opponent(Duration::ZERO)
        .unwrap()
        .unwrap()
        .compute();
    assert_eq!(proposal.report().outcome, OpponentOutcome::TimeoutFallback);
    assert_eq!(proposal.report().selector_invocations, Some(0));
    let before = other.observation();
    assert!(other.commit_opponent(proposal.clone()).is_err());
    assert_eq!(other.observation(), before);
    play.commit_opponent(proposal).unwrap();
    assert!(play.opponent_report().unwrap().committed);
}

#[test]
fn native_poll_returns_promptly_and_commits_the_same_policy_as_direct_evaluation() {
    let mut asynchronous = Play::introductory_seeded(42, None, Player::Runner);
    let mut direct = Play::introductory_seeded(42, None, Player::Runner);
    asynchronous.admit();
    direct.admit();
    let start = std::time::Instant::now();
    asynchronous.poll_opponent(true).unwrap();
    assert!(start.elapsed() < Duration::from_secs(1));
    let before = asynchronous.observation();
    asynchronous.cancel_opponent();
    asynchronous.poll_opponent(false).unwrap();
    assert_eq!(asynchronous.observation(), before);
    assert_eq!(
        asynchronous.opponent_report().unwrap().outcome,
        netrunner_unfolded::play::OpponentOutcome::Cancelled
    );
    asynchronous.poll_opponent(true).unwrap();
    while asynchronous.opponent_pending() && start.elapsed() < Duration::from_secs(2) {
        asynchronous.poll_opponent(false).unwrap();
        std::thread::yield_now();
    }
    direct.respond().unwrap();
    assert_eq!(asynchronous.observation(), direct.observation());
    assert_eq!(
        asynchronous.opponent_report().unwrap().selector_invocations,
        Some(1)
    );
    assert!(asynchronous.opponent_report().unwrap().committed);
}

fn save_json(path: &std::path::Path, mut value: serde_json::Value) {
    use sha2::{Digest, Sha256};
    value["checksum"] = serde_json::json!(format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(
                value["format"].clone(),
                value["kind"].clone(),
                value["engine"].clone(),
                value["content"].clone(),
                value["payload"].clone()
            ))
            .unwrap()
        )
    ));
    std::fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
}

#[test]
fn legacy_introductory_save_keeps_the_frozen_opponent_when_migrated() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.json");
    let options = SaveOptions {
        path: path.clone(),
        recover_from: None,
    };
    let mut play = Play::introductory_seeded(42, Some(options.clone()), Player::Runner);
    play.admit();
    drop(play);
    // The v1 payload had only the human seat and match; no opponent identity.
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["engine"] = serde_json::json!("introductory-match-1");
    value["payload"].as_object_mut().unwrap().remove("opponent");
    save_json(&path, value);
    let mut reopened = Play::introductory_seeded(999, Some(options.clone()), Player::Corp);
    reopened.admit();
    assert_eq!(reopened.opponent_policy(), Policy::CorpV0);
    reopened.respond().unwrap();
    let before = reopened.observation();
    drop(reopened);
    let migrated: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(migrated["engine"], "introductory-match-2");
    assert_eq!(
        migrated["payload"]["opponent"]["identity"],
        Policy::CorpV0.identity()
    );
    let mut reopened = Play::introductory_seeded(999, Some(options), Player::Corp);
    reopened.admit();
    assert_eq!(reopened.opponent_policy(), Policy::CorpV0);
    assert_eq!(reopened.observation(), before);
}

#[test]
fn unknown_policy_identity_refuses_without_changing_the_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.json");
    let options = SaveOptions {
        path: path.clone(),
        recover_from: None,
    };
    let mut play = Play::introductory_seeded(42, Some(options.clone()), Player::Runner);
    play.admit();
    drop(play);
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["payload"]["opponent"]["identity"] = serde_json::json!("different policy");
    save_json(&path, value);
    let before = std::fs::read(&path).unwrap();
    let mut reopened = Play::introductory_seeded(999, Some(options), Player::Corp);
    reopened.admit();
    assert!(
        matches!(reopened.admission(),netrunner_unfolded::play::Admission::Refused(reason) if reason.contains("policy"))
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

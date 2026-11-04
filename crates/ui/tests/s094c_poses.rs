//! S-094c pose regressions, updated by S-137 tester before implementation.
//! The old seven exact poses taught dropping on a highlighted action while
//! dragging. S-137 AC2 supersedes that sequence, but preserves pose diversity.
use kawaro_ui::film::{self, POSES};

#[test]
fn ac2_poses_require_a_distinct_post_drop_choice_before_result() {
    let drag = POSES.iter().position(|p| p.thumbnail).expect("drag");
    let awaiting = POSES
        .iter()
        .position(|p| p.step == 3 && p.hive && !p.thumbnail && !p.hot && !p.pressed && !p.result)
        .expect("unchosen actions after dropping");
    let click = POSES
        .iter()
        .position(|p| p.pressed && !p.thumbnail)
        .expect("post-drop click");
    let result = POSES.iter().position(|p| p.result).expect("result");
    assert!(drag < awaiting && awaiting < click && click < result);
    for p in &POSES {
        if p.thumbnail {
            assert!(!p.hot && !p.pressed && !p.result);
        }
    }
}

#[test]
fn no_two_neighbouring_poses_are_equal_and_the_poster_is_the_result() {
    for i in 1..POSES.len() {
        assert_ne!(POSES[i - 1], POSES[i], "pose {i} repeats pose {}", i - 1);
    }
    let poster = POSES[film::poster_index()];
    assert!(poster.result && !poster.thumbnail && !poster.pressed);
    assert_eq!(film::frames().len(), POSES.len());
}

#[test]
fn the_caption_rows_lit_per_step() {
    // S-137: drop hint and choosing an action are separate instructions.
    for step in 0..5 {
        assert_eq!(film::active_captions(step), &[step]);
    }
}

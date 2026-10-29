//! S-094c: pins every field of every pose of the film loop, so a pose can
//! neither lose a field nor become a duplicate of its neighbour.

use kawaro_ui::film::{self, POSES, Pose};

#[allow(clippy::too_many_arguments)]
fn pose(
    step: usize,
    selected: bool,
    pointer: bool,
    approach: f32,
    lift: f32,
    hive: bool,
    thumbnail: bool,
    drag: f32,
    hot: bool,
    pressed: bool,
    result: bool,
) -> Pose {
    Pose {
        step,
        selected,
        pointer,
        approach,
        lift,
        hive,
        thumbnail,
        drag,
        hot,
        pressed,
        result,
    }
}

#[test]
fn every_pose_of_the_loop_is_pinned() {
    let (t, f) = (true, false);
    let want = [
        pose(0, f, t, 0.0, 0.0, f, f, 0.0, f, f, f),
        pose(0, t, t, 1.0, 0.0, f, f, 0.0, f, f, f),
        pose(1, t, t, 1.0, 1.0, t, f, 0.0, f, f, f),
        pose(2, t, t, 1.0, 1.0, t, t, 0.5, f, f, f),
        pose(2, t, t, 1.0, 1.0, t, t, 1.0, t, f, f),
        pose(2, t, t, 1.0, 1.0, t, t, 1.0, t, t, f),
        pose(3, t, f, 1.0, 0.0, f, f, 0.0, f, f, t),
    ];
    assert_eq!(POSES.len(), want.len());
    for (i, w) in want.iter().enumerate() {
        assert_eq!(&POSES[i], w, "pose {i}");
    }
}

#[test]
fn no_two_neighbouring_poses_are_equal_and_the_poster_is_the_result() {
    for i in 1..POSES.len() {
        assert_ne!(POSES[i - 1], POSES[i], "pose {i} repeats pose {}", i - 1);
    }
    assert_eq!(film::poster_index(), 6);
    assert_eq!(film::frames().len(), POSES.len());
}

#[test]
fn the_caption_rows_lit_per_step() {
    assert_eq!(film::active_captions(0), &[0]);
    assert_eq!(film::active_captions(1), &[1]);
    assert_eq!(film::active_captions(2), &[2, 3]);
    assert_eq!(film::active_captions(3), &[4]);
}

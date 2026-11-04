//! The Introduction film (S-094c): the one scene model behind the live
//! vector film of the welcome window. `ui/film.slint` draws the scene from a
//! [`Pose`]; this module owns the poses, so the film the user sees and the
//! frame table the tests read are the same data.
//!
//! The film teaches the default drop-first flow: select/drag (0), trigger
//! (1), drop hint (2), post-drop choice and click (3), then result beside
//! the untouched original (4). The scene is
//! vector art drawn at the displayed size, so it is sharp at any window size
//! and scale; there are no raster frames.

/// The number of steps the film shows.
pub const STEPS: usize = 5;

/// How long one pose stays on screen while the film plays.
pub const FRAME_MS: u64 = 900;

/// One frame of the loop as the tests and the captions read it.
#[derive(Clone, Debug, PartialEq)]
pub struct FilmFrame {
    /// Step 0 to 4: select, trigger, drop hint, choice, result.
    pub step: usize,
    /// Opacity of the hive group; 1.0 whenever the hive is part of the step.
    pub hive_opacity: f32,
    /// The scene elements on screen: `file-manager`, `source-file`,
    /// `pointer`, `hive`, `thumbnail`, `result-file`.
    pub elements: Vec<String>,
}

/// What the scene draws in one frame. Positions are fractions of a path:
/// `approach` is the pointer arriving at the file, `lift` the pointer
/// pulling the file out to where the hive opens, `drag` the dragged file
/// travelling from there to the target cell.
#[derive(Clone, Copy, Debug, PartialEq)]
#[doc(hidden)]
pub struct Pose {
    pub step: usize,
    pub selected: bool,
    pub pointer: bool,
    pub approach: f32,
    pub lift: f32,
    pub hive: bool,
    pub thumbnail: bool,
    pub drag: f32,
    pub hot: bool,
    pub pressed: bool,
    pub result: bool,
}

const REST: Pose = Pose {
    step: 0,
    selected: false,
    pointer: true,
    approach: 0.0,
    lift: 0.0,
    hive: false,
    thumbnail: false,
    drag: 0.0,
    hot: false,
    pressed: false,
    result: false,
};

/// The loop, one pose per frame; every step has its own distinct poses.
#[doc(hidden)]
pub const POSES: [Pose; 8] = [
    // 0: select the file in the file manager.
    REST,
    Pose {
        selected: true,
        approach: 1.0,
        ..REST
    },
    // 1: the trigger opens only the drop hint at the pointer.
    Pose {
        step: 1,
        selected: true,
        approach: 1.0,
        lift: 1.0,
        hive: true,
        ..REST
    },
    // 2: drag onto the drop area, with no action cells.
    Pose {
        step: 2,
        selected: true,
        approach: 1.0,
        lift: 1.0,
        hive: true,
        thumbnail: true,
        drag: 0.5,
        ..REST
    },
    Pose {
        step: 3,
        selected: true,
        approach: 1.0,
        lift: 1.0,
        hive: true,
        drag: 0.0,
        ..REST
    },
    Pose {
        step: 3,
        selected: true,
        approach: 1.0,
        lift: 1.0,
        hive: true,
        drag: 1.0,
        hot: true,
        ..REST
    },
    Pose {
        step: 3,
        selected: true,
        approach: 1.0,
        lift: 1.0,
        hive: true,
        drag: 1.0,
        hot: true,
        pressed: true,
        ..REST
    },
    // 4: the result beside the untouched original.
    Pose {
        step: 4,
        pointer: false,
        selected: true,
        approach: 1.0,
        result: true,
        ..REST
    },
];

/// Index of the poster: the first frame that shows the result.
pub fn poster_index() -> usize {
    POSES.iter().position(|p| p.step == STEPS - 1).unwrap_or(0)
}

/// The pose behind frame `i` (wrapping).
pub(crate) fn pose(i: usize) -> Pose {
    POSES[i % POSES.len()]
}

/// The whole loop as a frame table.
pub fn frames() -> Vec<FilmFrame> {
    POSES
        .iter()
        .map(|p| {
            let mut elements = vec!["file-manager", "source-file"];
            for (shown, name) in [
                (p.pointer, "pointer"),
                (p.hive, "hive"),
                (p.thumbnail, "thumbnail"),
                (p.result, "result-file"),
            ] {
                if shown {
                    elements.push(name);
                }
            }
            FilmFrame {
                step: p.step,
                hive_opacity: 1.0,
                elements: elements.into_iter().map(String::from).collect(),
            }
        })
        .collect()
}

/// The caption rows that belong to a step. The five captions of
/// `kawaro_core::welcome` are the flow: select, trigger, drop, choice, result.
pub fn active_captions(step: usize) -> &'static [usize] {
    match step {
        0 => &[0],
        1 => &[1],
        2 => &[2],
        3 => &[3],
        _ => &[4],
    }
}

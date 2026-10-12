//! THE TOOLS OF THE PART.
use std::f64::consts::PI;

use qymcad::{Key, Modifiers};

use crate::contract::fixtures::Fixture;
use crate::contract::{When, Class, Context, Entry, Field, Flow, Mode, Node, Outcome, Pick, Tool, Upstream, Words};

/// A 40 x 30 box of height `h` standing on XY from `z0`.
const fn box_outcome(z0: f64, h: f64) -> Outcome {
    Outcome::Body { volume: 1200.0 * h, faces: 6, edges: 12, min: [0.0, 0.0, z0], max: [40.0, 30.0, z0 + h] }
}

/// The 40 x 30 box extruded `h` high.
fn extruded(h: f64) -> Outcome {
    box_outcome(0.0, h)
}

/// EXTRUDE: the rectangle of the sketch made a body, 10 high by default.
pub static EXTRUDE: Tool = Tool {
    id: "part.extrude",
    flow: Flow::Command,
    title: "cmd-extrude",
    entries: &[Entry::Button("tb-extrude-hint"), Entry::Key(Modifiers::NONE, Key::E), Entry::Search("cmd-extrude")],
    other: (Entry::Button("tb-revolve-hint"), "cmd-revolve"),
    fixture: Fixture::RectangleSketch,
    picks: &[],
    pick_trial: &[Pick::Space([20.0, 15.0, 0.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "f-length", by_placeholder: false, when: When::Before, class: Class::Length, typical: 10.0, lo: 0.1, hi: 10000.0, zero: false, negative: false, outcome: extruded }],
    modes: &[
        &[
            Mode { word: "cmd-add", clicks: None, outcome: Some(box_outcome(0.0, 10.0)) },
            Mode { word: "cmd-cut", clicks: None, outcome: None },
            Mode { word: "cmd-intersect", clicks: None, outcome: None },
        ],
        &[
            Mode { word: "cmd-to-length", clicks: None, outcome: Some(box_outcome(0.0, 10.0)) },
            Mode { word: "cmd-symmetric", clicks: None, outcome: Some(box_outcome(-5.0, 10.0)) },
            Mode { word: "cmd-two-sides", clicks: None, outcome: Some(box_outcome(-0.1, 10.1)) },
        ],
        &[Mode { word: "cmd-flip", clicks: None, outcome: Some(box_outcome(-10.0, 10.0)) }],
    ],
    result: box_outcome(0.0, 10.0),
    node: "Extrude",
    undo: "f-extrusion",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::SketchDimension { shown: "40", value: 50.0, then: Outcome::Body { volume: 15000.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [50.0, 30.0, 10.0] } }),
    dependency: Some(Node { kind: "Sketch", row: None }),
    contexts: &[Context::SecondPart, Context::AfterTool(&FILLET)],
    refusal: None,
    budget: (30, 2000),
    help: "part/01-extrude",
    not_applicable: &[(17, "a length of the right form over a closed contour always builds a prism: there is nothing for the kernel to refuse")],
};

/// The block with the top front edge rounded to `r`: the edge's quarter of a square taken off, less the quarter disc.
fn filleted(r: f64) -> Outcome {
    Outcome::Body { volume: 12000.0 - 40.0 * r * r * (1.0 - PI / 4.0), faces: 7, edges: 15, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0] }
}

/// FILLET: the top front edge of the block rounded, 2 by default.
pub static FILLET: Tool = Tool {
    id: "part.fillet",
    flow: Flow::Command,
    title: "cmd-fillet",
    entries: &[Entry::Button("tb-fillet-body-hint"), Entry::Key(Modifiers::NONE, Key::F), Entry::Search("cmd-fillet")],
    other: (Entry::Button("tb-chamfer-body-hint"), "cmd-chamfer"),
    fixture: Fixture::Block,
    picks: &[Pick::Edge([20.0, 0.0, 10.0])],
    pick_trial: &[Pick::Edge([20.0, 0.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "f-radius", by_placeholder: false, when: When::Before, class: Class::Radius, typical: 2.0, lo: 0.05, hi: 9.5, zero: false, negative: false, outcome: filleted }],
    modes: &[],
    result: Outcome::Body { volume: 12000.0 - 160.0 * (1.0 - PI / 4.0), faces: 7, edges: 15, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0] },
    node: "Fillet",
    undo: "f-fillet",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Body { volume: 24000.0 - 160.0 * (1.0 - PI / 4.0), faces: 7, edges: 15, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 20.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart, Context::AfterTool(&EXTRUDE)],
    refusal: Some(12.0),
    budget: (30, 2000),
    help: "part/05-fillet",
    not_applicable: &[(2, "the fillet's bar has no modes: it takes edges and a radius")],
};

/// The block with the top front edge cut by a chamfer of leg `d`: the triangle d^2 / 2 taken off along all 40 mm.
const fn chamfered(d: f64) -> Outcome {
    Outcome::Body { volume: 12000.0 - 20.0 * d * d, faces: 7, edges: 15, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0] }
}

/// CHAMFER: the top front edge of the block cut, the leg 1.5 by default. Its two asymmetric modes start at the same
/// cut - a second leg of 1.5, or 1.5 and 45 degrees - so all three make the same body of the ordinary picks.
pub static CHAMFER: Tool = Tool {
    id: "part.chamfer",
    flow: Flow::Command,
    title: "cmd-chamfer",
    entries: &[Entry::Button("tb-chamfer-body-hint"), Entry::Key(Modifiers::NONE, Key::C), Entry::Search("cmd-chamfer")],
    other: (Entry::Button("tb-fillet-body-hint"), "cmd-fillet"),
    fixture: Fixture::Block,
    picks: &[Pick::Edge([20.0, 0.0, 10.0])],
    pick_trial: &[Pick::Edge([20.0, 0.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "f-leg", by_placeholder: false, when: When::Before, class: Class::Radius, typical: 1.5, lo: 0.05, hi: 9.5, zero: false, negative: false, outcome: chamfered }],
    modes: &[&[
        Mode { word: "cmd-symmetric", clicks: None, outcome: Some(chamfered(1.5)) },
        Mode { word: "cmd-two-distances", clicks: None, outcome: Some(chamfered(1.5)) },
        Mode { word: "cmd-leg-angle", clicks: None, outcome: Some(chamfered(1.5)) },
    ]],
    result: chamfered(1.5),
    node: "Chamfer",
    undo: "f-chamfer",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Body { volume: 24000.0 - 45.0, faces: 7, edges: 15, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 20.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart, Context::AfterTool(&EXTRUDE)],
    refusal: Some(12.0),
    budget: (30, 2000),
    help: "part/06-chamfer",
    not_applicable: &[],
};

/// The block with a hole of diameter `d` bored into the middle of its top: right through when it is 10 deep or more -
/// the wall alone - blind otherwise, which adds its floor. The wall is one cylindrical face closed by a seam, so it
/// brings three edges either way: the circle at each end and the seam between them.
const fn holed(d: f64, depth: f64) -> Outcome {
    let through = depth >= 10.0;
    let taken = PI * d * d / 4.0 * if through { 10.0 } else { depth };
    Outcome::Body { volume: 12000.0 - taken, faces: if through { 7 } else { 8 }, edges: 15, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0] }
}

fn hole_across(d: f64) -> Outcome {
    holed(d, 15.0)
}

fn hole_deep(depth: f64) -> Outcome {
    holed(6.0, depth)
}

/// HOLE: a hole 6 across bored square into the face clicked, 15 deep by default - right through a block 10 thick.
pub static HOLE: Tool = Tool {
    id: "part.hole",
    flow: Flow::Command,
    title: "cmd-hole",
    entries: &[Entry::Button("tb-hole-hint"), Entry::Key(Modifiers::NONE, Key::O), Entry::Search("cmd-hole")],
    other: (Entry::Button("tb-shell-hint"), "cmd-shell"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[
        Field { caption: "f-diameter", by_placeholder: false, when: When::Before, class: Class::Length, typical: 6.0, lo: 0.1, hi: 29.0, zero: false, negative: false, outcome: hole_across },
        Field { caption: "f-depth", by_placeholder: false, when: When::Before, class: Class::Length, typical: 15.0, lo: 0.1, hi: 10000.0, zero: false, negative: false, outcome: hole_deep },
    ],
    modes: &[
        // placement: by a face is the one the ordinary picks are made for; by a sketch must ask for a sketch in words
        &[Mode { word: "cmd-by-face", clicks: None, outcome: Some(holed(6.0, 15.0)) }, Mode { word: "cmd-by-sketch", clicks: None, outcome: None }],
        // kind: a recess 12 across and 4 deep opens over the hole - a flat shoulder for a counterbore, a cone for a
        // countersink; each takes what the hole of 6 has not taken there already
        &[
            Mode { word: "cmd-simple", clicks: None, outcome: Some(holed(6.0, 15.0)) },
            Mode {
                word: "cmd-counterbore",
                clicks: None,
                outcome: Some(Outcome::Body { volume: 12000.0 - PI * 9.0 * 10.0 - PI * (36.0 - 9.0) * 4.0, faces: 9, edges: 18, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0] }),
            },
            Mode {
                word: "cmd-countersink",
                clicks: None,
                outcome: Some(Outcome::Body {
                    volume: 12000.0 - PI * 9.0 * 10.0 - PI * 4.0 / 3.0 * (36.0 + 18.0 + 9.0) + PI * 9.0 * 4.0,
                    faces: 8,
                    edges: 17,
                    min: [0.0, 0.0, 0.0],
                    max: [40.0, 30.0, 10.0],
                }),
            },
        ],
    ],
    result: holed(6.0, 15.0),
    node: "Hole",
    undo: "f-operation",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Body { volume: 24000.0 - PI * 9.0 * 15.0, faces: 8, edges: 15, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 20.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart, Context::AfterTool(&EXTRUDE)],
    refusal: Some(60.0),
    budget: (30, 2000),
    help: "part/08-hole",
    not_applicable: &[],
};

/// The block with the wall `t` standing outside its surface, less `cavity` inside it: the five faces of the block
/// under the open top move out by `t` (5200 mm^2 of them at t = 1), the eight edges between them become quarter
/// cylinders and the four bottom corners eighths of a sphere. The faces meeting the open top neither move nor round.
const fn offset_wall(t: f64, cavity: f64) -> Outcome {
    let grown = 12000.0 + 2600.0 * t + 180.0 * PI * t * t / 4.0 + 4.0 * PI * t * t * t / 6.0;
    Outcome::Body { volume: grown - cavity, faces: 23, edges: 48, min: [-t, -t, -t], max: [40.0 + t, 30.0 + t, 10.0] }
}

/// The block hollowed to the wall `t`, open at its top: the cavity is a box of the wall inside it, and the top
/// becomes a rim. Five faces of the cavity stand beside the six of the block; the rim keeps its own inner loop.
const fn shelled(t: f64) -> Outcome {
    Outcome::Body { volume: 12000.0 - (40.0 - 2.0 * t) * (30.0 - 2.0 * t) * (10.0 - t), faces: 11, edges: 24, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0] }
}

/// SHELL: the block hollowed through its top, the wall 2 by default. The wall goes inwards from the surface, or
/// outwards, or half each way, which moves the box of the body accordingly.
pub static SHELL: Tool = Tool {
    id: "part.shell",
    flow: Flow::Command,
    title: "cmd-shell",
    entries: &[Entry::Button("tb-shell-hint"), Entry::Key(Modifiers::NONE, Key::H), Entry::Search("cmd-shell")],
    other: (Entry::Button("tb-hole-hint"), "cmd-hole"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "f-thickness", by_placeholder: false, when: When::Before, class: Class::Radius, typical: 2.0, lo: 0.1, hi: 9.5, zero: false, negative: false, outcome: shelled }],
    modes: &[&[
        Mode { word: "cmd-inwards", clicks: None, outcome: Some(shelled(2.0)) },
        // outwards the old surface becomes the cavity and the wall stands outside it, following the surface: each
        // face moves out by the wall, each edge between two of them becomes a quarter cylinder of that radius and
        // each corner an eighth of a sphere, so the body grows by area * t + length * pi t^2 / 4 + 4 * pi t^3 / 6.
        // The face left open neither moves nor rounds, so only the bottom corners round: 5200 + 180 pi + 16 pi / 3.
        Mode { word: "cmd-outwards", clicks: None, outcome: Some(offset_wall(2.0, 12000.0)) },
        // centred: half the wall outside the surface, half inside - the outer side rounds the same way, and the
        // cavity is the block short of half the wall on every face but the open one
        Mode { word: "cmd-centred", clicks: None, outcome: Some(offset_wall(1.0, 38.0 * 28.0 * 9.0)) },
    ]],
    result: shelled(2.0),
    node: "Shell",
    undo: "f-shell",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Body { volume: 24000.0 - 36.0 * 26.0 * 18.0, faces: 11, edges: 24, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 20.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart, Context::AfterTool(&EXTRUDE)],
    refusal: Some(10.5),
    budget: (30, 2000),
    help: "part/07-shell",
    not_applicable: &[],
};

/// The rectangle turned a whole turn about the X axis it stands on: a cylinder of radius 30 and length 40 - its
/// side, its two ends, and the seam with the circle at each end.
const fn whole_turn() -> Outcome {
    Outcome::Body { volume: PI * 900.0 * 40.0, faces: 3, edges: 3, min: [0.0, -30.0, -30.0], max: [40.0, 30.0, 30.0] }
}

/// The same turned by `angle`: part of a turn is a wedge of that cylinder, cut by a plane at each end of the sweep,
/// and those two planes meet on the axis.
fn turned(angle: f64) -> Outcome {
    if angle >= 360.0 {
        return whole_turn();
    }
    let (sin, cos) = (angle.to_radians().sin(), angle.to_radians().cos());
    let (y0, y1) = if angle > 90.0 { (30.0 * cos.min(0.0), 30.0) } else { (0.0, 30.0) };
    // past a quarter turn the far corner has gone over the top: the height is the whole 30
    let (z0, z1) = if angle > 180.0 {
        (-30.0, 30.0)
    } else if angle >= 90.0 {
        (0.0, 30.0)
    } else {
        (0.0, 30.0 * sin.max(0.0))
    };
    Outcome::Body { volume: PI * 900.0 * 40.0 * angle / 360.0, faces: 5, edges: 9, min: [0.0, y0, z0], max: [40.0, y1, z1] }
}

/// REVOLVE: the rectangle of the sketch turned about the axis it stands on, a whole turn by default.
pub static REVOLVE: Tool = Tool {
    id: "part.revolve",
    flow: Flow::Command,
    title: "cmd-revolve",
    entries: &[Entry::Button("tb-revolve-hint"), Entry::Key(Modifiers::NONE, Key::R), Entry::Search("cmd-revolve")],
    other: (Entry::Button("tb-extrude-hint"), "cmd-extrude"),
    fixture: Fixture::RectangleSketch,
    picks: &[],
    pick_trial: &[Pick::Space([20.0, 15.0, 0.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "f-angle", by_placeholder: false, when: When::Before, class: Class::Angle, typical: 360.0, lo: 1.0, hi: 360.0, zero: false, negative: false, outcome: turned }],
    modes: &[
        // a first body can only be added: there is nothing yet to cut from or to meet
        &[
            Mode { word: "cmd-add", clicks: None, outcome: Some(whole_turn()) },
            Mode { word: "cmd-cut", clicks: None, outcome: None },
            Mode { word: "cmd-intersect", clicks: None, outcome: None },
        ],
        // a whole turn comes out the same laid to one side, laid about the middle, or reversed
        &[Mode { word: "cmd-one-side", clicks: None, outcome: Some(whole_turn()) }, Mode { word: "cmd-symmetric", clicks: None, outcome: Some(whole_turn()) }],
        &[Mode { word: "cmd-flip-btn", clicks: None, outcome: Some(whole_turn()) }],
    ],
    result: whole_turn(),
    node: "Revolve",
    undo: "f-revolution",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::SketchDimension { shown: "40", value: 50.0, then: Outcome::Body { volume: PI * 900.0 * 50.0, faces: 3, edges: 3, min: [0.0, -30.0, -30.0], max: [50.0, 30.0, 30.0] } }),
    dependency: Some(Node { kind: "Sketch", row: None }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/02-revolve",
    not_applicable: &[(
        17,
        "every angle of 1 to 360 degrees turns a contour lying beside its axis into a body; what the kernel cannot build is a contour lying across the axis, which is a matter of what is drawn, not of the value typed",
    )],
};

/// The block and its reflection in its right face, joined along it: one block twice as long.
const fn doubled() -> Outcome {
    Outcome::Body { volume: 24000.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [80.0, 30.0, 10.0] }
}

/// MIRROR: the block reflected in its own right face. The copy stands against the face and is joined to the body
/// along it, so a block doubled is a block. It takes no value: the body is clicked, then the plane.
pub static MIRROR: Tool = Tool {
    id: "part.mirror",
    flow: Flow::Command,
    title: "cmd-mirror",
    entries: &[Entry::Button("tb-mirror-body-hint"), Entry::Key(Modifiers::NONE, Key::M), Entry::SearchByArticle],
    other: (Entry::Button("tb-lin-array-body-hint"), "cmd-linear-array"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0]), Pick::Face([40.0, 15.0, 5.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0]), Pick::Face([40.0, 15.0, 5.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    // the original is kept by default; the switch turned off leaves the reflection alone, standing where it was
    modes: &[&[Mode { word: "cmd-with-original", clicks: None, outcome: Some(Outcome::Body { volume: 12000.0, faces: 6, edges: 12, min: [40.0, 0.0, 0.0], max: [80.0, 30.0, 10.0] }) }]],
    result: doubled(),
    node: "Mirror",
    undo: "f-mirror",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Body { volume: 48000.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [80.0, 30.0, 20.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/16-mirror",
    not_applicable: &[
        (5, "the mirror takes no value: what it makes is settled by the body and the plane clicked"),
        (6, "the mirror takes no value, so there is none to refuse"),
        (17, "a body reflected in a plane always builds: there is no value for the kernel to fail on"),
    ],
};

/// A ROW of three copies of the block along X, `pitch` apart. Nearer than its 40 they overlap into one block as long
/// as the row; at 40 they touch; farther apart each copy is a body of its own.
fn row_of_three(pitch: f64) -> Outcome {
    if pitch > 40.0 {
        return Outcome::Pieces { pieces: 3, volume: 36000.0 };
    }
    let length = 40.0 + 2.0 * pitch;
    Outcome::Body { volume: length * 300.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [length, 30.0, 10.0] }
}

/// The same row of `copies` at the ordinary pitch of 25, which overlaps: one block 40 + 25 (copies - 1) long.
const fn row_of(copies: f64) -> Outcome {
    let length = 40.0 + 25.0 * (copies - 1.0);
    Outcome::Body { volume: length * 300.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [length, 30.0, 10.0] }
}

/// LINEAR PATTERN: three copies of the block along X, 25 apart - nearer than the block is long, so they overlap into
/// one block 90 long.
pub static LINEAR_ARRAY: Tool = Tool {
    id: "part.linear-array",
    flow: Flow::Command,
    title: "cmd-linear-array",
    entries: &[Entry::Button("tb-lin-array-body-hint"), Entry::SearchByArticle],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[
        Field { caption: "f-pitch", by_placeholder: false, when: When::Before, class: Class::Length, typical: 25.0, lo: 0.01, hi: 100000.0, zero: false, negative: false, outcome: row_of_three },
        Field { caption: "cmd-copies", by_placeholder: false, when: When::Before, class: Class::Count, typical: 3.0, lo: 2.0, hi: 100.0, zero: false, negative: false, outcome: row_of },
    ],
    modes: &[],
    result: row_of(3.0),
    node: "LinearArray",
    undo: "f-operation",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Body { volume: 90.0 * 30.0 * 20.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [90.0, 30.0, 20.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/17-linear-array",
    not_applicable: &[
        (2, "the bar of the pattern holds its count and the direction it goes, not modes that put each other down"),
        (17, "copies of a body laid along a line always build: what the kernel could not make of them is a matter of where they fall, not of the value typed"),
    ],
};

/// The block with its top pushed along its own normal by `offset`: out for more, in for less. The neighbours follow,
/// so a block stays a block.
const fn pushed(offset: f64) -> Outcome {
    Outcome::Body { volume: 1200.0 * (10.0 + offset), faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0 + offset] }
}

/// PUSH FACE: the top of the block moved 5 out. A minus moves it in; in by the whole 10 of the height there would be
/// no body left, which the kernel refuses.
pub static PUSH_FACE: Tool = Tool {
    id: "part.push-face",
    flow: Flow::Command,
    title: "f-push-face",
    entries: &[Entry::Button("tb-push-face-hint"), Entry::Search("f-push-face")],
    other: (Entry::Button("tb-fillet-body-hint"), "cmd-fillet"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "f-offset", by_placeholder: false, when: When::Before, class: Class::Length, typical: 5.0, lo: -9.5, hi: 100000.0, zero: false, negative: true, outcome: pushed }],
    modes: &[],
    result: pushed(5.0),
    node: "PushFace",
    undo: "f-push-face",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Body { volume: 1200.0 * 25.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 25.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: Some(-10.0),
    budget: (30, 2000),
    help: "part/11-push-face",
    not_applicable: &[(2, "the bar of push face holds no modes: it takes a flat face and an offset, plus or minus")],
};

/// The block with a plate of `thickness` grown on its top: the sides of the plate stand in the sides of the block, so
/// a block made taller keeps the six faces of a block.
const fn thickened(thickness: f64) -> Outcome {
    Outcome::Body { volume: 12000.0 + 1200.0 * thickness, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0 + thickness] }
}

/// THICKEN: the top of the block grown into a plate 2 thick, joined to the body - the part stays one body.
pub static THICKEN: Tool = Tool {
    id: "part.thicken",
    flow: Flow::Command,
    title: "feat-name-thicken",
    entries: &[Entry::Button("tb-thicken-hint"), Entry::Search("feat-name-thicken")],
    other: (Entry::Button("tb-fillet-body-hint"), "cmd-fillet"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "f-thickness", by_placeholder: false, when: When::Before, class: Class::Length, typical: 2.0, lo: 0.1, hi: 100000.0, zero: false, negative: false, outcome: thickened }],
    modes: &[],
    result: thickened(2.0),
    node: "Thicken",
    undo: "f-operation",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Body { volume: 24000.0 + 2400.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 22.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/15-thicken",
    not_applicable: &[
        (2, "the bar of thicken holds no modes: it takes a face and a thickness, and the plate grows outwards"),
        (17, "a plate grown on a face always builds: the thickness has no value the kernel cannot make of it"),
    ],
};

/// `copies` of the far block turned about the vertical axis. Seen from that axis the block is 5.7 degrees wide, so up
/// to 63 copies stand apart: each is a piece of its own, and they multiply the 40 x 20 x 10 block.
const fn ring_of(copies: f64) -> Outcome {
    Outcome::Pieces { pieces: copies as u32, volume: 8000.0 * copies }
}

/// CIRCULAR_ARRAY: six copies of the block turned the whole way about the vertical axis.
pub static CIRCULAR_ARRAY: Tool = Tool {
    id: "part.circular-array",
    flow: Flow::Command,
    title: "cmd-circular-array",
    entries: &[Entry::Button("tb-circ-array-body-hint"), Entry::SearchByArticle],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::BlockOffAxis,
    picks: &[Pick::Face([220.0, 0.0, 10.0])],
    pick_trial: &[Pick::Face([220.0, 0.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "cmd-copies", by_placeholder: false, when: When::Before, class: Class::Count, typical: 6.0, lo: 2.0, hi: 52.0, zero: false, negative: false, outcome: ring_of }],
    // the whole circle switched off puts an angle field at the geometry, and 180 degrees over six copies still leaves
    // them 30 degrees apart - the same six pieces, standing over half the turn
    modes: &[&[Mode { word: "cmd-full-circle", clicks: None, outcome: Some(ring_of(6.0)) }]],
    result: ring_of(6.0),
    node: "CircularArray",
    undo: "f-operation",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen { node: Node { kind: "Extrude", row: Some("cmd-extrude") }, caption: "f-length", value: 20.0, then: Outcome::Pieces { pieces: 6, volume: 96000.0 } }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/18-circular-array",
    not_applicable: &[(17, "copies of a body turned about an axis always build: what the kernel could not make of them is a matter of where they fall, not of the count typed")],
};

/// The block cut across by a plane `depth` below its top: two pieces of 1200 mm^2 by what is left of the 10 of
/// height, together the block itself.
const fn cut_at(depth: f64) -> Outcome {
    let (upper, lower) = (1200.0 * depth, 1200.0 * (10.0 - depth));
    Outcome::Bodies { count: 2, volume: 12000.0, largest: if upper > lower { upper } else { lower } }
}

/// SPLIT_BODY: the block cut in two by a plane taken from its own top face, 5 below it.
pub static SPLIT_BODY: Tool = Tool {
    id: "part.split-body",
    flow: Flow::Command,
    title: "feat-name-split-body",
    entries: &[Entry::Button("tb-split-body-hint"), Entry::Search("feat-name-split-body")],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    // the offset runs along the normal of the face taken, so a cut inside the block is a negative one: 0 lies on the
    // face itself and -10 on the bottom, and both are the plane touching the body rather than cutting it
    fields: &[Field { caption: "f-offset", by_placeholder: false, when: When::Before, class: Class::Length, typical: -5.0, lo: -9.9, hi: -0.1, zero: false, negative: true, outcome: cut_at_below }],
    modes: &[],
    result: cut_at(5.0),
    node: "SplitBody",
    undo: "f-operation",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Bodies { count: 2, volume: 24000.0, largest: 18000.0 },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/13-split-body",
    not_applicable: &[
        (2, "the bar of the cut holds no modes: it takes a plane, a datum or a face and an offset from it"),
        (17, "a plane that cuts the body always cuts it; a plane that misses it is refused in words, which is the point about values"),
    ],
};

/// The same cut read from the offset a person types: below the top face by `offset`, taken as a depth.
fn cut_at_below(offset: f64) -> Outcome {
    cut_at(-offset)
}

/// SPLIT_FACE: the same plane, but the body stays one and its four sides are each cut in two - 6 faces become 10,
/// 12 edges become 20 (the four sides split, and the seam runs round the body).
pub static SPLIT_FACE: Tool = Tool {
    id: "part.split-face",
    flow: Flow::Command,
    title: "feat-name-split-face",
    entries: &[Entry::Button("tb-split-face-hint"), Entry::Search("feat-name-split-face")],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "f-offset", by_placeholder: false, when: When::Before, class: Class::Length, typical: -5.0, lo: -9.9, hi: -0.1, zero: false, negative: true, outcome: split_faces }],
    modes: &[],
    result: split_faces(-5.0),
    node: "SplitFace",
    undo: "f-operation",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Body { volume: 24000.0, faces: 10, edges: 20, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 20.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/14-split-face",
    not_applicable: &[
        (2, "the bar of the face cut holds no modes: it takes a plane, a datum or a face and an offset from it"),
        (17, "a plane that crosses the body always divides its faces; a plane that misses it is refused in words, which is the point about values"),
    ],
};

/// The block with its sides divided by a plane: the body and its box are untouched, the faces and edges are not.
const fn split_faces(_offset: f64) -> Outcome {
    Outcome::Body { volume: 12000.0, faces: 10, edges: 20, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0] }
}

/// The block with the side at y = 0 tilted by `angle` about its top edge, where the neutral face holds it: the face
/// leans out by 10 * tan(angle) at the bottom, and the wedge under it is 40 * 10 * 10 * tan(angle) / 2.
fn drafted(angle: f64) -> Outcome {
    let lean = 10.0 * (angle * PI / 180.0).tan();
    // a face that leans out carries the body past y = 0; one that leans in leaves the neutral edge where it was
    let low = if lean > 0.0 { -lean } else { 0.0 };
    Outcome::Body { volume: 12000.0 + 200.0 * lean, faces: 6, edges: 12, min: [0.0, low, 0.0], max: [40.0, 30.0, 10.0] }
}

/// DRAFT: one side of the block tilted 3 degrees, the top face holding the edge it turns about.
pub static DRAFT: Tool = Tool {
    id: "part.draft",
    flow: Flow::Command,
    title: "f-draft",
    entries: &[Entry::Button("tb-draft-hint"), Entry::Search("feat-name-draft")],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::Block,
    // the face to tilt, then the word that says the next click is the neutral one, then the face that holds still
    picks: &[Pick::Face([20.0, 0.0, 5.0]), Pick::Bar("cmd-neutral-face"), Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 0.0, 5.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "f-angle", by_placeholder: false, when: When::Before, class: Class::Angle, typical: 3.0, lo: -60.0, hi: 60.0, zero: false, negative: true, outcome: drafted }],
    modes: &[],
    // 12000 + 200 * 10 * tan(3 deg) = 12104.82
    result: Outcome::Body { volume: 12104.8155585661, faces: 6, edges: 12, min: [0.0, -0.5240777730942, 0.0], max: [40.0, 30.0, 10.0] },
    node: "Draft",
    // the step of undo is named by the tool, as the window writes it: "Draft", not the "Operation" of the others
    undo: "f-draft",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        // the wedge grows with the height it leans over: 24000 + 40 * 20 * 20 * tan(3 deg) / 2
        then: Outcome::Body { volume: 24419.2622342644, faces: 6, edges: 12, min: [0.0, -1.0481555461884, 0.0], max: [40.0, 30.0, 20.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/09-draft",
    not_applicable: &[(17, "a face tilted by an angle the field accepts always builds: what the kernel could not make of it is a matter of the faces beside it, not of the value typed")],
};

/// The block as it was: a datum is drawn beside the body and takes nothing from it.
const fn block_alone(_value: f64) -> Outcome {
    Outcome::Body { volume: 12000.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0] }
}

/// DATUM_PLANE: a plane 10 above the top face of the block, to build from.
pub static DATUM_PLANE: Tool = Tool {
    id: "part.datum-plane",
    flow: Flow::Command,
    title: "cmd-plane",
    entries: &[Entry::Button("g-datum-plane-hint"), Entry::Key(Modifiers::NONE, Key::D), Entry::Search("cmdname-datum-plane")],
    other: (Entry::Button("g-datum-point-hint"), "cmd-point"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    // a datum plane may sit on the face it is taken from (0) or under it (a negative offset), so both are values of
    // its own; the body is the same whatever the plane does, and where the plane stands is read by what is built on it
    // its tilts about its X and Y and its turn about its normal take any angle either way, and the body stays as it is
    fields: &[
        Field { caption: "f-offset", by_placeholder: false, when: When::Before, class: Class::Length, typical: 10.0, lo: -100000.0, hi: 100000.0, zero: true, negative: true, outcome: block_alone },
        Field { caption: "f-tilt-x", by_placeholder: false, when: When::Before, class: Class::Angle, typical: 15.0, lo: -360.0, hi: 360.0, zero: true, negative: true, outcome: block_alone },
        Field { caption: "f-tilt-y", by_placeholder: false, when: When::Before, class: Class::Angle, typical: 15.0, lo: -360.0, hi: 360.0, zero: true, negative: true, outcome: block_alone },
        Field { caption: "f-turn", by_placeholder: false, when: When::Before, class: Class::Angle, typical: 30.0, lo: -360.0, hi: 360.0, zero: true, negative: true, outcome: block_alone },
    ],
    modes: &[],
    result: block_alone(10.0),
    node: "Plane",
    undo: "f-operation",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Body { volume: 24000.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 20.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "general/04-datums",
    not_applicable: &[
        (2, "the bar of the datum plane holds no modes: it takes a base and an offset from it"),
        (17, "a plane at an offset from another always builds: there is no value of the right form the kernel cannot make a plane at"),
    ],
};

/// DATUM_AXIS: the axis along an edge of the block.
pub static DATUM_AXIS: Tool = Tool {
    id: "part.datum-axis",
    flow: Flow::Command,
    title: "cmd-axis",
    entries: &[Entry::Button("g-datum-axis-hint"), Entry::Search("cmdname-datum-axis")],
    other: (Entry::Button("g-datum-plane-hint"), "cmd-plane"),
    fixture: Fixture::Block,
    picks: &[Pick::Edge([20.0, 0.0, 10.0])],
    pick_trial: &[Pick::Edge([20.0, 0.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[],
    result: block_alone(0.0),
    node: "DatumAxis",
    undo: "f-operation",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Body { volume: 24000.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 20.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "general/04-datums",
    not_applicable: &[
        (5, "the axis takes no value: it lies along the edge or the cylinder picked"),
        (6, "the axis takes no value to refuse"),
        (17, "an axis along an edge always builds: there is no value to give it that the kernel could not make"),
    ],
};

/// DATUM_POINT: a point at the coordinates typed, zero by zero by zero unless a vertex is picked.
pub static DATUM_POINT: Tool = Tool {
    id: "part.datum-point",
    flow: Flow::Command,
    title: "cmd-point",
    entries: &[Entry::Button("g-datum-point-hint"), Entry::Search("cmdname-datum-point")],
    other: (Entry::Button("g-datum-plane-hint"), "cmd-plane"),
    fixture: Fixture::Block,
    // the point is placed by its coordinates; a vertex clicked only fills them in
    picks: &[],
    pick_trial: &[],
    wrong_picks: &[],
    words: &[],
    fields: &[
        Field { caption: "X", by_placeholder: false, when: When::Before, class: Class::Length, typical: 0.0, lo: -10000000.0, hi: 10000000.0, zero: true, negative: true, outcome: block_alone },
        Field { caption: "Y", by_placeholder: false, when: When::Before, class: Class::Length, typical: 0.0, lo: -10000000.0, hi: 10000000.0, zero: true, negative: true, outcome: block_alone },
        Field { caption: "Z", by_placeholder: false, when: When::Before, class: Class::Length, typical: 0.0, lo: -10000000.0, hi: 10000000.0, zero: true, negative: true, outcome: block_alone },
    ],
    modes: &[],
    result: block_alone(0.0),
    node: "DatumPoint",
    undo: "f-operation",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Body { volume: 24000.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 20.0] },
    }),
    dependency: None,
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "general/04-datums",
    not_applicable: &[
        (2, "the bar of the datum point holds no modes: it takes three coordinates"),
        (3, "the point is placed by its coordinates and takes no pick of its own; a vertex clicked only fills them in"),
        (4, "with nothing to pick there is no wrong pick: a click in space puts the point where it points"),
        (15, "a point at coordinates stands on nothing of the timeline, so there is nothing to delete under it"),
        (17, "a point at coordinates always builds"),
    ],
};

/// FACE_COPY: the top face of the block taken out as a surface of its own - the block stays whole, and the sheet
/// beside it holds no volume.
pub static FACE_COPY: Tool = Tool {
    id: "part.face-copy",
    flow: Flow::Command,
    title: "feat-name-face-copy",
    entries: &[Entry::Button("tb-face-copy-hint"), Entry::SearchByArticle],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[],
    result: Outcome::Bodies { count: 2, volume: 12000.0, largest: 12000.0 },
    node: "FaceCopy",
    // the step of undo is named by the tool, as the window writes it: "Copy face"
    undo: "f-face-copy",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Bodies { count: 2, volume: 24000.0, largest: 24000.0 },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/20-face-copy",
    not_applicable: &[
        (2, "the bar of the face copy holds no modes: it takes the faces to copy and nothing else"),
        (5, "the face copy takes no value: it repeats the face picked"),
        (6, "the face copy takes no value to refuse"),
        (17, "a face the kernel can draw it can copy: there is no value to give it that it could not make"),
    ],
};

/// The top of the block, 40 x 30, lifted `d`: a sheet of 1200 at z 10 + d.
fn top_lifted(d: f64) -> Outcome {
    Outcome::Sheet { area: 1200.0, min: [0.0, 0.0, 10.0 + d], max: [40.0, 30.0, 10.0 + d] }
}

/// OFFSET_SURFACE: the top face of the block taken out as a sheet lifted 5 - the block stays whole, the sheet of 1200
/// stands at z 15. The distance is a length that may be zero (a copy in place) or below it (into the block).
pub static OFFSET_SURFACE: Tool = Tool {
    id: "part.offset-surface",
    flow: Flow::Command,
    title: "feat-name-offset-surface",
    entries: &[Entry::Button("tb-offset-surface-hint"), Entry::SearchByArticle],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "f-distance", by_placeholder: false, when: When::Before, class: Class::Length, typical: 5.0, lo: -1000.0, hi: 1000.0, zero: true, negative: true, outcome: top_lifted }],
    modes: &[],
    result: Outcome::Sheet { area: 1200.0, min: [0.0, 0.0, 15.0], max: [40.0, 30.0, 15.0] },
    node: "OffsetSurface",
    undo: "feat-name-offset-surface",
    undo_steps: 1,
    stays: false,
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 20.0,
        then: Outcome::Sheet { area: 1200.0, min: [0.0, 0.0, 25.0], max: [40.0, 30.0, 25.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/27-offset-surface",
    not_applicable: &[
        (2, "the bar of the offset holds no modes: the side is the sign of the distance"),
        (17, "a flat face moves any distance: the kernel refuses only a curved face turned inside out, which the block has none of"),
    ],
};

/// REMOVE_FACE: the wall of the hole taken away, the neighbouring faces closed over it - the block as it was before
/// the hole was made.
pub static REMOVE_FACE: Tool = Tool {
    id: "part.remove-face",
    flow: Flow::Command,
    title: "feat-name-remove-face",
    entries: &[Entry::Button("tb-remove-face-hint"), Entry::Search("feat-name-remove-face")],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::BlockWithHole,
    picks: &[Pick::Face([20.0, 25.0, 5.0])],
    pick_trial: &[Pick::Face([20.0, 25.0, 5.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[],
    result: Outcome::Body { volume: 12000.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0] },
    node: "RemoveFace",
    undo: "f-remove-face",
    undo_steps: 1,
    stays: false,
    // the hole above it is opened wider: its wall is another wall, and taking it away gives the same plain block.
    // The extrusion is not the node to change here - a block made 20 thick leaves the hole of 15 blind, and a blind
    // hole is a wall AND a bottom, which is a different feature from the one the pick named
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Hole", row: Some("feat-name-hole") },
        // 24 across still stands clear of the 30 the block is wide; a hole wider than that would cut the sides away
        // and leave no wall to take
        caption: "f-diameter",
        value: 24.0,
        then: Outcome::Body { volume: 12000.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/12-remove-face",
    not_applicable: &[
        (2, "the bar of remove face holds no modes: it takes the faces of the feature to take away"),
        (5, "remove face takes no value: it takes the faces picked away"),
        (6, "remove face takes no value to refuse"),
        (17, "a wall the kernel made it can take away; a face it cannot close over is a matter of the pick, not of a value"),
    ],
};

/// BOOLEAN: body A (the upper half of the cut block, picked beforehand) and body B (the lower half grown by a boss that
/// reaches 1000 into A, clicked with the tool in hand). Taking B away from A leaves A with the boss's pocket, 5000, and
/// uses B up; joining them gives the whole block back, its faces merged across the seam; their common part is the
/// 1000 where the boss runs into A. Pieces that only touch are refused in words (`part_booleans`).
pub static BOOLEAN: Tool = Tool {
    id: "part.boolean",
    flow: Flow::Command,
    title: "bool-bodies-btn",
    entries: &[Entry::Button("tb-bool-bodies-hint")],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::OverlappingPiecesUpperPicked,
    picks: &[Pick::Face([20.0, 0.0, 2.5])],
    pick_trial: &[Pick::Face([20.0, 0.0, 2.5])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[&[
        Mode { word: "f-cut-ab", clicks: None, outcome: Some(Outcome::Bodies { count: 1, volume: 5000.0, largest: 5000.0 }) },
        Mode { word: "f-union", clicks: None, outcome: Some(Outcome::Body { volume: 12000.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0] }) },
        Mode { word: "f-intersect", clicks: None, outcome: Some(Outcome::Bodies { count: 1, volume: 1000.0, largest: 1000.0 }) },
    ]],
    result: Outcome::Bodies { count: 1, volume: 5000.0, largest: 5000.0 },
    node: "BodyBoolean",
    undo: "f-operation",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: Some(Node { kind: "SplitBody", row: Some("feat-name-split-body") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/25-boolean",
    not_applicable: &[
        (5, "the boolean takes no value: its kind is a mode of the bar, and its bodies are picked"),
        (6, "the boolean takes no value to refuse"),
        (14, "above it stand the cut that made its two bodies and the boss that grew one into the other; their values are their own point 14, and this one follows them as the result does"),
        (17, "pieces that only touch are refused in words before a node, by the cut and by the intersection alike (part_booleans)"),
    ],
};

/// The block with its hole and a flat disc of surface over the top of the hole: the disc holds no volume, the body keeps
/// its 8858.41.
const PATCHED: Outcome = Outcome::Bodies { count: 2, volume: 8858.407346410208, largest: 8858.407346410208 };

/// PATCH: a surface spanned across the top edge of the hole - one closed circle, which is a boundary by itself, as in
/// the professional systems. On a flat circle "by position" and "smooth" give the same disc.
pub static PATCH: Tool = Tool {
    id: "part.patch",
    flow: Flow::Command,
    title: "feat-name-patch",
    entries: &[Entry::Button("tb-patch-hint"), Entry::SearchByArticle],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::BlockWithHole,
    picks: &[Pick::Edge([30.0, 15.0, 10.0])],
    pick_trial: &[Pick::Edge([30.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[&[Mode { word: "cmd-patch-flat", clicks: None, outcome: Some(PATCHED) }, Mode { word: "cmd-patch-tangent", clicks: None, outcome: Some(PATCHED) }]],
    result: PATCHED,
    node: "Patch",
    undo: "f-patch",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: Some(Node { kind: "Hole", row: Some("feat-name-hole") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/22-patch",
    not_applicable: &[
        (5, "the patch takes no value: its surface is given by the edges picked and the mode"),
        (6, "the patch takes no value to refuse"),
        (14, "the hole under it is changed in its own contract, and the disc follows the edge it was spanned on - which point 15 checks from the other side"),
        (17, "edges that do not close a boundary are refused by the picks, not by a value"),
    ],
};

/// SKETCH_ON_FACE: the pencil, then a click on the top face of the block - a sketch opens on that face, empty, a node
/// of the timeline standing on the extrusion under it.
pub static SKETCH_ON_FACE: Tool = Tool {
    id: "part.sketch-on-face",
    flow: Flow::Command,
    title: "status-new-sketch",
    entries: &[Entry::Button("g-sketch-pick-hint"), Entry::Key(Modifiers::NONE, Key::K)],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[],
    result: crate::contract::EMPTY_SKETCH,
    node: "Sketch",
    undo: "status-new-sketch",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (10, 2000),
    help: "general/04-datums",
    not_applicable: &[
        (2, "the pencil has no bar of modes: a click on a plane, a datum or a face is the whole of it"),
        (3, "the face is taken by the click that opens the sketch on it: there is no taking it back but leaving the sketch"),
        (5, "the pencil takes no value"),
        (6, "the pencil takes no value to refuse"),
        (7, "the sketch opens on the click: there is nothing to show before it"),
        (8, "the sketch is opened by the click on the face, and it is left by \"Finish\", not applied with Enter"),
        (9, "Esc before the click puts the pencil down, which is point 1; after it the sketch is open and left by \"Finish\""),
        (13, "the sketch is reopened by a double click on its row, which is the sketch's own contract"),
        (14, "a change above is carried by the sketch on the face, which is the matter of in-context references"),
        (17, "any flat face can carry a sketch: there is nothing the kernel could refuse"),
    ],
};

/// What a sweep or a loft leaves out: it works on sketches picked in the tree, and takes no value.
const FROM_SKETCHES: &[(u8, &str)] = &[
    (4, "the sections are rows of the tree; a click on empty space there picks nothing to refuse"),
    (5, "the tool takes no value: the shape is given by the sketches"),
    (6, "the tool takes no value to refuse"),
    (14, "the change above is a change of a sketch, which is the sketch's own contract of dimensions"),
    (15, "the sketches it stands on are deleted from the tree, which is the sketch's own matter"),
    (16, "the fixture builds its own part from the start: there is no second part to take it in"),
    (17, "sections that the kernel cannot join are a matter of the sketches, not of a value typed"),
];

/// SWEEP: the square of 10 swept along the straight line of 40 up the front plane - a bar of 10 x 10 x 40.
pub static SWEEP: Tool = Tool {
    id: "part.sweep",
    flow: Flow::Command,
    title: "cmd-sweep",
    entries: &[Entry::Button("tb-sweep-hint"), Entry::SearchByArticle],
    other: (Entry::Button("tb-extrude-hint"), "cmd-extrude"),
    fixture: Fixture::SweepStraight,
    picks: &[Pick::Row("Sketch 2")],
    pick_trial: &[Pick::Row("Sketch 2")],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[],
    result: Outcome::Body { volume: 4000.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [10.0, 10.0, 40.0] },
    node: "Sweep",
    undo: "cmd-sweep",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: None,
    contexts: &[],
    refusal: None,
    budget: (30, 2000),
    help: "part/03-sweep",
    not_applicable: FROM_SKETCHES,
};

/// LOFT: from the square of 10 on the table to the square of 20 thirty above - a frustum of 30 / 3 * (100 + 400 +
/// sqrt(100 * 400)) = 7000.
pub static LOFT: Tool = Tool {
    id: "part.loft",
    flow: Flow::Command,
    title: "cmd-loft",
    entries: &[Entry::Button("tb-loft-hint"), Entry::SearchByArticle],
    other: (Entry::Button("tb-extrude-hint"), "cmd-extrude"),
    fixture: Fixture::LoftSquares,
    picks: &[Pick::Row("Sketch 2")],
    pick_trial: &[Pick::Row("Sketch 2")],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[],
    result: Outcome::Body { volume: 7000.0, faces: 6, edges: 12, min: [-5.0, -5.0, 0.0], max: [15.0, 15.0, 30.0] },
    node: "Loft",
    undo: "cmd-loft",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: None,
    contexts: &[],
    refusal: None,
    budget: (30, 2000),
    help: "part/04-loft",
    not_applicable: FROM_SKETCHES,
};

/// The cylinder of 20 across and 10 tall with an outer M20 (pitch 2.5) cut over `length` from its top: what the groove
/// takes is the standard's own profile (ISO 68-1), with no fit clearance and no run-out, to a hundredth.
fn threaded(length: f64) -> Outcome {
    let groove = crate::iso::outer_groove(20.0, 2.5, length);
    // the profile swept by Pappus is the average over whole turns; the two ends of a helix cut over part of a turn
    // stray from it by less than a cubic millimetre (0.57 measured over 0.5 of length, a fifth of a pitch)
    Outcome::Volume { volume: PI * 100.0 * 10.0 - groove, tol: groove * 0.01 + 1.0, min: [10.0, 5.0, 0.0], max: [30.0, 25.0, 10.0] }
}

/// The words of a thread made to the standard: the size of the face, no clearance, no run-in or run-out.
fn to_the_standard(_text: &str) -> Outcome {
    threaded(10.0)
}

/// THREAD: an outer M20 the whole 10 of the cylinder - the size typed to match the face (20 across), the fit clearance
/// and the run-in and run-out set to nothing, so the groove is the standard's profile: pi * 100 * 10 - 469.11 =
/// 2672.48, to a hundredth of the groove. The matrix of threads carries the inner thread, another size and a flat face.
pub static THREAD: Tool = Tool {
    id: "part.thread",
    flow: Flow::Command,
    title: "cmd-thread",
    entries: &[Entry::Button("tb-thread-hint"), Entry::SearchByArticle],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::CylinderBody,
    picks: &[Pick::Face([27.071, 7.929, 5.0])],
    pick_trial: &[Pick::Face([27.071, 7.929, 5.0])],
    wrong_picks: &[],
    words: &[
        Words { caption: "f-nominal-d", by_placeholder: false, typical: "20", valid: &[], invalid: &[], outcome: to_the_standard, enter: false },
        Words { caption: "th-fit-clearance", by_placeholder: false, typical: "0", valid: &[], invalid: &[], outcome: to_the_standard, enter: false },
        Words { caption: "th-lead-in", by_placeholder: false, typical: "0", valid: &[], invalid: &[], outcome: to_the_standard, enter: false },
        Words { caption: "th-lead-out", by_placeholder: false, typical: "0", valid: &[], invalid: &[], outcome: to_the_standard, enter: false },
    ],
    fields: &[Field { caption: "f-length", by_placeholder: false, when: When::Before, class: Class::Length, typical: 10.0, lo: 0.5, hi: 10.0, zero: false, negative: false, outcome: threaded }],
    modes: &[],
    // pi * 100 * 10 - 469.11 (the groove of M20 x 2.5 over 10, worked out by Pappus from the ISO 68-1 profile)
    result: Outcome::Volume { volume: 2672.479, tol: 5.69, min: [10.0, 5.0, 0.0], max: [30.0, 25.0, 10.0] },
    node: "Thread",
    undo: "cmd-thread",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: None,
    contexts: &[],
    refusal: None,
    budget: (60, 2000),
    help: "part/10-thread",
    not_applicable: &[
        (2, "the kinds of thread in the bar (outer, inner, standards) are the matrix of threads' cases"),
        (14, "the cylinder under the thread is made by the fixture's own extrusion; a change of it is the matrix's matter"),
        (15, "deleting the extrusion under the thread is the matrix of threads' matter"),
        (16, "the fixture builds its own part from the start: there is no second part to take it in"),
        (17, "a thread of another size than the face is refused - that is the matrix of threads' case"),
    ],
};

/// The block and the one surface the two copied faces make once stitched along the edge they meet at.
const fn stitched(_tol: f64) -> Outcome {
    Outcome::Bodies { count: 2, volume: 12000.0, largest: 12000.0 }
}

/// STITCH: the two sheets clicked in turn and Enter - one surface of the two, the pieces used up; the block beside it
/// is untouched.
pub static STITCH: Tool = Tool {
    id: "part.stitch",
    flow: Flow::Command,
    title: "f-stitch",
    entries: &[Entry::Button("tb-stitch-hint"), Entry::SearchByArticle],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::BlockAndTwoSheets,
    picks: &[Pick::Face([20.0, 15.0, 10.0]), Pick::Face([20.0, 0.0, 5.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    // how far apart two edges may be and still be one seam: the sheets meet exactly, so any tolerance joins them
    fields: &[Field {
        caption: "f-stitch-tol",
        by_placeholder: false,
        when: When::Before,
        class: Class::Tolerance,
        typical: 0.01,
        lo: 0.000001,
        hi: 10.0,
        zero: false,
        negative: false,
        outcome: stitched,
    }],
    modes: &[],
    result: stitched(0.01),
    node: "Stitch",
    undo: "f-operation",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/23-stitch",
    not_applicable: &[
        (2, "the stitch has no modes: the sheets and the tolerance are the whole of it"),
        (14, "the change above is a change of the copies it is made of, which is the face copy's own point 14"),
        (17, "sheets that do not meet are refused by the picks, not by a value"),
    ],
};

/// REPLACE_FACE: the top face of the block and the sheet copied from it - the face takes the sheet's shape, which is its
/// own, so the block is what it was; the sheet handed back is used up, the other one stays.
pub static REPLACE_FACE: Tool = Tool {
    id: "part.replace-face",
    flow: Flow::Command,
    title: "f-surface-replace",
    entries: &[Entry::Button("tb-surface-replace-hint"), Entry::SearchByArticle],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::BlockAndTwoSheets,
    picks: &[Pick::Face([20.0, 15.0, 10.0]), Pick::Face([20.0, 15.0, 10.0])],
    // the right face: no sheet lies on it, so the second click lands on the same face - on the top a sheet lies over
    // the face, and the second click there rightly takes the sheet as the surface
    pick_trial: &[Pick::Face([40.0, 15.0, 5.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[],
    result: Outcome::Bodies { count: 2, volume: 12000.0, largest: 12000.0 },
    node: "SurfaceReplace",
    undo: "f-surface-replace",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/21-surface-replace",
    not_applicable: &[
        (2, "replacing a face has no modes: the face and the surface are the whole of it"),
        (5, "replacing a face takes no value"),
        (6, "replacing a face takes no value to refuse"),
        (14, "the change above is a change of the copy it hands back, which is the face copy's own point 14"),
        (17, "a surface that does not cover the face is refused by the picks, not by a value"),
    ],
};

/// The block with the boss 10 across and 10 tall on its top.
const BOSSED: f64 = 12000.0 + PI * 25.0 * 10.0;

/// TRIM_SURFACE: the sheet copied off the top face clicked where it is kept, off the boss, then the top of the boss
/// clicked as the body to cut with, and Enter - the disc under the boss is cut out of the sheet; the part stays one
/// body of 12000 + pi x 5^2 x 10 = 12785.4 with the trimmed sheet beside it.
pub static TRIM_SURFACE: Tool = Tool {
    id: "part.trim-surface",
    flow: Flow::Command,
    title: "feat-name-trim",
    entries: &[Entry::Button("tb-trim-surface-hint"), Entry::SearchByArticle],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::BlockSheetAndBoss,
    picks: &[Pick::Face([5.0, 5.0, 10.0]), Pick::Face([20.0, 15.0, 20.0])],
    pick_trial: &[Pick::Face([5.0, 5.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[],
    result: Outcome::Bodies { count: 2, volume: BOSSED, largest: BOSSED },
    node: "Trim",
    undo: "f-operation",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (30, 2000),
    help: "part/24-trim",
    not_applicable: &[
        (2, "the trim has no modes: the side kept and the body to cut with are the whole of it"),
        (5, "the trim takes no value: where it cuts is given by the body"),
        (6, "the trim takes no value to refuse"),
        (14, "the change above is a change of the copy it cuts, which is the face copy's own point 14"),
        (17, "a body that does not reach the sheet is refused by the picks, not by a value"),
    ],
};

/// The cylinder with an outer thread of 20 over its whole 10, cut at a pitch of one's own - the coarse pitch a printed
/// thread wants: the groove is the ISO 60-degree profile drawn at that pitch, so the same Pappus sum gives it.
fn threaded_at(pitch: f64) -> Outcome {
    // a pitch of 0 is the standard's own, as the field's caption says: 2.5 for M20
    let pitch = if pitch == 0.0 { 2.5 } else { pitch };
    let groove = crate::iso::outer_groove(20.0, pitch, 10.0);
    Outcome::Volume { volume: PI * 100.0 * 10.0 - groove, tol: groove * 0.01 + 1.0, min: [10.0, 5.0, 0.0], max: [30.0, 25.0, 10.0] }
}

/// THREAD_OWN_PITCH: the same M20 over the whole 10, at a pitch of 4 typed in place of the standard 2.5 - a thread for a
/// printed part. A pitch finer than 1 does not print and one coarser than 5 leaves no thread on a shaft of 20; a pitch
/// ten times over that must be refused.
pub static THREAD_OWN_PITCH: Tool = Tool {
    id: "part.thread-own-pitch",
    flow: Flow::Command,
    title: "cmd-thread",
    entries: &[Entry::Button("tb-thread-hint")],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::CylinderBody,
    picks: &[Pick::Face([27.071, 7.929, 5.0])],
    pick_trial: &[Pick::Face([27.071, 7.929, 5.0])],
    wrong_picks: &[],
    words: &[
        Words { caption: "f-nominal-d", by_placeholder: false, typical: "20", valid: &[], invalid: &[], outcome: to_the_standard, enter: false },
        Words { caption: "th-fit-clearance", by_placeholder: false, typical: "0", valid: &[], invalid: &[], outcome: to_the_standard, enter: false },
        Words { caption: "th-lead-in", by_placeholder: false, typical: "0", valid: &[], invalid: &[], outcome: to_the_standard, enter: false },
        Words { caption: "th-lead-out", by_placeholder: false, typical: "0", valid: &[], invalid: &[], outcome: to_the_standard, enter: false },
        Words { caption: "f-length", by_placeholder: false, typical: "10", valid: &[], invalid: &[], outcome: to_the_standard, enter: false },
    ],
    fields: &[Field { caption: "f-pitch-std", by_placeholder: false, when: When::Before, class: Class::Length, typical: 4.0, lo: 1.0, hi: 5.0, zero: true, negative: false, outcome: threaded_at }],
    modes: &[],
    // pi * 100 * 10 - 722.52 (the ISO profile drawn at a pitch of 4 over 10, by Pappus)
    result: Outcome::Volume { volume: 2419.076, tol: 8.23, min: [10.0, 5.0, 0.0], max: [30.0, 25.0, 10.0] },
    node: "Thread",
    undo: "cmd-thread",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: None,
    contexts: &[],
    refusal: None,
    budget: (60, 2000),
    help: "part/10-thread",
    not_applicable: &[
        (2, "the kinds of thread in the bar are the matrix of threads' cases"),
        (14, "the cylinder under the thread is made by the fixture's own extrusion; a change of it is the matrix's matter"),
        (15, "deleting the extrusion under the thread is the matrix of threads' matter"),
        (16, "the fixture builds its own part from the start: there is no second part to take it in"),
        (17, "a thread of another size than the face is refused - that is the matrix of threads' case"),
    ],
};

/// SECTION: the section of the view, the top face of the block clicked as its plane - half the model is hidden and its
/// inside shows; the document is untouched.
pub static SECTION: Tool = Tool {
    id: "part.section",
    flow: Flow::Command,
    title: "sec-btn",
    entries: &[Entry::Button("tb-section-hint-bar")],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[],
    result: Outcome::Section { on: true },
    node: "",
    undo: "tb-section-off",
    undo_steps: 0,
    stays: true,
    upstream: None,
    dependency: None,
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (10, 2000),
    help: "general/09-viewport",
    not_applicable: &[
        (2, "the section's offset and tilts in the bar move the plane; where it cuts is not read yet"),
        (5, "the section takes its offset from the bar, and where it cuts is not read yet"),
        (6, "the section takes its offset from the bar, and where it cuts is not read yet"),
        (11, "a section is the view, not the document: it lays no step of undo"),
        (12, "a section is the view, not the document: saving and opening does not carry it"),
        (13, "a section lays no node"),
        (14, "nothing of a timeline stands above a section"),
        (15, "a section stands on nothing of the document"),
        (17, "a plane can always cut the view"),
    ],
};

/// What a measure leaves out: it reads, it does not make - no node, no step of undo, nothing saved.
const READS: &[(u8, &str)] = &[
    (2, "the measure has no modes: what is clicked decides what is measured"),
    (5, "the measure takes no value"),
    (6, "the measure takes no value to refuse"),
    (7, "the measure answers on the second click: there is nothing to show before it"),
    (8, "a measure changes nothing and has nothing to apply: its answer stands in the bar as the second element is clicked"),
    (11, "a measure changes nothing: it lays no step of undo"),
    (12, "a measure changes nothing: saving and opening carries nothing of it"),
    (13, "a measure lays no node"),
    (14, "nothing of a timeline stands above a measure"),
    (15, "a measure stands on nothing of the document"),
    (17, "two points can always be measured"),
];

/// MEASURE_3D: the two corners of the front top edge of the block clicked - they are 40 apart, and the status line
/// says so.
pub static MEASURE_3D: Tool = Tool {
    id: "part.measure-3d",
    flow: Flow::Command,
    title: "hotkey-part-i",
    entries: &[Entry::Button("tb-measure3d-hint")],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::Block,
    picks: &[Pick::Vertex([0.0, 0.0, 10.0]), Pick::Vertex([40.0, 0.0, 10.0])],
    pick_trial: &[Pick::Vertex([0.0, 0.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[],
    result: Outcome::Says { text: "40" },
    node: "",
    undo: "hotkey-part-i",
    undo_steps: 0,
    stays: true,
    upstream: None,
    dependency: None,
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: (10, 2000),
    help: "general/11-measure",
    not_applicable: READS,
};

/// RECOGNISE: the mesh of the block clicked and Enter - turned into a body again: by exact surfaces the box it came
/// from, six faces and twelve edges; as it is, a polyhedron of its triangles, the same material.
pub static RECOGNISE: Tool = Tool {
    id: "part.recognise",
    flow: Flow::Command,
    title: "f-recognise",
    entries: &[Entry::Button("tb-recognise-hint"), Entry::SearchByArticle],
    other: (Entry::Button("g-datum-plane-hint"), "cmd-plane"),
    fixture: Fixture::MeshOfBlock,
    // a mesh has no faces to name: the click is on the mesh itself
    picks: &[Pick::Space([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Space([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[&[
        Mode { word: "cmd-recognise-exact", clicks: None, outcome: Some(Outcome::Body { volume: 12000.0, faces: 6, edges: 12, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0] }) },
        Mode { word: "cmd-recognise-asis", clicks: None, outcome: Some(Outcome::Bodies { count: 1, volume: 12000.0, largest: 12000.0 }) },
    ]],
    result: Outcome::Bodies { count: 1, volume: 12000.0, largest: 12000.0 },
    node: "MeshRecognised", // the node of the timeline is named by what it holds: a mesh recognised
    undo: "f-recognise",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: None,
    contexts: &[],
    refusal: None,
    budget: (60, 2000),
    help: "part/26-recognise",
    not_applicable: &[
        (5, "recognising takes the tolerances of the bar as they come; the values are the matrix of meshes' cases"),
        (6, "recognising takes the tolerances of the bar as they come; the values are the matrix of meshes' cases"),
        (14, "nothing of a timeline stands above an imported mesh"),
        (15, "the mesh stands on the file it came from, not on a node"),
        (16, "the fixture builds its own project from the start: there is no second part to take it in"),
        (17, "a mesh that holds no closed volume is refused by the pick, not by a value"),
    ],
};

/// Any name is two parts: the block split 5 below its top, and the upper piece taken into a part of its own.
fn two_parts(_: &str) -> Outcome {
    Outcome::Parts { parts: 2, grounded: 0, joints: 0, assemblies: 0 }
}

/// MAKE A PART: the upper of the two bodies a split left, picked by a double click, made a part of its own through the
/// command search - the name is asked beside the cursor and given with Enter.
pub static PIECE_TO_PART: Tool = Tool {
    id: "part.piece-to-part",
    flow: Flow::Action,
    title: "act-piece-to-part",
    entries: &[Entry::Search("act-piece-to-part")],
    other: (Entry::Button("tb-mirror-body-hint"), "cmd-mirror"),
    fixture: Fixture::OverlappingPiecesUpperPicked,
    picks: &[],
    pick_trial: &[],
    wrong_picks: &[],
    words: &[Words {
        caption: "piece-part-name",
        by_placeholder: false,
        typical: "Upper",
        // an ordinary name, one of another alphabet, a long one
        valid: &["Upper", "\u{412}\u{435}\u{440}\u{445}", "the upper piece of the block split five below its top"],
        // nothing, and spaces alone, are no name
        invalid: &["", "   "],
        outcome: two_parts,
        enter: true,
    }],
    fields: &[],
    modes: &[],
    result: Outcome::Parts { parts: 2, grounded: 0, joints: 0, assemblies: 0 },
    node: "Piece",
    undo: "act-piece-to-part",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: Some(Node { kind: "SplitBody", row: Some("feat-name-split-body") }),
    contexts: &[],
    refusal: None,
    budget: (30, 2000),
    help: "part/13-split-body",
    not_applicable: &[
        (2, "the command holds no bar: it asks one name beside the cursor"),
        (3, "the piece is picked before the command, by a double click; the click taking it after is the command's own case of nothing picked"),
        (4, "the piece is picked before the command: there is no pick of its own to refuse"),
        (7, "a part made of a piece is the piece itself: there is nothing to show before it"),
        (13, "the node of the piece has no command to reopen: it reads the piece, and the cut that made it is edited instead"),
        (14, "above the piece stand the split and the boss; their values are their own point 14, and the piece follows them"),
        (16, "a piece is made a part in the part that shows it: there is no second place to take it"),
        (17, "a piece can always be made a part: the kernel only copies it"),
        (19, "the command is not held, so F1 has no tool in hand to open an article of"),
    ],
};

//! The tools that read the part - section, measure, recognise - and the one that makes a piece a part.

use super::*;

/// SECTION: the section of the view, the top face of the block clicked as its plane - half the model is hidden and its
/// inside shows; the document is untouched.
pub static SECTION: Tool = Tool {
    id: "part.section",
    flow: Flow::Command,
    title: "sec-btn",
    entries: &[Entry::Button("tb-section-hint-bar")],
    other: Other { entry: Entry::Button("tb-mirror-body-hint"), title: "cmd-mirror" },
    fixture: Fixture::Block,
    picks: &[Pick::Face([20.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[],
    result: Outcome::Section { on: true },
    node: "",
    inputs: &[],
    undo: "tb-section-off",
    undo_steps: 0,
    stays: true,
    upstream: None,
    dependency: None,
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: Budget { apply_secs: 10, frame_ms: 2000 },
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
pub(crate) const READS: &[(u8, &str)] = &[
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
    other: Other { entry: Entry::Button("tb-mirror-body-hint"), title: "cmd-mirror" },
    fixture: Fixture::Block,
    picks: &[Pick::Vertex([0.0, 0.0, 10.0]), Pick::Vertex([40.0, 0.0, 10.0])],
    pick_trial: &[Pick::Vertex([0.0, 0.0, 10.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[],
    modes: &[],
    result: Outcome::Says { text: "40" },
    node: "",
    inputs: &[],
    undo: "hotkey-part-i",
    undo_steps: 0,
    stays: true,
    upstream: None,
    dependency: None,
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: Budget { apply_secs: 10, frame_ms: 2000 },
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
    other: Other { entry: Entry::Button("g-datum-plane-hint"), title: "cmd-plane" },
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
    inputs: &[],
    undo: "f-recognise",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: None,
    contexts: &[],
    refusal: None,
    budget: Budget { apply_secs: 60, frame_ms: 2000 },
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

/// The block with its hole of 20 made again exact at the diameter `d`: the same block, a hole of `d` through it.
const fn drilled_again(d: f64) -> Outcome {
    let v = 12000.0 - PI * d * d / 4.0 * 10.0;
    Outcome::Bodies { count: 1, volume: v, largest: v }
}

/// RECOGNISE FEATURE: a block with a hole of 20, come in as a mesh and made a polyhedron, the wall of its hole a ring of
/// flat strips. A strip of the wall clicked - the hole found through, its diameter brought to 20 in the field - and
/// Enter makes it again exact as one node: the same block, its hole round, its diameter driving it. Switched to "Take
/// off", Enter closes the hole: the block whole. The tool stays in hand for the next feature.
pub static RECOGNISE_FEATURE: Tool = Tool {
    id: "part.recognise-feature",
    flow: Flow::Command,
    title: "f-recognise-feature",
    entries: &[Entry::Button("tb-recognise-feature-hint"), Entry::SearchByArticle],
    other: Other { entry: Entry::Button("tb-mirror-body-hint"), title: "cmd-mirror" },
    fixture: Fixture::PolyhedronWithHole,
    picks: &[Pick::Face([20.0, 25.0, 5.0])],
    pick_trial: &[Pick::Face([20.0, 25.0, 5.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "f-diameter", by_placeholder: false, when: When::Before, class: Class::Length, typical: 20.0, lo: 0.1, hi: 29.0, zero: false, negative: false, outcome: drilled_again }],
    modes: &[&[
        Mode { word: "cmd-remade-exact", clicks: None, outcome: Some(drilled_again(20.0)) },
        Mode { word: "cmd-remade-off", clicks: None, outcome: Some(Outcome::Bodies { count: 1, volume: 12000.0, largest: 12000.0 }) },
    ]],
    result: drilled_again(20.0),
    node: "Remade",
    inputs: &[],
    undo: "f-recognise-feature",
    undo_steps: 1,
    stays: true,
    upstream: None,
    dependency: Some(Node { kind: "MeshSolid", row: Some("feat-name-mesh-solid") }),
    contexts: &[],
    refusal: None,
    budget: Budget { apply_secs: 30, frame_ms: 2000 },
    help: "part/28-recognise-feature",
    not_applicable: &[
        (14, "nothing of a timeline stands above an imported mesh"),
        (16, "the fixture builds its own project from the start: there is no second part to take it in"),
        (17, "a hole wider than the block is refused by the bounds of the field; the kernel has nothing else to refuse here"),
    ],
};

/// The stepped block with its upper top, the first face picked, moved to stand `d` above its lower top at 10.
const fn gapped(d: f64) -> Outcome {
    Outcome::Body { volume: 6000.0 + 600.0 * (10.0 + d), faces: 8, edges: 18, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0 + d] }
}

/// SIZE ON BODY: the upper top of the stepped block picked, then its lower top - 5 apart - and Enter: the field stands
/// at the 5 they are apart and lays nothing until another size is typed. 8 typed: the upper top moves up 3. Moving the second face takes the lower top down 3 instead, both by half each of them 1.5.
pub static FACE_GAP: Tool = Tool {
    id: "part.face-gap",
    flow: Flow::Command,
    title: "f-face-gap",
    entries: &[Entry::Button("tb-face-gap-hint"), Entry::SearchByArticle],
    other: Other { entry: Entry::Button("tb-mirror-body-hint"), title: "cmd-mirror" },
    fixture: Fixture::SteppedBlock,
    picks: &[Pick::Face([30.0, 15.0, 15.0]), Pick::Face([10.0, 15.0, 10.0])],
    pick_trial: &[Pick::Face([30.0, 15.0, 15.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field { caption: "f-gap", by_placeholder: false, when: When::After, class: Class::Length, typical: 8.0, lo: 0.01, hi: 100000.0, zero: false, negative: false, outcome: gapped }],
    modes: &[&[
        Mode { word: "cmd-gap-one", clicks: None, outcome: Some(gapped(8.0)) },
        Mode { word: "cmd-gap-other", clicks: None, outcome: Some(Outcome::Body { volume: 13200.0, faces: 8, edges: 18, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 15.0] }) },
        Mode { word: "cmd-gap-both", clicks: None, outcome: Some(Outcome::Body { volume: 15000.0, faces: 8, edges: 18, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 16.5] }) },
    ]],
    result: gapped(8.0),
    node: "FaceGap",
    inputs: &[],
    undo: "f-face-gap",
    undo_steps: 1,
    stays: false,
    // the block made 12 tall: its top stands 3 under the upper top, and the size of 8 takes the upper top to 20
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 12.0,
        then: Outcome::Body { volume: 7200.0 + 600.0 * 20.0, faces: 8, edges: 18, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 20.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: Budget { apply_secs: 30, frame_ms: 2000 },
    help: "part/29-face-gap",
    not_applicable: &[(17, "a size the faces cannot stand at is refused by the bounds of its field; past them there is nothing for the kernel to refuse")],
};

/// The bossed block with its boss redrawn `h` high above the top of the block.
const fn redrawn(h: f64) -> Outcome {
    Outcome::Body { volume: 12000.0 + 200.0 * h, faces: 11, edges: 24, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 10.0 + h] }
}

/// SKETCH FROM FACE: the top of the boss on the bossed block clicked - the boss lit, its height of 5 in the field - and
/// Enter: a sketch of the top, fully defined, and one node redrawing the boss by it; another height typed raises or
/// lowers it. The field stands before Enter, so it is filled before the tool is finished. Sketch only lays the sketch and leaves the body as it is. The tool stays in hand for the next face.
pub static SKETCH_FROM_FACE: Tool = Tool {
    id: "part.sketch-from-face",
    flow: Flow::Command,
    title: "f-sketch-from-face",
    entries: &[Entry::Button("tb-sketch-from-face-hint"), Entry::SearchByArticle],
    other: Other { entry: Entry::Button("tb-mirror-body-hint"), title: "cmd-mirror" },
    fixture: Fixture::BossedBlock,
    picks: &[Pick::Face([20.0, 15.0, 15.0])],
    pick_trial: &[Pick::Face([20.0, 15.0, 15.0])],
    wrong_picks: &[],
    words: &[],
    fields: &[Field {
        caption: "f-redrawn-height",
        by_placeholder: false,
        when: When::Before,
        class: Class::Length,
        typical: 5.0,
        lo: 0.01,
        hi: 100000.0,
        zero: false,
        negative: false,
        outcome: redrawn,
    }],
    // a mode taken leaves the height as found: the boss redrawn as it stood, or only its sketch laid - one body either way
    modes: &[&[Mode { word: "cmd-sketch-replace", clicks: None, outcome: Some(redrawn(5.0)) }, Mode { word: "cmd-sketch-only", clicks: None, outcome: Some(redrawn(5.0)) }]],
    result: redrawn(5.0),
    node: "Redrawn",
    inputs: &["Sketch"],
    undo: "f-sketch-from-face",
    undo_steps: 1,
    stays: true,
    // the block made 12 tall: the boss redrawn from its base stands its 5 above it
    upstream: Some(Upstream::Reopen {
        node: Node { kind: "Extrude", row: Some("cmd-extrude") },
        caption: "f-length",
        value: 12.0,
        then: Outcome::Body { volume: 14400.0 + 1000.0, faces: 11, edges: 24, min: [0.0, 0.0, 0.0], max: [40.0, 30.0, 17.0] },
    }),
    dependency: Some(Node { kind: "Extrude", row: Some("cmd-extrude") }),
    contexts: &[Context::SecondPart],
    refusal: None,
    budget: Budget { apply_secs: 30, frame_ms: 2000 },
    help: "part/30-sketch-from-face",
    not_applicable: &[(17, "a height the boss cannot stand at is refused by the bounds of its field; past them there is nothing for the kernel to refuse")],
};

/// Any name is two parts: the block split 5 below its top, and the upper piece taken into a part of its own.
pub(crate) fn two_parts(_: &str) -> Outcome {
    Outcome::Parts { parts: 2, grounded: 0, joints: 0, assemblies: 0 }
}

/// MAKE A PART: the upper of the two bodies a split left, picked by a double click, made a part of its own through the
/// command search - the name is asked beside the cursor and given with Enter.
pub static PIECE_TO_PART: Tool = Tool {
    id: "part.piece-to-part",
    flow: Flow::Action,
    title: "act-piece-to-part",
    entries: &[Entry::Search("act-piece-to-part")],
    other: Other { entry: Entry::Button("tb-mirror-body-hint"), title: "cmd-mirror" },
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
    inputs: &[],
    undo: "act-piece-to-part",
    undo_steps: 1,
    stays: false,
    upstream: None,
    dependency: Some(Node { kind: "SplitBody", row: Some("feat-name-split-body") }),
    contexts: &[],
    refusal: None,
    budget: Budget { apply_secs: 30, frame_ms: 2000 },
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

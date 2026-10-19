//! THE PART WORKBENCH: the feature commands.
//!
//! Extrude, revolve, sweep, loft, fillet, chamfer, shell, the array, the thread, the hole, the split -
//! their option bars, their popups at the geometry, their previews and what they do to the document. Every
//! function here works over `qymcad_ui_state::PartCtx` and asks the application for nothing.

mod face_gap;
mod sketch_from_face;
mod sketch_from_section;
mod recognise_feature;
mod recognise_all;
mod trial;
pub use sketch_from_face::{
    apply_sketch_from_face_cmd, edit_redrawn, open_redrawn, sketch_from_face_at, sketch_from_face_mode_changed, sketch_from_face_ready, start_sketch_from_face_cmd, JUST_SKETCH, REPLACE,
    SKETCH_FROM_FACE,
};
pub use sketch_from_section::{
    apply_sketch_from_section_cmd, section_asked, section_base, section_plane_taken, section_preview, sketch_from_section_at, sketch_from_section_plane_changed, sketch_from_section_ready,
    start_sketch_from_section_cmd, told_pieces, SectionAsked, ON_XY, ON_XZ, ON_YZ, SKETCH_FROM_SECTION,
};
pub use face_gap::{apply_face_gap_cmd, edit_face_gap, edit_gap, face_gap_at, face_gap_holds_hole, face_gap_ready, gap_moves, open_face_gap, start_face_gap_cmd, FACE_GAP, MOVE_BOTH, MOVE_ONE, MOVE_OTHER};
pub use recognise_feature::{
    apply_recognise_feature_cmd, edit_remade, hit_on_face, open_remade, recognise_feature_at, recognise_mode_changed, start_recognise_feature_cmd, MAKE_EXACT, RECOGNISE_FEATURE, TAKE_OFF,
};
pub use recognise_all::{apply_recognise_all_cmd, group_told, open_remade_all, recognise_all_at, recognise_all_ready, start_recognise_all_cmd, RECOGNISE_ALL};
pub use trial::{trial_refusal, Trial};

use egui::{Pos2, Rect};
use egui_phosphor::regular as ph;
use qymcad_core::model::{ArrayAxis, Id, Project};
use qymcad_ui_state::WinKind;

mod bars;
pub use bars::*;
mod popups;
pub use popups::*;
mod params;
pub use params::*;
mod picks;
pub use picks::*;
mod topology;
pub use topology::*;
mod reopen;
pub use reopen::*;
mod start;
pub use start::*;
mod apply;
pub use apply::*;

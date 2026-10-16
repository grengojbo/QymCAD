//! THE TOOLS OF THE SKETCH.
use qymcad::{Key, Modifiers};

use crate::contract::fixtures::Fixture;
use crate::contract::{Skip, Other, Budget, When, Class, Context, Entry, Field, Finish, Flow, Mode, Outcome, Pick, Tool, Under, Words};

/// Why the points of the contract that belong to a command do not apply to a tool that draws.
const DRAWS: &[Skip] = &[
    Skip { point: 3, why: "a drawing takes no reference: a click puts a point down, it does not pick something already there" },
    Skip { point: 4, why: "a drawing takes no reference, so there is no wrong one to refuse" },
    Skip { point: 13, why: "a drawn shape has no node in the tree to double-click: its dimensions are edited on the sheet (see the dimensions of B1)" },
    Skip { point: 14, why: "nothing of the timeline stands above a drawing but the plane of its sketch, which is checked with the sketch itself" },
    Skip { point: 15, why: "a drawing stands on its sketch alone: deleting the sketch takes the drawing with it, and that is checked with the sketch" },
    Skip { point: 17, why: "the kernel builds nothing here: a sketch is drawn in the plane, and the geometry holds whatever was clicked" },
];

/// The points above, and those of a tool with no field of its own.
const DRAWS_WITHOUT_FIELDS: &[Skip] = &[
    Skip { point: 3, why: DRAWS[0].why },
    Skip { point: 4, why: DRAWS[1].why },
    Skip { point: 5, why: "the tool has no field: what it draws is given by its clicks, and sized afterwards by a dimension" },
    Skip { point: 6, why: "the tool has no field to refuse a value in" },
    Skip { point: 13, why: DRAWS[2].why },
    Skip { point: 14, why: DRAWS[3].why },
    Skip { point: 15, why: DRAWS[4].why },
    Skip { point: 17, why: DRAWS[5].why },
];

mod draws;
pub use draws::*;
mod dims;
pub use dims::*;
mod cuts;
pub use cuts::*;
mod corners;
pub use corners::*;
mod projects;
pub use projects::*;
mod acts;
pub use acts::*;
mod constraints;
pub use constraints::*;
mod edits;
pub use edits::*;
mod reads;
pub use reads::*;

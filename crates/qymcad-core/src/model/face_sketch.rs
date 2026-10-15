//! A SKETCH TAKEN OFF A FACE, AND A BOSS OR A POCKET REDRAWN BY IT.
//!
//! A solid from a file has no sketch behind its bosses: to make one 40 wide instead of 30 the face has to be drawn again
//! by hand. Here the outline of the face is copied into a sketch on it as curves of its own - not a projection, it does
//! not follow the body and can be dragged - given the relations and sizes it shows until it is fully defined
//! (`fully_define`), and placed from the outline of the face it stands on, held as a projection: a person measures a
//! boss from the edge of the plate. Redrawn, the boss is taken off and laid again by the sketch, one node, and the sizes
//! of the sketch drive it from then on.
use super::{Anchor, Defined, Id, ProjSource, Prism, Project};
use crate::feature::{FaceKey, SketchPlane};

/// WHAT A SKETCH IS TAKEN OFF: the body, its flat face, the edges whose plan is the drawing (none: the outline of the
/// face), the edges the drawing is placed from (none: from the axes of the sketch), and the name the sketch is given.
pub struct FromFace<'a> {
    pub body: Id,
    pub face: FaceKey,
    pub profile: &'a [u32],
    pub outline: &'a [u32],
    pub name: String,
}

/// A SKETCH TAKEN OFF A FACE: its place among the sketches, its id, and what defining it laid.
#[derive(Clone, Debug)]
pub struct FaceSketch {
    pub si: usize,
    pub sketch: Id,
    pub defined: Defined,
}

impl Project {
    /// TAKE A SKETCH OFF A FLAT FACE: a sketch on the face, its outline copied in as curves of its own, the edges of
    /// `outline` projected in as construction lines it is placed from, and the drawing defined to the end. Answers
    /// `None` when the face gives no outline, and lays nothing then.
    pub fn add_sketch_from_face(&mut self, from: FromFace, kernel: &dyn crate::feature::Kernel) -> Option<FaceSketch> {
        let FromFace { body, face, profile, outline, name } = from;
        let si = self.new_sketch(name.clone());
        self.sketches[si].plane = SketchPlane::Face(body, face);
        // the drawing: the outline of the face, or the edges named for it - a rounded rim leaves the face smaller than
        // the boss, and the boss is drawn by where its walls stand
        let sources: Vec<ProjSource> = if profile.is_empty() { vec![ProjSource::Face(face.id)] } else { profile.iter().map(|&e| ProjSource::Edge(e)).collect() };
        let mut ents = Vec::new();
        for src in sources {
            let pid = self.add_sketch_projection(si, body, src, kernel);
            ents.extend(self.sketches[si].projections.iter().find(|p| p.id == pid).map(|p| p.entities.clone()).unwrap_or_default());
            self.release_sketch_projection(si, pid);
        }
        if ents.is_empty() {
            self.sketches.pop();
            return None;
        }
        // curves copied edge by edge meet at corners of their own: one point each, as a curve drawn on from another
        self.merge_close_points(si, 1e-6);
        let sketch = self.sketches[si].id;
        self.add_sketch_node(sketch, name);
        // the edges it is placed from, held by the body, drawn as construction: never a profile of its own
        let mut lines = Vec::new();
        for &edge in outline {
            let p = self.add_sketch_projection(si, body, ProjSource::Edge(edge), kernel);
            let held = self.sketches[si].projections.iter().find(|x| x.id == p).map(|x| x.entities.clone()).unwrap_or_default();
            for e in self.sketches[si].entities.iter_mut().filter(|e| held.contains(&e.id)) {
                e.construction = true;
                if matches!(e.kind, super::EntityKind::Line { .. }) {
                    lines.push(e.id);
                }
            }
        }
        let anchor = if lines.is_empty() { Anchor::Origin } else { Anchor::Lines(lines) };
        let defined = self.fully_define(si, &ents, &anchor);
        self.regen_sketch(si);
        Some(FaceSketch { si, sketch, defined })
    }

    /// A BOSS OR A POCKET OF BODY `src` REDRAWN BY `sketch`, as one node (`FeatureKind::Redrawn`): the faces of `prism`
    /// go by name, as a pick does, the height lives as the `height` dimension.
    pub fn add_redrawn(&mut self, src: Id, prism: &Prism, sketch: Id) -> Id {
        use crate::feature::{FeatureKind, FeatureNode};
        let body = self.alloc_id();
        let parent = Some(self.body_parent());
        let faces = crate::refs::Ref::picks(&prism.faces());
        // the widest face of the base, by its fingerprint: the plane the height is measured from
        let widest = self.regen_faces.get(&src).and_then(|fs| fs.iter().filter(|f| prism.base.contains(&f.id)).max_by(|a, b| a.area.total_cmp(&b.area)));
        let base = match widest {
            Some(f) => crate::refs::Ref::one(f.id, crate::refs::Fingerprint { centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal }),
            None => crate::refs::Ref::picks(&prism.base),
        };
        let kind = FeatureKind::Redrawn { src, faces, base, sketch, height: prism.height, kind: prism.kind, rims: prism.rims, body };
        let name = match prism.kind {
            super::PrismKind::Boss => "feat-name-redrawn-boss",
            super::PrismKind::Pocket => "feat-name-redrawn-pocket",
        };
        self.push_timeline(FeatureNode { id: body, name: name.into(), kind, parent, dirty: true, suppressed: false })
    }
}

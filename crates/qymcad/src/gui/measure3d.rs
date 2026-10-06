//! MEASURING IN 3D — resolving a click into the element being measured, and the tool itself.
//!
//! The arithmetic lives in the kernel (`qymcad_core::measure`) and is checked on known geometry;
//! what is here is only "what did the cursor hit". Measuring used to be possible ONLY in a sketch and
//! only between two points on a plane: the gap between parts, the distance between faces, the angle
//! of convergence and the diameter of a hole in 3D had nothing to measure them with.
pub use qymcad_ui_state::MeasurePick;
pub(crate) use qymcad_ui_state::measure_text;
use super::{App, Id};
use egui::{Pos2, Rect};
use qymcad_core::feature::apply12;
use qymcad_core::measure::MeasureItem;

impl App {
    /// THE RESULT TEXT — the only place where the numbers turn into a string (the status line and the
    /// plate at the geometry say the same thing: two wordings of one measurement drift apart
    /// silently).
    pub(super) fn measure_text(&self) -> String {
        measure_text(&self.painting())
    }

    /// SWITCH the 3D measuring tool ON or OFF.
    pub(super) fn toggle_measure_3d(&mut self) {
        let on = !self.side.m3.on;
        self.cancel_all_tools(); // exclusivity: the measuring tool puts down the previous one
        self.side.m3.clear();
        self.side.m3.on = on;
        if on {
            self.viewing.mode_3d = true;
            self.status = crate::i18n::tr("m3-hint");
        }
    }

    /// A click of the measuring tool: resolve the hit and update the result.
    pub(super) fn measure_3d_click(&mut self, rect: Rect, pos: Pos2) {
        let Some(p) = self.measure_resolve(rect, pos) else {
            self.status = crate::i18n::tr("m3-miss");
            return;
        };
        self.side.m3.take(p);
        self.status = if self.side.m3.picks.is_empty() { crate::i18n::tr("m3-hint") } else { self.measure_text() };
    }

    /// WHAT THE CURSOR HIT: vertex -> edge -> face.
    ///
    /// The order is exactly that (from small to large), as in every CAD: a vertex lies ON an edge and
    /// an edge on a face, so "the nearest thing of all" would always give the face, and neither a
    /// vertex nor an edge could ever be picked.
    pub(super) fn measure_resolve(&mut self, rect: Rect, pos: Pos2) -> Option<MeasurePick> {
        // WHAT IS OCCLUDED IS NOT PICKED. An isometric view folds the far bottom corner of a part
        // exactly onto the middle of its own top face: without a depth check a click on a visible face
        // returned THE EDGE ON THE FAR SIDE, and instead of the thickness of the part a diagonal came
        // out. A small element beats a face only if it is IN FRONT of it (or on it — the silhouette).
        let in_front = |me: &Self, w: [f64; 3]| crate::gui::pick::point_not_hidden(&me.painting(), rect, w);
        if let Some(w) = crate::gui::pick::pick_vertex_pos(&self.painting(), rect, pos) {
            if in_front(self, w) {
                return Some(MeasurePick { item: MeasureItem::Point(w), what: crate::i18n::tr("m3-vertex"), at: w });
            }
        }
        if let Some(p) = self.measure_edge_at(rect, pos) {
            if in_front(self, p.at) {
                return Some(p);
            }
        }
        self.measure_face_at(rect, pos)
    }

    /// The edge under the cursor -> a line or a circle in the WORLD coordinates of the active context.
    fn measure_edge_at(&mut self, rect: Rect, pos: Pos2) -> Option<MeasurePick> {
        let basis = self.viewing.cam.basis();
        let ctx = qymcad_ui_state::current_ctx_id(&self.active_path, &self.project);
        let mut best: Option<(f32, Id, u32)> = None;
        for (_mi, body) in crate::gui::pick::shown_bodies(&self.painting()) {
            if !crate::gui::pick::body_bbox_hit(&self.painting(), body, rect, pos, &basis, 12.0) {
                continue;
            }
            let Some(edges) = crate::gui::pick::body_edges_cached(&self.cache, &self.live, &self.regen, body) else { continue };
            let wt = self.project.body_display_transform(body, ctx);
            for (poly, id) in edges.polys.iter().zip(edges.ids.iter().copied()) {
                if id == 0 {
                    continue;
                }
                let pts: Vec<Pos2> =
                    poly.iter().map(|p| qymcad_ui_state::Screen { cam: &self.viewing.cam, set: &self.set, rect, basis: &basis }.at(apply12(&wt, [p[0] as f64, p[1] as f64, p[2] as f64])).0).collect();
                for w in pts.windows(2) {
                    let d = super::screen_dist_seg(pos, w[0], w[1]);
                    if best.is_none_or(|(bd, _, _)| d < bd) {
                        best = Some((d, body, id));
                    }
                }
            }
        }
        let (d, body, eid) = best.filter(|(d, _, _)| *d <= 8.0)?;
        let _ = d;
        let shape = self.live.shapes.get(&body)?;
        let wt = self.project.body_display_transform(body, ctx);
        let item = qymcad_doc::measure::edge_item(shape, eid, &wt)?;
        let what = match item {
            MeasureItem::Circle { r, .. } => crate::i18n::tr1("m3-circle", "d", &crate::i18n::num(2.0 * r, 2)),
            MeasureItem::Line { len, .. } => crate::i18n::tr1("m3-edge", "v", &crate::i18n::num(len, 2)),
            _ => return None,
        };
        Some(MeasurePick { item, what, at: qymcad_doc::measure::shown_at(&item) })
    }

    /// The face under the cursor -> a plane or a cylinder in the WORLD coordinates of the active
    /// context.
    fn measure_face_at(&mut self, rect: Rect, pos: Pos2) -> Option<MeasurePick> {
        let (body, fid, hit) = crate::gui::pick::pick_face_ray(&self.painting(), rect, pos)?;
        let ctx = qymcad_ui_state::current_ctx_id(&self.active_path, &self.project);
        let wt = self.project.body_display_transform(body, ctx);
        let item = qymcad_doc::measure::face_item(&self.project, self.live.shapes.get(&body), body, fid, &wt);
        let what = match item {
            MeasureItem::Cylinder { r, .. } => crate::i18n::tr1("m3-cylinder", "d", &crate::i18n::num(2.0 * r, 2)),
            _ => crate::i18n::tr("m3-face"),
        };
        Some(MeasurePick { item, what, at: hit })
    }
}

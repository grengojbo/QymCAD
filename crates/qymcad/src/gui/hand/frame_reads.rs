//! WHAT THE LAST FRAME DREW, read back: segments and filled triangles of a colour, rows lit as selected, strokes near
//! the pointer.

impl super::Hand<'_> {
    /// THE SEGMENTS OF `colour` THE LAST FRAME DREW, where they stand on screen: a dashed line is drawn as its dashes.
    pub fn segments_in(&self, colour: egui::Color32) -> Vec<[egui::Pos2; 2]> {
        fn walk(s: &egui::Shape, colour: egui::Color32, out: &mut Vec<[egui::Pos2; 2]>) {
            match s {
                egui::Shape::LineSegment { points, stroke } if stroke.color == colour => out.push(*points),
                egui::Shape::Vec(v) => v.iter().for_each(|x| walk(x, colour, out)),
                _ => {}
            }
        }
        let mut out = Vec::new();
        self.win.shapes.iter().for_each(|cs| walk(&cs.shape, colour, &mut out));
        out
    }

    /// HOW MANY CORNERS OF FILLED TRIANGLES IN `colour` THE LAST FRAME DREW: a lit face is drawn as its triangles.
    pub fn mesh_corners_in(&self, colour: egui::Color32) -> usize {
        fn walk(s: &egui::Shape, colour: egui::Color32) -> usize {
            match s {
                egui::Shape::Mesh(m) => m.vertices.iter().filter(|v| v.color == colour).count(),
                egui::Shape::Vec(v) => v.iter().map(|x| walk(x, colour)).sum(),
                _ => 0,
            }
        }
        self.win.shapes.iter().map(|cs| walk(&cs.shape, colour)).sum()
    }

    /// WHETHER THE ROW WHOSE WORDS HOLD `word` STANDS LIT AS SELECTED, the next frame drawn: a filled rectangle in the
    /// colour of the selection lies under where it is written.
    pub fn row_lit(&mut self, word: &str) -> bool {
        fn fills(s: &egui::Shape, at: egui::Pos2, colour: egui::Color32) -> bool {
            match s {
                egui::Shape::Rect(r) => r.fill == colour && r.rect.contains(at),
                egui::Shape::Vec(v) => v.iter().any(|x| fills(x, at, colour)),
                _ => false,
            }
        }
        self.frame(Vec::new());
        let colour = self.win.ctx.global_style().visuals.selection.bg_fill;
        let Some(at) = self.win.drawn.iter().find(|(t, _)| t.contains(word)).map(|(_, r)| r.center()) else { return false };
        self.win.shapes.iter().any(|cs| fills(&cs.shape, at, colour))
    }

    /// HOW MANY STROKES OF `colour` THE FRAME DRAWS NEAR THE POINTER, the hand resting over `place` of the sketch: a
    /// path or a segment with a point within 40 px of it. What follows the pointer is drawn there.
    pub fn strokes_near2d(&mut self, place: (f64, f64), colour: egui::Color32) -> usize {
        fn near(s: &egui::Shape, at: egui::Pos2, colour: egui::Color32) -> usize {
            match s {
                egui::Shape::Path(p) if p.stroke.color == egui::epaint::ColorMode::Solid(colour) && p.points.iter().any(|q| q.distance(at) < 40.0) => 1,
                egui::Shape::LineSegment { points, stroke } if stroke.color == colour && points.iter().any(|q| q.distance(at) < 40.0) => 1,
                egui::Shape::Vec(v) => v.iter().map(|x| near(x, at, colour)).sum(),
                _ => 0,
            }
        }
        // on the sheet of the sketch, as the other `...2d` hands are: a new hand starts in the 3D view
        self.app.viewing.mode_3d = false;
        let at = self.rest_over2d(place);
        let n = self.win.shapes.iter().map(|cs| near(&cs.shape, at, colour)).sum();
        self.close_window();
        n
    }
}

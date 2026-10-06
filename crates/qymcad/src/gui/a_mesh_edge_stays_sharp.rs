//! A MESH'S SHARP EDGE STAYS SHARP ON BOTH PAINTERS. A mesh from a file shares its vertices across its edges, and
//! light smoothed over every shared vertex made a cube bulge like a cushion: a gradient over every face on the
//! software picture, eight vertices lit along the diagonals in the device's scene.
//!
//! Reported behaviour, measured: the owner's print head from PLY showed streaks along the edges and fillets of the
//! housing, 0.79 % of the picture apart from the head from STEP, where the same triangles with vertices of their own
//! came 0.03 % apart.
#[cfg(test)]
mod tests {
    use crate::gui::check_folder::tests::CheckFolder;
    use crate::gui::import_door::tests::{answer, cube_stl, frame, key, running, settle};
    use crate::gui::App;
    use qymcad_ui_state::Want;

    /// Which check reads the cube. libtest runs the checks of a module side by side, and the door reads the file on
    /// a thread of its own after the check has written it: a file shared by two checks is read while the other one
    /// truncates it, and the cube comes in empty.
    #[derive(Clone, Copy)]
    enum Check {
        Scene,
        Raster,
        Read,
        Rewritten,
    }

    /// The folder the cube of `check` is written to, one per check and run; the cube is `sharp.stl` in it.
    fn cube_folder(check: Check) -> CheckFolder {
        CheckFolder::new(match check {
            Check::Scene => "sharp-scene",
            Check::Raster => "sharp-raster",
            Check::Read => "sharp-read",
            Check::Rewritten => "sharp-rewritten",
        })
    }

    /// A cube from a text STL through the door, seen in 3D: its corners welded, every one shared by three faces.
    fn the_cube(check: Check) -> App {
        let folder = cube_folder(check);
        let p = folder.file("sharp.stl");
        std::fs::write(&p, cube_stl(10.0)).expect("written");
        let (mut app, ctx) = running();
        answer(&mut app, &ctx, Want::Anything, &p.to_string_lossy());
        settle(&mut app, &ctx);
        let _ = frame(&mut app, &ctx, key(egui::Key::Enter)); // the window about the unit, as the file has it
        let _ = frame(&mut app, &ctx, Vec::new());
        assert_eq!(app.project.bodies.iter().map(|b| b.mesh.verts.len()).collect::<Vec<_>>(), [8], "the cube's corners are not welded");
        app.viewing.mode_3d = true;
        app.viewing.cam.init = true;
        app
    }

    /// THE DEVICE'S SCENE: every corner of a face lit by the face - the 36 corners of the cube's 12 triangles as 24
    /// points with a normal of their own (a corner of the cube once for each of its three faces), none along a diagonal.
    #[test]
    fn the_scene_for_the_device_keeps_a_cubes_edges_sharp() {
        let app = the_cube(Check::Scene);
        let (verts, _) = crate::gui::render_scene::gpu_scene_flat(&app.painting());
        let axes = |v: &qymcad_ui_state::GpuVert| v.nrm.to_le_bytes()[..3].iter().filter(|b| (**b as i8).unsigned_abs() > 20).count();
        let diagonal = verts.iter().filter(|v| axes(v) > 1).count();
        let lit: std::collections::HashSet<([u32; 3], u32)> = verts.iter().map(|v| (v.pos.map(f32::to_bits), v.nrm)).collect();
        assert_eq!((verts.len(), lit.len(), diagonal), (36, 24, 0), "the cube's {} corners go to the device as {} lit points, {diagonal} corners lit along a diagonal", verts.len(), lit.len());
    }

    /// THE SOFTWARE PICTURE: a face in one shade, not a gradient.
    #[test]
    fn the_software_raster_paints_a_cubes_faces_flat() {
        let mut app = the_cube(Check::Raster);
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(700.0, 600.0));
        crate::gui::fit3d(&mut app.viewing.cam, &app.project, rect);
        let basis = app.viewing.cam.basis();
        let img = qymcad_render::rasterize_3d(&app.painting(), rect, &basis, 1.0, 1.0).expect("a raster");
        let shades: std::collections::HashSet<[u8; 3]> = img.pixels.iter().filter(|c| c.a() == 255).map(|c| [c.r(), c.g(), c.b()]).collect();
        let lit = img.pixels.iter().filter(|c| c.a() == 255).count();
        eprintln!("the cube: {lit} pixels in {} shades", shades.len());
        assert!(lit > 1000 && shades.len() <= 12, "the cube's {lit} pixels are painted in {} shades", shades.len());
    }

    /// ONE CHECK DOES NOT READ ANOTHER'S FILE. Reported behaviour: the scene check failed once in a whole run and never
    /// alone - the raster check beside it rewrote the same file while the door was reading it. Here the file of
    /// another check is rewritten without a pause while the cube is read 20 times; with one file for both, 1 to 7 of
    /// the 20 cubes came in with no vertex at all, in each of 10 runs.
    #[test]
    fn a_cube_comes_in_whole_while_another_check_writes_its_file() {
        let theirs = cube_folder(Check::Rewritten);
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let other = {
            let stop = stop.clone();
            let p = theirs.file("sharp.stl");
            std::thread::spawn(move || {
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = std::fs::write(&p, cube_stl(10.0));
                }
            })
        };
        let empty = (0..20).filter(|_| std::panic::catch_unwind(|| the_cube(Check::Read)).is_err()).count();
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        other.join().expect("the other check's writer");
        assert_eq!(empty, 0, "{empty} of 20 cubes did not come in whole while another check wrote its file");
    }
}

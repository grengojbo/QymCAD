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
        Triangles,
        Recognised,
    }

    /// The folder the cube of `check` is written to, one per check and run; the cube is `sharp.stl` in it.
    fn cube_folder(check: Check) -> CheckFolder {
        CheckFolder::new(match check {
            Check::Scene => "sharp-scene",
            Check::Raster => "sharp-raster",
            Check::Read => "sharp-read",
            Check::Rewritten => "sharp-rewritten",
            Check::Triangles => "sharp-triangles",
            Check::Recognised => "sharp-recognised",
        })
    }

    /// A cube from a text STL through the door, seen in 3D: its corners welded, every one shared by three faces.
    fn the_cube(check: Check) -> App {
        the_cube_running(check).app
    }

    /// A cube in the window, and the frames it runs in.
    struct Running {
        app: App,
        ctx: egui::Context,
    }

    /// The same cube, with the frames it runs in.
    fn the_cube_running(check: Check) -> Running {
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
        Running { app, ctx }
    }

    /// THE DEVICE'S SCENE: every corner of a face lit by the face - the 36 corners of the cube's 12 triangles as 24
    /// points with a normal of their own (a corner of the cube once for each of its three faces), none along a diagonal.
    #[test]
    fn the_scene_for_the_device_keeps_a_cubes_edges_sharp() {
        let app = the_cube(Check::Scene);
        let (verts, _) = crate::gui::render_scene::gpu_scene_flat(&app.painting());
        // how many axes the normal leans along: one for a face of the cube, more for a corner lit along a diagonal
        let axes = |v: &qymcad_ui_state::GpuVert| qymcad_core::geom::unpack_direction(v.nrm).map_or(0, |n| n.iter().filter(|c| c.abs() > 0.16).count());
        let diagonal = verts.iter().filter(|v| axes(v) > 1).count();
        let lit: std::collections::HashSet<([u32; 3], u32)> = verts.iter().map(|v| (v.pos.map(f32::to_bits), v.nrm)).collect();
        assert_eq!((verts.len(), lit.len(), diagonal), (36, 24, 0), "the cube's {} corners go to the device as {} lit points, {diagonal} corners lit along a diagonal", verts.len(), lit.len());
    }

    /// THE SOFTWARE PICTURE: a face in one shade, not a gradient.
    #[test]
    fn the_software_raster_paints_a_cubes_faces_flat() {
        let mut app = the_cube(Check::Raster);
        // the shading alone is counted: the dark lines of the edges are a shade of their own
        app.set.body_edges = qymcad_ui_state::BodyEdges::Hidden;
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(700.0, 600.0));
        crate::gui::fit3d(&mut app.viewing.cam, &app.project, rect);
        let basis = app.viewing.cam.basis();
        let img = qymcad_render::rasterize_3d(&app.painting(), rect, &basis, 1.0, 1.0).expect("a raster");
        let shades: std::collections::HashSet<[u8; 3]> = img.pixels.iter().filter(|c| c.a() == 255).map(|c| [c.r(), c.g(), c.b()]).collect();
        let lit = img.pixels.iter().filter(|c| c.a() == 255).count();
        eprintln!("the cube: {lit} pixels in {} shades", shades.len());
        assert!(lit > 1000 && shades.len() <= 12, "the cube's {lit} pixels are painted in {} shades", shades.len());
    }

    /// How dark the picture of `app` is at the middle of the cube's top, (5, 5, 10): the darkest pixel within 1 of it,
    /// its three channels summed. The middle lies on the diagonal the two triangles of the top share, whichever it is.
    fn the_middle_of_the_top(app: &mut App) -> u32 {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(700.0, 600.0));
        crate::gui::fit3d(&mut app.viewing.cam, &app.project, rect);
        let basis = app.viewing.cam.basis();
        let img = qymcad_render::rasterize_3d(&app.painting(), rect, &basis, 1.0, 1.0).expect("a raster");
        let at = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect, basis: &basis }.at([5.0, 5.0, 10.0]).0;
        let mut darkest = u32::MAX;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y) = ((at.x as i32 + dx) as usize, (at.y as i32 + dy) as usize);
                if let Some(c) = img.pixels.get(y * img.size[0] + x).filter(|_| x < img.size[0]) {
                    darkest = darkest.min(c.r() as u32 + c.g() as u32 + c.b() as u32);
                }
            }
        }
        darkest
    }

    /// "MESH TRIANGLES" FROM THE VIEW MENU DRAWS THE TRIANGLES OF A MESH: the diagonal across the top of a cube from STL,
    /// a side two of its triangles share and no edge of the cube, comes out a dark line once it is chosen, and not
    /// before.
    #[test]
    fn the_view_menu_draws_the_triangles_of_a_mesh() {
        let mut app = the_cube(Check::Triangles);
        let before = the_middle_of_the_top(&mut app);
        {
            let mut hand = crate::gui::hand::Hand::new(&mut app);
            assert!(hand.press_word(&qymcad_i18n::tr("menu-view"), egui::pos2(0.0, 0.0)), "the bar shows no View menu");
            assert!(hand.press_word(&qymcad_i18n::tr("menu-mesh-lines"), egui::pos2(0.0, 0.0)), "the View menu shows no mesh triangles");
        }
        assert_eq!(app.set.mesh_lines, qymcad_ui_state::MeshLines::Shown, "the menu did not turn the triangles on");
        let after = the_middle_of_the_top(&mut app);
        assert!(after + 60 < before, "no dark line along the diagonal of the top: {after} with the triangles, {before} without");
    }

    /// A CUBE RECOGNISED WHOLE SHOWS NO TRIANGLES: each of its sides an exact plane the kernel meshes as two triangles,
    /// none a face of one triangle left as mesh.
    #[test]
    fn a_cube_recognised_whole_shows_no_triangles() {
        let Running { mut app, ctx } = the_cube_running(Check::Recognised);
        let piece = app.project.bodies[0].id;
        app.project.add_mesh_recognised(piece, 1.0, 30.0, 0.0);
        // a recognition runs on a worker, as in the window: frames until it lands
        crate::gui::session::waiting::until("the recognition to land", || {
            let _ = frame(&mut app, &ctx, Vec::new());
            app.regen.busy.is_none() && app.project.timeline.iter().all(|n| !n.dirty)
        });
        let body = app.project.bodies.iter().find(|b| b.id != piece).expect("the recognised body");
        assert_eq!(body.faces.len(), 6, "the cube is not recognised as six faces");
        let lines = qymcad_ui_state::mesh_lines_of(body, qymcad_ui_state::MadeAs::Faces);
        assert!(lines.is_empty(), "{} lines of a mesh over a cube recognised whole", lines.len());
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

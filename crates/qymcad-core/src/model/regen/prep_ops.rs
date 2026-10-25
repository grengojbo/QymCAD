//! THE OPERATIONS ON A BODY PREPARED: fillets, chamfers, shells, faces, drafts, mirrors, moves, primitives, lofts.

use super::*;

impl Project {
    /// A FILLET: round the picked edges of body `src` by `radius`, with a variable radius at named vertices.
    ///
    /// Returns whether the node's error record may be cleared (see `apply_regen`).
    pub(crate) fn prep_fillet(&mut self, p: &mut Pass, src: Id, radius: f64, edges: &crate::refs::Ref, at_vertices: &[(crate::refs::Ref, f64)], body: Id) -> crate::feature::KernelJob {
        let radius = p.dim("radius", radius);
        // Two paths, and the difference between them is fundamental.
        //
        // A hand-picked set holds recorded edge names, and an edge name is derived from its pair of
        // faces and changes with them. Such references have to be translated through the rename map
        // of the source body — a proven mechanism that must not be disturbed; fillets in a real
        // document once went red on it.
        //
        // A descriptive query ("every edge of this face") stores no names at all, so there is
        // nothing to translate: it asks today's geometry directly.
        //
        // A reference that resolved to nothing is not "every edge". The kernel fillets the whole
        // body when the list is empty, which is correct for the query "fillet everything". But the
        // same empty list arises when one edge was picked and its reference stopped resolving:
        // measured on a box, a fillet over a non-existent edge produced 26 faces — exactly as many
        // as filleting all twelve — while the node stayed green. One edge was asked for, the whole
        // part was rounded, and the timeline said nothing.
        //
        // The distinction is made by the query rather than by the result: when descriptors were
        // named and no live edges were found, the reference is lost and that is reported.
        let asked_count = asked_edge_count(edges);
        let asked_edges = asked_count > 0 || !edges.query.is_pick_list();
        let lost_error = edges_lost(edges, asked_count);
        // A PART OF THE PICKED EDGES GONE - a cut above took one away - the rest are done and the node says, in yellow,
        // how many were left: it is neither a failure of the whole nor to be kept quiet. Reported behaviour: a rounding
        // of four edges stood green with three after a cut above took the fourth.
        self.regen_warnings.remove(&p.node);
        let partly_lost = if asked_count > 0 { self.edge_refs_lost(p.node, src, &edges.query.picked_descs(), p.emap, p.kernel) } else { 0 };
        let picked = self.picked_edges(super::apply::BlendOn { node: p.node, src }, edges, p.emap, p.kernel);
        let edges = &self.live_fillet_edges(p.node, src, edges, p.emap, p.kernel);
        let lost_edges = asked_edges && edges.is_empty();
        if partly_lost > 0 && !edges.is_empty() {
            self.regen_warnings.insert(p.node, crate::errors::CoreError::EdgesDropped { asked: asked_count, dropped: partly_lost });
        }
        // A variable radius is specified at vertices. Each table entry is resolved as a reference:
        // a vertex name is derived from its edges, so editing the neighbours does not disturb it.
        // The value is parametric like the radius itself, under the dimension key
        // `at{descriptor}`.
        let verts: Vec<crate::feature::RadiusAt> = at_vertices
            .iter()
            .filter_map(|(r, val)| {
                let desc = self.resolve_vertex_refs(src, r, "ref-what-fillet-vertex").ok()?.first().copied()?;
                let radius = p.dim(&format!("at{desc}"), *val);
                self.vertex_point(src, desc).map(|at| crate::feature::RadiusAt { at, radius })
            })
            .collect();
        // A solid tool does not operate on a surface. Otherwise it appears to work: the node goes
        // red while a degenerate two-triangle body is left next to the part (found by the fuzzer).
        let on_sheet = p.kernel.body_is_sheet(src);
        let edges: Vec<u32> = edges.to_vec();
        if on_sheet {
            crate::feature::KernelJob::refused(crate::errors::CoreError::NeedsSolidNotSheet)
        } else if lost_edges {
            crate::feature::KernelJob::refused(lost_error)
        } else if !verts.is_empty() && !edges.is_empty() {
            // the faces it makes are named by the bridge, after this node, as a fillet of one radius names them
            let roles = crate::feature::BlendRoles::Named { feature: p.node, picked: picked.clone() };
            crate::feature::KernelJob::new(vec![src], move |k| k.fillet_at_vertices(body, src, radius, &edges, &verts, roles))
        } else {
            // the faces it makes are named by the bridge, after this node
            let roles = crate::feature::BlendRoles::Named { feature: p.node, picked: picked.clone() };
            crate::feature::KernelJob::new(vec![src], move |k| k.fillet(body, src, radius, &edges, roles))
        }
    }

    /// Chamfer: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_chamfer(&mut self, p: &mut Pass, src: Id, dist: f64, edges: &crate::refs::Ref, shape: ChamferShape, body: Id) -> crate::feature::KernelJob {
        let ChamferShape { mode, d2, flip, ref_face } = shape;
        let dist = p.dim("dist", dist);
        let d2 = p.dim("d2", d2);
        // As for a fillet: an empty list means the whole part, and a lost reference must not
        // masquerade as that.
        let asked_count = asked_edge_count(edges);
        let asked_edges = asked_count > 0 || !edges.query.is_pick_list();
        let lost_error = edges_lost(edges, asked_count);
        // A PART OF THE PICKED EDGES GONE - a cut above took one away - the rest are done and the node says, in yellow,
        // how many were left: it is neither a failure of the whole nor to be kept quiet. Reported behaviour: a rounding
        // of four edges stood green with three after a cut above took the fourth.
        self.regen_warnings.remove(&p.node);
        let partly_lost = if asked_count > 0 { self.edge_refs_lost(p.node, src, &edges.query.picked_descs(), p.emap, p.kernel) } else { 0 };
        let picked = self.picked_edges(super::apply::BlendOn { node: p.node, src }, edges, p.emap, p.kernel);
        let edges = &self.live_fillet_edges(p.node, src, edges, p.emap, p.kernel);
        let lost_edges = asked_edges && edges.is_empty();
        if partly_lost > 0 && !edges.is_empty() {
            self.regen_warnings.insert(p.node, crate::errors::CoreError::EdgesDropped { asked: asked_count, dropped: partly_lost });
        }
        // Asymmetry (two setbacks, or setback plus angle) applies only to an explicit edge
        // selection; otherwise the chamfer is symmetric.
        let edges: Vec<u32> = edges.to_vec();
        if lost_edges {
            crate::feature::KernelJob::refused(lost_error)
        } else if mode != crate::feature::ChamferMode::Symmetric && !edges.is_empty() {
            // the faces it makes are named by the bridge, after this node, as a symmetric chamfer names them
            let roles = crate::feature::BlendRoles::Named { feature: p.node, picked: picked.clone() };
            crate::feature::KernelJob::new(vec![src], move |k| k.chamfer_ex(body, src, dist, ChamferShape { mode, d2, flip, ref_face }, &edges, roles))
        } else {
            // the faces it makes are named by the bridge, after this node: the same rule as for a fillet
            let roles = crate::feature::BlendRoles::Named { feature: p.node, picked: picked.clone() };
            crate::feature::KernelJob::new(vec![src], move |k| k.chamfer(body, src, dist, &edges, roles))
        }
    }

    /// Shell: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_shell(&mut self, p: &mut Pass, src: Id, thickness: f64, faces: &crate::refs::Ref, side: crate::feature::ShellSide, body: Id) -> crate::feature::KernelJob {
        let feature = p.node;
        let thickness = p.dim("thickness", thickness);
        // Names of the inner walls come from the recipe: a wall is produced by offsetting a face
        // and, without a name of its own, takes the name of the outer face, so a reference to an
        // inner edge lands on the outer one.
        // Resolution goes through the query, and a refusal stops the operation: opening the wrong
        // face is worse than opening none, since the body then looks similar with the hole in the
        // wrong place.
        //
        // Asking for two faces and opening one is not success. Measured: a shell given two faces
        // where one reference did not resolve opened one and stayed green (11 faces instead of 10).
        // The part looks similar but is closed on the side where an opening was expected. Only a
        // hand-picked set is checked this way: for a descriptive query the number of descriptors
        // says nothing about the result.
        let asked_faces = if faces.query.is_pick_list() { faces.query.picked_descs().len() } else { 0 };

        match self.faces_by_ref(p.node, src, faces, "ref-what-shell-faces") {
            Err(_) => crate::feature::KernelJob::refused(crate::errors::CoreError::FacesNotFound),
            Ok(ids) if asked_faces > 0 && ids.len() < asked_faces => crate::feature::KernelJob::refused(crate::errors::CoreError::FacesNotFound),
            Ok(ids) => match side {
                crate::feature::ShellSide::Centred => crate::feature::KernelJob::new(vec![src], move |k| k.shell_center(body, src, thickness, &ids)),
                side => crate::feature::KernelJob::new(vec![src], move |k| k.shell_named(body, src, thickness, side == crate::feature::ShellSide::Outward, &ids, feature)),
            },
        }
    }

    pub(crate) fn prep_removeface(&mut self, p: &Pass, src: Id, faces: &crate::refs::Ref, body: Id) -> crate::feature::KernelJob {
        match self.faces_by_ref(p.node, src, faces, "ref-what-removed-faces") {
            Ok(ids) if !ids.is_empty() => crate::feature::KernelJob::new(vec![src], move |k| k.remove_faces(body, src, &ids)),
            _ => crate::feature::KernelJob::refused(crate::errors::CoreError::FacesNotFound),
        }
    }

    /// A FEATURE MADE AGAIN EXACT, in one job: its faces taken off `src` into `body`, then the hole drilled, or the edge
    /// the two sides meet in once healed rounded or chamfered, on `body` itself. That edge is born of the removal and has
    /// no name yet: it is found in the job by the two faces the kernel says it parts.
    /// THE FEATURES OF A NODE MADE EXACT: the faces of all of them taken off at once, then the holes drilled, the chamfers
    /// cut a group at a time, the roundings laid a group at a time, the larger radius first - a rounding of 2 running
    /// round a rounding of 4 at a corner was laid after it, and only so does it come out as it was. A group of blends
    /// is one call of its tool on the edges of all its features: a chain of roundings tangent to each other is laid
    /// along at once.
    pub(crate) fn prep_remade(&mut self, p: &Pass, src: Id, items: &[RemadeItem], body: Id) -> crate::feature::KernelJob {
        let mut ids = Vec::new();
        for it in items {
            match self.faces_by_ref(p.node, src, &it.faces, "ref-what-removed-faces") {
                Ok(found) if !found.is_empty() => ids.extend(found),
                _ => return crate::feature::KernelJob::refused(crate::errors::CoreError::FacesNotFound),
            }
        }
        let mut holes: Vec<Drilled> = Vec::new();
        let mut cuts: Vec<Laid> = Vec::new();
        let mut rounds: Vec<Laid> = Vec::new();
        for it in items {
            match it.exact {
                Exact::Hole { entry, at, diameter, depth, ref rims, .. } => {
                    let tool = HoleTool { kind: 0, diameter: p.dim(&group_key("diameter", it.group), diameter), depth: p.dim(&group_key("depth", it.group), depth).abs(), dia2: 0.0, depth2: 0.0 };
                    // the hole stands where it was found, brought onto the entry face as it stands now
                    let face = crate::refs::Ref::one(entry.id, crate::refs::Fingerprint { centroid: entry.centroid, normal: entry.normal });
                    let Ok(c) = self.face_by_ref(p.node, src, &face, "ref-what-hole-face") else { return crate::feature::KernelJob::refused(crate::errors::CoreError::FaceNotFound) };
                    let off = (0..3).map(|i| (at[i] - c.centroid[i]) * c.normal[i]).sum::<f64>();
                    let point = [at[0] - off * c.normal[0], at[1] - off * c.normal[1], at[2] - off * c.normal[2]];
                    holes.push(Drilled { group: it.group, tool, pl: crate::feature::PlaneFrame::from_origin_normal(point, c.normal, 0.0).matrix12(), rims: rims.clone() });
                }
                Exact::Chamfer { sides, leg } => lay_in(&mut cuts, it.group, p.dim(&group_key("dist", it.group), leg), sides),
                Exact::Round { sides, radius } => lay_in(&mut rounds, it.group, p.dim(&group_key("radius", it.group), radius), sides),
            }
        }
        rounds.sort_by(|a, b| b.size.total_cmp(&a.size).then(a.group.cmp(&b.group)));
        let feature = p.node;
        let roles = crate::feature::BlendRoles::Named { feature, picked: Vec::new() };
        crate::feature::KernelJob::new(vec![src], move |k| {
            let mut built = k.remove_faces(body, src, &ids)?;
            // what refuses after the faces are off is told with its group, so the tool can leave that group out
            for h in &holes {
                // the faces standing before the drill: the mouth of the hole is where a face of these meets one it made
                let before: std::collections::HashSet<u32> = k.edge_face_pairs(body).iter().flat_map(|p| p.faces).collect();
                built = k.hole(body, body, h.tool, h.pl, feature).map_err(|e| in_group(e, h.group))?;
                for rim in &h.rims {
                    let edges = mouth_of(k, body, rim.face, &before);
                    if edges.is_empty() {
                        return Err(in_group(crate::errors::CoreError::DescribedEdgesNotFound, h.group));
                    }
                    built = match rim.blend.cut {
                        crate::model::RimCut::Round => k.fillet(body, body, rim.blend.size, &edges, roles.clone()),
                        crate::model::RimCut::Chamfer => k.chamfer(body, body, rim.blend.size, &edges, roles.clone()),
                    }
                    .map_err(|e| in_group(e, h.group))?;
                }
            }
            for c in &cuts {
                let edges = c.sides.iter().map(|&s| parted_by(k, body, s)).collect::<Result<Vec<u32>, _>>().map_err(|e| in_group(e, c.group))?;
                built = k.chamfer(body, body, c.size, &edges, roles.clone()).map_err(|e| in_group(e, c.group))?;
            }
            for r in &rounds {
                let edges = r.sides.iter().map(|&s| parted_by(k, body, s)).collect::<Result<Vec<u32>, _>>().map_err(|e| in_group(e, r.group))?;
                built = k.fillet(body, body, r.size, &edges, roles.clone()).map_err(|e| in_group(e, r.group))?;
            }
            Ok(built)
        })
    }

    pub(crate) fn prep_patch(&mut self, p: &Pass, src: Id, edges: &crate::refs::Ref, tangent: bool, body: Id) -> crate::feature::KernelJob {
        let edges = edges.clone();
        let live = self.live_fillet_edges(p.node, src, &edges, p.emap, p.kernel);
        if live.is_empty() {
            return crate::feature::KernelJob::refused(edges_lost(&edges, asked_edge_count(&edges)));
        }
        let feature = p.node;
        crate::feature::KernelJob::new(vec![src], move |k| k.patch(body, src, &live, tangent, feature))
    }

    pub(crate) fn prep_trim(&mut self, src: Id, tool: Id, keep: [f64; 3], body: Id) -> crate::feature::KernelJob {
        crate::feature::KernelJob::new(vec![src, tool], move |k| k.trim(body, src, tool, keep))
    }

    pub(crate) fn prep_stitch(&mut self, p: &Pass, parts: &[Id], tol: f64, body: Id) -> crate::feature::KernelJob {
        let t = p.dim("tol", tol);
        let parts = parts.to_vec();
        if parts.len() < 2 {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::OpFailed(crate::errors::Op::Stitch));
        }
        crate::feature::KernelJob::new(parts.clone(), move |k| k.stitch(body, &parts, t))
    }

    /// The mesh is the node's recipe: it travels with the work, and no live shape is read.
    // the mesh is made lighter inside the job, off the frame: a mesh of 178 032 triangles takes seconds
    pub(crate) fn prep_mesh_solid(&mut self, src: Id, body: Id, simplify: f64) -> crate::feature::KernelJob {
        let Some(mesh) = self.mesh_index(src).map(|i| self.bodies[i].mesh.clone()) else {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::SourceBodyNotBuilt);
        };
        crate::feature::KernelJob::new(Vec::new(), move |k| k.mesh_solid(body, &lighter(mesh, simplify)))
    }

    pub(crate) fn prep_mesh_recognised(&mut self, src: Id, body: Id, tol: f64, sharp: f64, simplify: f64) -> crate::feature::KernelJob {
        let Some(mesh) = self.mesh_index(src).map(|i| self.bodies[i].mesh.clone()) else {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::SourceBodyNotBuilt);
        };
        crate::feature::KernelJob::new(Vec::new(), move |k| {
            let mesh = lighter(mesh, simplify);
            let tol = within_simplification(&mesh, tol, simplify);
            k.mesh_recognised(body, &mesh, tol, sharp)
        })
    }

    /// A ROUNDING OR A CHAMFER MADE EXACT BESIDE THE PUSHED FACE KEEPS ITS SIZE: its face is taken off, the face pushed,
    /// and the feature laid down again on the edge its two faces meet in, the size its own node holds now and under the
    /// names that node gave it. Pushed with the face on, the blend's face follows the push as any face beside does.
    /// Reported behaviour, measured: the side beside a chamfer of leg 2 pushed out by 10 stretched the chamfer's face
    /// from 28.3 to 169.7 mm^2; on a mesh made a body the strips of a rounding run nearly along the face pushed, and a
    /// push above the rounding drew them out past the face itself.
    pub(crate) fn prep_pushface(&mut self, p: &Pass, src: Id, face: &crate::refs::Ref, dist: f64, body: Id) -> crate::feature::KernelJob {
        let dist = eval_dim(p.dims, "dist", dist, p.vars);
        if p.kernel.body_is_sheet(src) {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::PushFaceOnSheet);
        }
        match self.face_by_ref(p.node, src, face, "ref-what-pushed-face") {
            Err(_) => crate::feature::KernelJob::refused(crate::errors::CoreError::FaceNotFound),
            Ok(_) if dist.abs() < 1e-9 => crate::feature::KernelJob::refused(crate::errors::CoreError::ZeroPushDistance),
            Ok(c) => {
                let carried = self.blends_beside(p, src, c.desc);
                pushed_job(src, body, vec![Shift { face: c.desc, by: dist }], carried)
            }
        }
    }

    /// A SIZE BETWEEN TWO FLAT FACES: the faces resolved, the distance between their planes measured along the normal
    /// of the first, and the face or faces `moves` names pushed so the distance becomes `gap` - the rest as a push does
    /// it, the roundings and chamfers made exact beside them carried across.
    pub(crate) fn prep_facegap(&mut self, p: &Pass, src: Id, faces: GapRefs, gap: f64, moves: crate::feature::GapMoves, body: Id) -> crate::feature::KernelJob {
        let gap = p.dim("gap", gap);
        if p.kernel.body_is_sheet(src) {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::PushFaceOnSheet);
        }
        let (Ok(a), Ok(b)) = (self.face_by_ref(p.node, src, faces.one, "ref-what-gap-face"), self.face_by_ref(p.node, src, faces.other, "ref-what-gap-face")) else {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::FaceNotFound);
        };
        let n = a.normal;
        let turn = n[0] * b.normal[0] + n[1] * b.normal[1] + n[2] * b.normal[2];
        let apart = (0..3).map(|i| n[i] * (b.centroid[i] - a.centroid[i])).sum::<f64>();
        // the faces and the number refused apart, as for a hole moved
        if turn.abs() < 1.0 - PLANE_TURN || apart.abs() < PLANE_GAP {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::GapNotParallel);
        }
        if gap <= PLANE_GAP {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::GapNotAboveZero);
        }
        let by = moves.shifts(apart, turn, gap);
        let shifts: Vec<Shift> = [Shift { face: a.desc, by: by.one }, Shift { face: b.desc, by: by.other }].into_iter().filter(|s| s.by.abs() > 1e-9).collect();
        let mut carried = self.blends_beside(p, src, a.desc);
        for c in self.blends_beside(p, src, b.desc) {
            if !carried.iter().any(|k| k.face == c.face) {
                carried.push(c);
            }
        }
        pushed_job(src, body, shifts, carried)
    }

    /// A HOLE MOVED: the plane of the face resolved, the axis of the hole - square into its entry face - found standing
    /// parallel to it, and the hole made again exact with its axis `gap` from the plane, on the side it stood.
    pub(crate) fn prep_holemoved(&mut self, p: &Pass, src: Id, refs: HoleRefs, exact: &Exact, gap: f64, body: Id) -> crate::feature::KernelJob {
        let gap = p.dim("gap", gap);
        let Exact::Hole { entry, at, diameter, depth, reach, ref rims } = *exact else {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::GapNotParallel);
        };
        let Ok(c) = self.face_by_ref(p.node, src, refs.face, "ref-what-gap-face") else {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::FaceNotFound);
        };
        let n = c.normal;
        let across = (0..3).map(|i| n[i] * entry.normal[i]).sum::<f64>();
        let s = (0..3).map(|i| n[i] * (at[i] - c.centroid[i])).sum::<f64>();
        // THE FACE AND THE NUMBER REFUSED APART: the faces picked are right when only the number is wrong, and words
        // asking for other faces sent the person to pick the same two again
        if across.abs() > AXIS_TURN {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::HoleAxisAcrossFace);
        }
        if gap <= PLANE_GAP {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::HoleGapNotAboveZero);
        }
        let want = if s < 0.0 { -gap } else { gap };
        let at = [at[0] + n[0] * (want - s), at[1] + n[1] * (want - s), at[2] + n[2] * (want - s)];
        self.prep_remade(p, src, &[RemadeItem { faces: refs.faces.clone(), exact: Exact::Hole { entry, at, diameter, depth, reach, rims: rims.clone() }, group: 0 }], body)
    }

    /// A boss or a pocket redrawn (`FeatureKind::Redrawn`): its faces taken off `src`, the body healed, and the largest
    /// closed contour of the sketch extruded `height` from the plane of its base towards the sketch - fused for a boss,
    /// cut for a pocket. Measured from the sketch, a taller boss only went deeper into the plate it stood on (its top
    /// is the sketch): 8 made 10 left the body as it was. The base is read where it stands now, so a base moved above
    /// carries the boss. The tool reaches `REDRAW_PAST` back past the base, into the plate or out of the opening: an end
    /// lying in the plane of a face of the body leaves a sliver of a face behind it.
    ///
    /// A STOCK - the body itself from one of its ends - has no base to heal over: its old prism is cut out by pushing its
    /// face in to the far end, what stood on it is left, and the sketch extruded from the far end is fused in; a body
    /// that was the stock and nothing else is the extruded sketch alone. Loops inside the face are holes of the sketch, so
    /// a hole through stays a hole and a boss on the face keeps the column it stands on.
    pub(crate) fn prep_redrawn(&mut self, p: &Pass, r: Redraw, height: f64) -> crate::feature::KernelJob {
        let Redraw { src, faces, base, sketch, kind, rims, body } = r;
        let ids = match self.faces_by_ref(p.node, src, faces, "ref-what-redrawn-faces") {
            Ok(ids) if !ids.is_empty() => ids,
            _ => return crate::feature::KernelJob::refused(crate::errors::CoreError::FacesNotFound),
        };
        let h = p.dim("height", height);
        if h <= PLANE_GAP {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::PrismHeightNotAboveZero);
        }
        let Some(outer) = self.outer_contour(sketch) else { return crate::feature::KernelJob::refused(crate::errors::CoreError::ProfileNotFound) };
        let Some(sourced) = self.encode_profiles_sourced(sketch, &[outer], &[]) else { return crate::feature::KernelJob::refused(crate::errors::CoreError::ProfileNotFound) };
        let pl = self.sketch_place(sketch);
        let Ok(b) = self.face_by_ref(p.node, src, base, "ref-what-redrawn-base") else { return crate::feature::KernelJob::refused(crate::errors::CoreError::FaceNotFound) };
        // where the base stands along the normal of the sketch, and which way from it the sketch lies
        let (n, o) = ([pl[2], pl[6], pl[10]], [pl[3], pl[7], pl[11]]);
        let off = (0..3).map(|i| (b.centroid[i] - o[i]) * n[i]).sum::<f64>();
        let toward = if off > 0.0 { -1.0 } else { 1.0 };
        let stock = (kind == crate::model::PrismKind::Stock).then(|| self.stock_of(src, &ids, Plane { n, o }));
        // a stock starts at its far end: nothing of the body lies past it
        let near = if stock.is_some() { off } else { off - toward * REDRAW_PAST };
        let far = off + toward * h;
        let span = crate::feature::ToolSpan { start: near.min(far), length: (far - near).abs(), ends: crate::feature::ToolEnds::Kept };
        let op = match kind {
            crate::model::PrismKind::Boss | crate::model::PrismKind::Stock => 1,
            crate::model::PrismKind::Pocket => 0,
        };
        // the rims as the tree has them, laid again where the new walls meet the new face (`far`) and the base (`off`), the
        // edges found by the outline of the sketch in plan: the new faces are numbered by the kernel only once they are made
        let face_rim = rims.at_face.map(|r| crate::model::RimBlend { size: p.dim("rim", r.size), ..r });
        let base_rim = rims.at_base.map(|r| crate::model::RimBlend { size: p.dim("base_rim", r.size), ..r });
        let outline: Vec<crate::geom::Point2> = self.contour_index(outer).map(|i| self.contours[i].points.clone()).unwrap_or_default();
        let laid: Vec<RimAt> = [face_rim.map(|rim| RimAt { rim, at: far }), base_rim.map(|rim| RimAt { rim, at: off })].into_iter().flatten().collect();
        let feature = p.node;
        crate::feature::KernelJob::new(vec![src], move |k| {
            let read = match stock {
                Some(Stock { alone: true, .. }) => 0,
                Some(Stock { face: Some(face), .. }) => {
                    // pushed in to the far end as it stands and no further - not by the height typed anew: a boss may
                    // stand on the far end
                    k.push_face(body, src, face, -off.abs())?;
                    body
                }
                Some(Stock { face: None, .. }) => return Err(crate::errors::CoreError::FaceNotFound),
                None => {
                    k.remove_faces(body, src, &ids)?;
                    body
                }
            };
            let mut built =
                k.combine_region_multi(BodyOp { src: read, op, body }, &sourced.profiles, span, pl, crate::feature::Sides { feature, sources: &sourced.sources, regions: &sourced.regions })?;
            for r in &laid {
                let edges = crate::model::edges_round_outline(k, body, &pl, r.at, &outline);
                if edges.is_empty() {
                    let op = if r.rim.cut == crate::model::RimCut::Round { crate::errors::Op::Fillet } else { crate::errors::Op::Chamfer };
                    return Err(crate::errors::CoreError::OpFailed(op));
                }
                built = blend_rim(k, body, r.rim, &edges, feature)?;
            }
            Ok(built)
        })
    }

    /// WHAT A STOCK REDRAWN READS of body `src` as it stands: its face among the faces `ids` it goes with - the one
    /// lying in the plane of the sketch, facing its way - and whether those faces and its far end are the whole body.
    fn stock_of(&self, src: Id, ids: &[u32], plane: Plane) -> Stock {
        let Plane { n, o } = plane;
        let faces = self.regen_faces.get(&src).cloned().unwrap_or_default();
        let face = faces
            .iter()
            .filter(|f| ids.contains(&f.id))
            .find(|f| {
                let along = (f.centroid.x - o[0]) * n[0] + (f.centroid.y - o[1]) * n[1] + (f.centroid.z - o[2]) * n[2];
                let facing = (0..3).map(|i| f.normal[i] * n[i]).sum::<f64>();
                along.abs() < 1e-4 && facing > 1.0 - 1e-6
            })
            .map(|f| f.id);
        // the faces taken off and the one far end: nothing else is there to keep
        Stock { face, alone: faces.len() == ids.len() + 1 }
    }

    /// THE LARGEST CLOSED CONTOUR OF A SKETCH, by the area it encloses: the outline a boss or a pocket is redrawn by,
    /// the contours inside it its holes.
    pub fn outer_contour(&self, sketch: Id) -> Option<Id> {
        let si = self.sketch_index(sketch)?;
        let area = |cid: Id| -> f64 {
            let Some(c) = self.contour_index(cid).map(|i| &self.contours[i]) else { return 0.0 };
            if !c.closed || c.points.len() < 3 {
                return 0.0;
            }
            let n = c.points.len();
            (0..n).map(|k| c.points[k].x * c.points[(k + 1) % n].y - c.points[(k + 1) % n].x * c.points[k].y).sum::<f64>().abs() * 0.5
        };
        self.sketches[si].contour_ids.iter().copied().filter(|c| area(*c) > 0.0).max_by(|a, b| area(*a).total_cmp(&area(*b)))
    }

    /// The roundings and chamfers made exact on the chain body `src` comes from that stand on face `pushed` and still
    /// have their face on `src`. A blend names its two faces as they were on the body it was found on; a later body names
    /// them anew, so they are found on `src` by their planes. The face of a blend is the one its node made that meets
    /// both of them: a node holds several blends.
    fn blends_beside(&mut self, p: &Pass, src: Id, pushed: u32) -> Vec<Carried> {
        use crate::feature::FeatureKind;
        let mut found: Vec<Made> = Vec::new();
        let mut cur = src;
        while let Some(n) = self.timeline.iter().find(|n| n.kind.body() == Some(cur) && !n.suppressed) {
            if let FeatureKind::Remade { src: made_on, items, .. } = &n.kind {
                found.extend(items.iter().map(|it| Made { node: n.id, made_on: *made_on, exact: it.exact.clone(), group: it.group }));
            }
            match n.kind.consumed_body() {
                Some(below) if below != cur => cur = below,
                _ => break,
            }
        }
        let now = self.regen_faces.get(&src).cloned().unwrap_or_default();
        let edges = self.regen_edges.get(&src).cloned().unwrap_or_default();
        let mut out = Vec::new();
        for Made { node, made_on, exact, group } in found {
            let dims = self.feat_dims.get(&node).cloned();
            let (sides, blend) = match exact {
                Exact::Round { sides, radius } => (sides, Blend::Round(eval_dim(dims.as_ref(), &group_key("radius", group), radius, p.vars))),
                Exact::Chamfer { sides, leg } => (sides, Blend::Chamfer(eval_dim(dims.as_ref(), &group_key("dist", group), leg, p.vars))),
                Exact::Hole { .. } => continue,
            };
            let then = self.regen_faces.get(&made_on).map(Vec::as_slice).unwrap_or_default();
            let side = |id: u32| then.iter().find(|f| f.id == id).and_then(|f| in_plane_of(f, &now));
            let (Some(one), Some(other)) = (side(sides.one), side(sides.other)) else { continue };
            // the face of the blend is named by the bridge after the node and the edge it was laid on: found by its node
            // and by the two faces it meets
            let meets = |face: u32, side: u32| edges.iter().any(|e| (e.faces.one == face && e.faces.other == side) || (e.faces.one == side && e.faces.other == face));
            let made_by = |f: &&crate::geom::MeshFace| self.names.get(f.id).is_some_and(|g| g.feature == node && g.role == crate::names::Role::Blend) && meets(f.id, one) && meets(f.id, other);
            if let (true, Some(face)) = (one == pushed || other == pushed, now.iter().find(made_by).map(|f| f.id)) {
                out.push(Carried { face, sides: crate::geom::EdgeFaces { one, other }, blend, feature: node });
            }
        }
        out
    }

    pub(crate) fn prep_surfacereplace(&mut self, p: &Pass, src: Id, faces: &crate::refs::Ref, surface: Id, body: Id) -> crate::feature::KernelJob {
        let faces = faces.clone();
        match self.faces_by_ref(p.node, src, &faces, "ref-what-surface-replace") {
            Ok(ids) if !ids.is_empty() => crate::feature::KernelJob::new(vec![src, surface], move |k| k.replace_faces(body, src, &ids, surface)),
            _ => crate::feature::KernelJob::refused(crate::errors::CoreError::FacesNotFound),
        }
    }

    pub(crate) fn prep_facecopy(&mut self, p: &Pass, src: Id, faces: &crate::refs::Ref, body: Id) -> crate::feature::KernelJob {
        let faces = faces.clone();
        match self.faces_by_ref(p.node, src, &faces, "ref-what-face-copy") {
            Ok(ids) if !ids.is_empty() => {
                let feature = p.node;
                crate::feature::KernelJob::new(vec![src], move |k| k.copy_faces(body, src, &ids, feature))
            }
            _ => crate::feature::KernelJob::refused(crate::errors::CoreError::FacesNotFound),
        }
    }

    /// OffsetSurface: the faces, resolved as a face copy resolves them, and the distance, a dimension of its own (`dist`)
    /// a formula or a global parameter drives.
    pub(crate) fn prep_offset_surface(&mut self, p: &Pass, src: Id, faces: &crate::refs::Ref, dist: f64, body: Id) -> crate::feature::KernelJob {
        let d = eval_dim(p.dims, "dist", dist, p.vars);
        match self.faces_by_ref(p.node, src, faces, "ref-what-offset-surface") {
            Ok(ids) if !ids.is_empty() => {
                let feature = p.node;
                crate::feature::KernelJob::new(vec![src], move |k| k.offset_faces(body, src, &ids, feature, d))
            }
            _ => crate::feature::KernelJob::refused(crate::errors::CoreError::FacesNotFound),
        }
    }

    /// Thicken: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_thicken(&mut self, p: &mut Pass, src: Id, face: u32, thickness: f64, join: Id, body: Id) -> crate::feature::KernelJob {
        let feature = p.node;
        // The thickness is parametric (the `thickness` feature dimension) and the face reference
        // resolves by name.
        let t = eval_dim(p.dims, "thickness", thickness, p.vars);
        // The face reference goes through the shared resolution rather than a comparison of
        // numbers.
        //
        // While a face was a positional number the reference depended on the numbering, and as soon
        // as the source gained names the stored number stopped matching, turning a thicken red with
        // "face not found" in a live project. The order now matches the one used for edges: name,
        // then a recorded merge, then the single face of a sheet, then a place snapshot.
        let face = self.resolve_face_id(p.node, src, face).unwrap_or(face);
        let alive = self.regen_faces.get(&src).is_some_and(|fs| fs.iter().any(|f| f.id == face));

        if face == 0 || !alive {
            crate::feature::KernelJob::refused(crate::errors::CoreError::FaceNotFound)
        } else if t.abs() < 1e-9 {
            crate::feature::KernelJob::refused(crate::errors::CoreError::ZeroThickness)
        } else {
            // Plate names come from the recipe: the offset side is produced by its face and each
            // side wall by its boundary edge, which is their entire recipe and does not depend on
            // numbering. The pairs are prepared in advance, since interning mutates the name table
            // while the kernel only needs the finished substitution.
            let inputs = if join != 0 { vec![src, join] } else { vec![src] };
            crate::feature::KernelJob::new(inputs, move |k| k.thicken_face(body, src, face, t, join, feature))
        }
    }

    /// SplitFace: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_splitface(&mut self, p: &mut Pass, src: Id, at: Option<([f64; 3], [f64; 3])>, offset: f64, body: Id) -> crate::feature::KernelJob {
        // The plane is a reference, as for a body split: a world plane, a datum or a face (see `op_plane`).
        let lost = at.is_none();
        let (o0, n) = at.unwrap_or(([0.0; 3], [0.0, 0.0, 1.0]));
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();

        if lost {
            crate::feature::KernelJob::refused(crate::errors::CoreError::SplitPlaneDeleted)
        } else if len < 1e-9 {
            crate::feature::KernelJob::refused(crate::errors::CoreError::ZeroNormal)
        } else {
            let d = p.dim("offset", offset);
            let u = [n[0] / len, n[1] / len, n[2] / len];
            let at = [o0[0] + u[0] * d, o0[1] + u[1] * d, o0[2] + u[2] * d];
            crate::feature::KernelJob::new(vec![src], move |k| k.split_faces(body, src, at, u))
        }
    }

    /// Draft: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_draft(&mut self, p: &mut Pass, src: Id, faces: &crate::refs::Ref, shape: DraftShape, body: Id) -> crate::feature::KernelJob {
        let DraftShape { neutral, angle, flip } = shape;
        // The angle is parametric (the `angle` feature dimension); the neutral face resolves into an
        // origin and a normal from the rebuilt source, and the pull direction is that normal,
        // reversed by `flip`.
        let angle = eval_dim(p.dims, "angle", angle, p.vars);
        // Both references are queries. The neutral face defines the pull direction, so taking the
        // wrong one drafts the part the other way and losing it is a refusal.
        //
        // A partial loss is a loss too. Measured: a draft given four live walls and one unresolvable
        // reference drafted four and stayed green. The part looks similar while one wall stands
        // vertical — and the mould is cast against it. As for a shell, only a hand-picked set is
        // checked this way: for a descriptive query the number of descriptors says nothing about the
        // result.
        let asked_faces = if faces.query.is_pick_list() { faces.query.picked_descs().len() } else { 0 };
        // a draft of 0 tilts nothing: it stood green with the body as it was
        if angle.abs() < 1e-9 {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::DraftAngleZero);
        }
        match (self.faces_by_ref(p.node, src, faces, "ref-what-draft-faces"), self.face_by_ref(p.node, src, &neutral, "ref-what-draft-neutral")) {
            (Ok(ids), _) if asked_faces > 0 && ids.len() < asked_faces => crate::feature::KernelJob::refused(crate::errors::CoreError::DraftNeedsFaces),
            (Ok(ids), Ok(np)) if !ids.is_empty() => {
                let np_o = np.centroid;
                let np_n = if flip { [-np.normal[0], -np.normal[1], -np.normal[2]] } else { np.normal };
                // the side faces a draft makes are named by the bridge, after the node
                let feature = p.node;
                crate::feature::KernelJob::new(vec![src], move |k| {
                    k.draft(body, src, &ids, crate::feature::DraftPull { angle, dir: np_n }, crate::feature::PlaneAt { origin: np_o, normal: np_n }, feature)
                })
            }
            _ => crate::feature::KernelJob::refused(crate::errors::CoreError::DraftNeedsFaces),
        }
    }

    /// Mirror: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_mirror(&mut self, p: &mut Pass, src: Id, world: Option<u8>, keep: bool, at: Option<([f64; 3], [f64; 3])>, body: Id) -> crate::feature::KernelJob {
        // A datum plane or a face supplies its origin and normal (see `op_plane`) and the mirror is taken about it;
        // otherwise a world plane 0, 1 or 2 is used.
        // An image is not the original face: with `keep` the body holds both halves, and without
        // names of their own they are indistinguishable — the same collision a pattern had.
        let keeps = if keep { crate::feature::MirrorKeeps::Both } else { crate::feature::MirrorKeeps::Image };
        let made = crate::feature::Mirrored { keeps, names: crate::feature::ImageNames::Instance { feature: p.node } };
        // A deleted plane means no mirror. Falling back to a world plane sounds defensible — a
        // mirror about another plane is still a mirror — but the measurement refutes it: the part
        // moves. A mirror about a datum at x = 50 gave face centres from x = 10 to x = 90, and
        // deleting the datum moved them to -30..30 with no red node. A split already refuses in the
        // same situation, for exactly the same reason.
        match (world, at) {
            (Some(plane), _) => crate::feature::KernelJob::new(vec![src], move |k| k.mirror(body, src, plane, made)),
            (None, Some((o, n))) => crate::feature::KernelJob::new(vec![src], move |k| k.mirror_plane(body, src, crate::feature::PlaneAt { origin: o, normal: n }, made)),
            (None, None) => crate::feature::KernelJob::refused(crate::errors::CoreError::MirrorPlaneDeleted),
        }
    }

    /// The body a datum plane or axis stands on - its face or its edge - by the datum's id; `None` for a datum on a
    /// world plane, another datum or coordinates.
    pub(crate) fn datum_base_body(&self, id: Id) -> Option<Id> {
        use crate::model::{AxisDef, PlaneDef};
        if let Some(p) = self.planes.iter().find(|p| p.id == id) {
            return match p.def {
                PlaneDef::OffsetFace { body, .. } => Some(body),
                _ => None,
            };
        }
        self.datum_axes.iter().find(|a| a.id == id).and_then(|a| match a.def {
            AxisDef::FromEdge { body, .. } | AxisDef::FromFace { body, .. } => Some(body),
            _ => None,
        })
    }

    /// THE PLANE A MIRROR OR A SPLIT STANDS ON, as origin and normal: a face of a body (read off it at this rebuild, so
    /// it follows the face), a datum plane (resolved earlier in the pass), or a world plane 0 XY, 1 XZ, 2 YZ. `None`
    /// when the face's body or the datum is gone: the operation refuses rather than cut along another plane.
    pub fn op_plane(&self, plane: u8, datum: Id, face: Option<(Id, crate::feature::FaceKey)>) -> Option<([f64; 3], [f64; 3])> {
        if let Some((body, key)) = face {
            return self.regen_faces.contains_key(&body).then(|| self.resolve_face(body, &key));
        }
        if datum != 0 {
            return self.planes.iter().find(|p| p.id == datum).map(|p| (p.origin, p.normal));
        }
        Some(match plane {
            1 => ([0.0; 3], [0.0, 1.0, 0.0]),
            2 => ([0.0; 3], [1.0, 0.0, 0.0]),
            _ => ([0.0; 3], [0.0, 0.0, 1.0]),
        })
    }

    /// Move: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_move(&mut self, p: &mut Pass, src: Id, mat: [f64; 12], body: Id) -> crate::feature::KernelJob {
        // THE SHIFT FOLLOWS ITS FORMULAS, typed at the gizmo or named: the translation of the transform, axis by axis
        let mut mat = mat;
        for k in 0..3 {
            let at = 3 + 4 * k;
            mat[at] = p.dim(["dx", "dy", "dz"][k], mat[at]);
        }
        // A BODY MADE OF A MESH MOVES IN THE DOCUMENT: it has no B-rep for the kernel to move - its mesh and the faces found
        // on it are moved as they are, and nothing is found on it again
        if self.mesh_born(src) {
            let Some(mi) = self.mesh_index(src) else { return crate::feature::KernelJob::refused(crate::errors::CoreError::SourceBodyNotBuilt) };
            let mut mesh = self.bodies[mi].mesh.clone();
            mesh.transform(&mat);
            let faces: Vec<crate::geom::MeshFace> = self.bodies[mi].faces.iter().map(|f| f.transformed(&mat)).collect();
            return crate::feature::KernelJob::new(Vec::new(), move |_| Ok(crate::geom::Built { mesh, faces }));
        }
        crate::feature::KernelJob::new(vec![src], move |k| k.transform_body(body, src, mat))
    }

    /// BodyBoolean: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_bodyboolean(&mut self, a: Id, b: Id, op: u8, body: Id) -> crate::feature::KernelJob {
        // Parametric body-to-body boolean: `op` applied to the B-reps of `a` and `b`, producing
        // `body`; both operands are consumed.
        crate::feature::KernelJob::new(vec![a, b], move |k| k.body_boolean(body, a, b, op))
    }

    /// Box3: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_box3(&mut self, p: &Pass, dx: f64, dy: f64, dz: f64, body: Id) -> crate::feature::KernelJob {
        let (dx, dy, dz) = (p.dim("dx", dx), p.dim("dy", dy), p.dim("dz", dz));
        if !above_zero(&[dx, dy, dz]) {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::PrimitiveSizeNotAboveZero);
        }
        self.named_prism_job(p.node, &rect_profile(dx, dy), dz, body)
    }

    /// A PRIMITIVE EXTRUDED FROM A POLYGON, every face named by recipe: side k of the polygon (from 1) makes the wall
    /// `Role::Wall` of number k, the ends are the caps of the node. The number of a side is its place in the recipe of
    /// the primitive - the side from the first corner of a box, the side from the first vertex of a prism - and not
    /// the place the face takes in the walk of the result: measured, a box and a prism laid with an unnamed profile
    /// had 6 of 6 and 8 of 8 faces on positional numbers, and a fillet of an edge of a box held a number.
    pub(crate) fn named_prism_job(&mut self, feature: Id, xy: &[f64], height: f64, body: Id) -> crate::feature::KernelJob {
        let points: Vec<crate::geom::Point2> = xy.as_chunks::<2>().0.iter().map(|[x, y]| crate::geom::Point2::new(*x, *y)).collect();
        let sides = points.len() as Id;
        let mut outline = crate::geom::Contour::closed(points);
        outline.edge_src = (1..=sides).collect();
        // side k is the source at place k: its slot holds k itself
        let sources: Vec<crate::feature::SideSource> = (1..=sides).map(|k| crate::feature::SideSource { src: k, split: 0, cut: 0 }).collect();
        let profiles = vec![crate::geom::encode_profile_named(&outline, &[], &|src: Id, _: &crate::geom::ProfEdge| src as u32)];
        crate::feature::KernelJob::new(Vec::new(), move |k| {
            k.combine_region_multi(
                BodyOp { src: 0, op: 1, body },
                &profiles,
                crate::feature::ToolSpan { start: 0.0, length: height, ends: crate::feature::ToolEnds::Kept },
                crate::feature::PLACE_IDENTITY,
                crate::feature::Sides { feature, sources: &sources, regions: &[] },
            )
        })
    }

    /// Cylinder: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    /// THE PARCEL OF A CYLINDER, and of every primitive below it: numbers and names, nothing borrowed from
    /// the document.
    ///
    /// Preparing one takes `&mut self` and always will: minting the names of the faces to come is a change to
    /// the document, and it has to happen before the geometry is built. That is why the split is where it is -
    /// the preparation stays on the thread that owns the document, and only the parcel travels.
    pub(crate) fn prep_cylinder(&mut self, p: &Pass, r: f64, h: f64, body: Id) -> crate::feature::KernelJob {
        let (r, h) = (p.dim("r", r), p.dim("h", h));
        if !above_zero(&[r, h]) {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::PrimitiveSizeNotAboveZero);
        }
        let feature = p.node;
        crate::feature::KernelJob::new(Vec::new(), move |k| k.cylinder(body, r, h, feature))
        // Exact B-rep cylinder, three faces.
    }

    /// Sphere: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_sphere(&mut self, p: &Pass, r: f64, body: Id) -> crate::feature::KernelJob {
        let r = eval_dim(p.dims, "r", r, p.vars);
        if !above_zero(&[r]) {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::PrimitiveSizeNotAboveZero);
        }
        let feature = p.node;
        crate::feature::KernelJob::new(Vec::new(), move |k| k.sphere(body, r, feature))
        // Exact sphere, one face.
    }

    /// Cone: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_cone(&mut self, p: &Pass, r1: f64, r2: f64, h: f64, body: Id) -> crate::feature::KernelJob {
        let (r1, r2, h) = (p.dim("r1", r1), p.dim("r2", r2), p.dim("h", h));
        // one end may close to a point, not both, and neither end is narrower than nothing
        if !above_zero(&[h, r1.max(r2)]) || r1.min(r2) < 0.0 {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::PrimitiveSizeNotAboveZero);
        }
        let feature = p.node;
        crate::feature::KernelJob::new(Vec::new(), move |k| k.cone(body, r1, r2, h, feature))
        // Exact cone.
    }

    /// Torus: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_torus(&mut self, p: &Pass, major: f64, minor: f64, body: Id) -> crate::feature::KernelJob {
        let (major, minor) = (p.dim("major", major), p.dim("minor", minor));
        if !above_zero(&[major, minor]) {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::PrimitiveSizeNotAboveZero);
        }
        // a tube as thick as the ring or thicker passes through itself: a ring of 5 and a tube of 5 stood green as a
        // spindle of 2467 mm^3
        if minor >= major {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::TorusThroughItself);
        }
        let feature = p.node;
        crate::feature::KernelJob::new(Vec::new(), move |k| k.torus(body, major, minor, feature))
        // Exact torus, one face.
    }

    /// Prism: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_prism(&mut self, p: &Pass, r: f64, n: u32, h: f64, body: Id) -> crate::feature::KernelJob {
        let (r, h) = (p.dim("r", r), p.dim("h", h));
        if !above_zero(&[r, h]) {
            return crate::feature::KernelJob::refused(crate::errors::CoreError::PrimitiveSizeNotAboveZero);
        }
        // the number of sides is a dimension of the node as the radius is: sides typed as `k` follow `k`
        let n = p.dim("n", f64::from(n)).round().max(3.0) as u32;
        self.named_prism_job(p.node, &polygon_profile(r, n), h, body)
    }

    /// Loft: one branch of the timeline rebuild. Returns whether the node's error record may
    /// be cleared (see `apply_regen`).
    pub(crate) fn prep_loft(&mut self, p: &mut Pass, sketches: Vec<Id>, contours: Vec<Id>, ruled: bool, surface: bool, bo: BodyOp) -> crate::feature::KernelJob {
        let BodyOp { src, op, body } = bo;
        // Each section sits on its own plane (the placement is encoded in `places`), and two or more
        // sections produce a body. A zero `src` makes a separate body; otherwise the lofted solid is
        // combined with body `src` as a lofted cut or boss.
        let feature = p.node;
        match self.loft_encoded_sourced(&sketches, &contours) {
            // A surface is the same loft, not closed into a solid. It admits no boolean: there is
            // nothing to combine a surface with a body by until it has been given a thickness.
            Some(enc) if src == 0 => crate::feature::KernelJob::new(Vec::new(), move |k| {
                let body_kind = if surface { crate::feature::LoftBody::Sheet } else { crate::feature::LoftBody::Solid };
                let s = &enc.sections;
                k.loft(
                    body,
                    crate::feature::LoftSections { data: &s.data, offsets: &s.offsets, places: &s.places },
                    walls(ruled),
                    body_kind,
                    crate::feature::Sides { feature, sources: &enc.sources, regions: &[] },
                )
            }),
            Some(enc) => crate::feature::KernelJob::new(vec![src], move |k| {
                let s = &enc.sections;
                k.loft_combine(BodyOp { src, op, body }, &s.data, &s.offsets, &s.places, walls(ruled), crate::feature::Sides { feature, sources: &enc.sources, regions: &[] })
            }),
            None => crate::feature::KernelJob::refused(crate::errors::CoreError::LoftNeedsTwoSections),
        }
    }
}

/// The plane of a sketch: its normal and its origin.
#[derive(Clone, Copy)]
struct Plane {
    n: [f64; 3],
    o: [f64; 3],
}

/// A STOCK AS ITS REDRAWING READS IT: the face its old prism is pushed out by, and whether the stock is the whole body.
#[derive(Clone, Copy)]
struct Stock {
    face: Option<u32>,
    alone: bool,
}

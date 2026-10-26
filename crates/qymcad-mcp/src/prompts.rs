//! THE PROMPTS: ways of working a client offers its person by name - design a part for a printer, change a mesh file,
//! change an exact one, find features by eye, get a body ready for a printer - each the order of tools this server has for it, with the person's own words filled in.

use serde_json::{json, Map, Value};

/// Whether an argument must be given.
#[derive(Clone, Copy, PartialEq)]
enum Need {
    Required,
    Optional,
}

/// An argument of a prompt.
struct Arg {
    name: &'static str,
    description: &'static str,
    need: Need,
}

/// A prompt: its name, what it is for, what it takes, and its text from the arguments given.
struct Prompt {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    arguments: &'static [Arg],
    text: fn(&Map<String, Value>) -> String,
}

/// An argument's text, or `or` when it is not given.
fn arg<'a>(given: &'a Map<String, Value>, name: &str, or: &'a str) -> &'a str {
    given.get(name).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).unwrap_or(or)
}

fn design_for_fdm(given: &Map<String, Value>) -> String {
    let part = arg(given, "part", "the part");
    let nozzle: f64 = arg(given, "nozzle", "0.4").parse().unwrap_or(0.4);
    let wall = nozzle * 3.0;
    format!(
        "Design {part} to be printed on an FDM printer with a {nozzle} mm nozzle, in QymCAD through its tools.\n\n\
         1. Name the main sizes as parameters (set_parameter) and give the tools expressions over them, so one change rebuilds the whole part.\n\
         2. Lay the body: a primitive (box, cylinder, prism) or a sketch (create_sketch, sketch_add) extruded. Put the face that lies on the bed on the XY plane.\n\
         3. Holes with the hole tool, at the points of a sketch or on a face. A hole for a screw is 0.2-0.4 mm wider than the screw (3.4 for M3); a counterbore or countersink by kind. Read the warnings: a hole that meets no material is named there.\n\
         4. Walls at least {wall:.1} mm (three lines of the nozzle). Round vertical edges freely (fillet); an edge facing down past 45 degrees prints badly rounded - chamfer it instead.\n\
         5. Steps that belong together go as one batch (apply_ops), one undo step.\n\
         6. Check before printing: inspect (valid, one solid), measure the sizes that matter, render from iso and from the top and look at the picture.\n\
         7. Save the document (save_project) and write the part out for the slicer: export_mesh to a .3mf file, quality standard.\n\n\
         Tell the person the sizes you chose and why, and what to change for a different fit."
    )
}

fn edit_stl(given: &Map<String, Value>) -> String {
    let path = arg(given, "path", "the STL file");
    let change = arg(given, "change", "the change the person asks for");
    format!(
        "Change the mesh file {path}: {change}.\n\n\
         1. Bring it in with import_mesh. Read the size it came in at and its warning: a mesh written in inches or metres comes in at the wrong size - import it again with the factor that makes it right.\n\
         2. Look at it: render from iso and the sides; measure what the change depends on.\n\
         3. A mesh is triangles, not a body: make it one with recognise_mesh - mode exact finds its planes, cylinders and cones; as_is, a polyhedron of flat faces, for a mesh exact surfaces do not hold. Then its holes, roundings and chamfers become features with recognise_all (then list first) or recognise_feature, their sizes editable; mesh_section and sketch_from_section draw a section of it to build on.\n\
         4. What is added is built as bodies of their own - primitives, sketches extruded - and placed by measuring the body.\n\
         5. Check with render, then write it out with export_mesh (.stl or .3mf) and save the document."
    )
}

fn edit_step(given: &Map<String, Value>) -> String {
    let path = arg(given, "path", "the STEP file");
    let change = arg(given, "change", "the change the person asks for");
    format!(
        "Change the exact solid file {path}: {change}.\n\n\
         1. Bring it in with import_cad. Its bodies are exact: they take fillets, holes, shells and cuts like modelled ones.\n\
         2. Find the faces and edges the change is on: list_faces and list_edges give their keys, kinds and places; resolve shows what a query finds before a feature is laid on it.\n\
         3. Make the change with the tools (hole, fillet, chamfer, shell, push_face, a sketch on a face extruded or cut), steps that belong together in one apply_ops batch.\n\
         4. Check: inspect (valid, one solid), measure, render.\n\
         5. Write it out with export_cad (.step keeps the tree of parts) or export_mesh for a printer, and save the document."
    )
}

fn edit_what_i_selected(given: &Map<String, Value>) -> String {
    let change = arg(given, "change", "the change the person asks for");
    format!(
        "In the open QymCAD window, on what the person has selected: {change}.\n\n\
         1. Read what is selected with get_selection. Nothing selected: ask the person to click what they mean, and stop. Refused with no-window: pass its hint on - it names the switch the way the person's window shows it - and stop. Name menus and settings to the person in the language get_selection gives as language.\n\
         2. Say back what you see, in their words: \"the rounding of radius 2 on the top edges\", \"the flat top face of the plate\".\n\
         3. A change of size goes to the feature that made the thing (made_by): edit_feature on its key. When the size follows an expression, write the change into that feature's expression (\"r - 0.2\"), not into the parameter - other features may read the parameter. When the feature makes several faces or edges at once (faces_made more than one), tell the person they all change together.\n\
         4. Something new on what is selected takes the item's use: create_sketch on a face or a plane, hole on a face, fillet or chamfer on an edge.\n\
         5. Each call is one step the person can take back with Ctrl+Z; say which step it is. If the window is busy, the person is working in it - ask again after they finish."
    )
}

fn recognise_by_eye(given: &Map<String, Value>) -> String {
    let path = arg(given, "path", "the mesh in the document");
    format!(
        "Find the holes, roundings and chamfers of {path} by looking at it, and make them features whose sizes can be changed.\n\n\
         1. A file not yet in the document comes in with import_mesh (an STL, OBJ, 3MF) or import_cad (a STEP). A mesh is made a body with recognise_mesh (mode exact; as_is when exact surfaces do not hold).\n\
         2. Try recognise_all with then list: it names every group it finds. Look by eye for what it missed or misread.\n\
         3. Look: render from iso and the sides. Where you see a hole, a rounding or a chamfer, take a pixel on it - the wall of a hole, the strip of a rounding, the slanted face of a chamfer - and ask point_at with the same view, width and height. It answers the face and the point there; the ground is refused.\n\
         4. Before laying anything, read it with fit_at_point at that point: the kind, the size measured and the size it would be laid at, a hole's depth and whether it runs through. To check where your points stand, render again with marks - the points as numbered rings.\n\
         5. The picture says where; the size comes from the geometry, never from the picture. If fit_at_point refuses, take another pixel on the feature, or raise tolerance for a rough mesh.\n\
         6. Make it a feature with recognise_feature at the same point - one node, one undo step; give size to lay it at another size. Then edit_feature changes it.\n\
         7. Tell the person what you found, with sizes, and what you made editable."
    )
}

fn prepare_for_print(given: &Map<String, Value>) -> String {
    let path = arg(given, "path", "the body in the document");
    let printer = arg(given, "printer", "the person's printer");
    let nozzle = arg(given, "nozzle", "0.4");
    format!(
        "Get {path} ready to print on {printer} with a {nozzle} mm nozzle, and write it out for the slicer.\n\n\
         1. A file not yet in the document comes in with import_mesh (an STL, OBJ, 3MF) or import_cad (a STEP). Read the size it came in at: a mesh written in inches or metres is put right with scale (factor 25.4 or 1000).\n\
         2. A mesh: read it with mesh_report - holes, edges shared by more than two triangles, triangles wound inward, pieces, its weight in triangles. Mend what it finds with mesh_repair (fill_holes, drop_under for crumbs, simplify for a mesh too heavy), then read mesh_report again: closed, one piece.\n\
         3. Ask check_printability with printer {printer} and nozzle {nozzle}. Each check has its verdict: the room of the printer, the contact with the bed, the overhangs past the angle, walls thinner than two lines of the nozzle, details too small for it. A warning names the places; tell the person what they mean.\n\
         4. Overhangs or a small contact: suggest_orientation gives the best ways to lay the body, each with the arguments for place_on_face. Lay it with the first, unless the person wants a face of their own on the bed, then ask check_printability again.\n\
         5. Too big for the room even turned: split_for_bed with the printer cuts it into pieces that fit, with joints across every cut (a plug, or a dowel printed apart) - the joints' sizes are features, edited with edit_feature. Lay each piece and check it as above.\n\
         6. Write it out with export_mesh to a .3mf file, quality standard: the pieces of a split and the pins go in one file. Save the document (save_project).\n\
         7. Tell the person the verdicts, what was mended, turned or cut, and where the .3mf is. Sending it to the printer is for a printer server the person has connected, if any; the print is started by the person."
    )
}

const PROMPTS: [Prompt; 6] = [
    Prompt {
        name: "design_for_fdm",
        title: "Design a part for FDM printing",
        description: "Model a part to be printed: parameters, body, holes with clearance, walls and edges a printer can make, checks, a 3MF for the slicer.",
        arguments: &[
            Arg { name: "part", description: "What to make, in the person's words", need: Need::Required },
            Arg { name: "nozzle", description: "Nozzle diameter in mm (0.4 by default)", need: Need::Optional },
        ],
        text: design_for_fdm,
    },
    Prompt {
        name: "edit_stl",
        title: "Change a mesh file",
        description: "Bring in an STL (or another mesh), make it a body, change it, and write it out again.",
        arguments: &[Arg { name: "path", description: "The mesh file", need: Need::Required }, Arg { name: "change", description: "What to change", need: Need::Required }],
        text: edit_stl,
    },
    Prompt {
        name: "edit_step",
        title: "Change a STEP file",
        description: "Bring in an exact solid (STEP, IGES), find the faces and edges to change, change them with features, and write it out again.",
        arguments: &[Arg { name: "path", description: "The STEP or IGES file", need: Need::Required }, Arg { name: "change", description: "What to change", need: Need::Required }],
        text: edit_step,
    },
    Prompt {
        name: "edit_what_i_selected",
        title: "Change what is selected in the window",
        description: "In the open QymCAD window: read what the person selected, say it back, and change the feature that made it - one step of undo.",
        arguments: &[Arg { name: "change", description: "What to change, in the person's words", need: Need::Required }],
        text: edit_what_i_selected,
    },
    Prompt {
        name: "recognise_by_eye",
        title: "Find features by looking at the model",
        description: "Look at a mesh or a STEP in pictures, find its holes, roundings and chamfers by eye, read each by its geometry, and make them features with sizes that can be changed.",
        arguments: &[Arg { name: "path", description: "The file, when it is not in the document yet", need: Need::Optional }],
        text: recognise_by_eye,
    },
    Prompt {
        name: "prepare_for_print",
        title: "Get a body ready for a printer",
        description: "Read and mend a mesh, check that a body prints on a printer, lay it the best way, cut it to fit the printer with joints, and write a 3MF for the slicer.",
        arguments: &[
            Arg { name: "path", description: "The file, when it is not in the document yet", need: Need::Optional },
            Arg { name: "printer", description: "The printer by its room, as check_printability names them", need: Need::Optional },
            Arg { name: "nozzle", description: "Nozzle diameter in mm (0.4 by default)", need: Need::Optional },
        ],
        text: prepare_for_print,
    },
];

/// The prompts a client may offer.
pub fn listing() -> Value {
    Value::Array(
        PROMPTS
            .iter()
            .map(|p| {
                let arguments: Vec<Value> = p.arguments.iter().map(|a| json!({ "name": a.name, "description": a.description, "required": a.need == Need::Required })).collect();
                json!({ "name": p.name, "title": p.title, "description": p.description, "arguments": arguments })
            })
            .collect(),
    )
}

/// THE PROMPT `name` with `given` filled in, as one message from the person; or what is wrong with the request.
pub fn get(name: &str, given: &Map<String, Value>) -> Result<Value, String> {
    let p = PROMPTS.iter().find(|p| p.name == name).ok_or_else(|| format!("no prompt {name}"))?;
    let missing: Vec<&str> = p.arguments.iter().filter(|a| a.need == Need::Required && arg(given, a.name, "").is_empty()).map(|a| a.name).collect();
    if !missing.is_empty() {
        return Err(format!("prompt {name} needs {missing:?}"));
    }
    Ok(json!({ "description": p.description, "messages": [{ "role": "user", "content": { "type": "text", "text": (p.text)(given) } }] }))
}

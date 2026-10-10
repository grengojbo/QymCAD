//! THE PROMPTS: ways of working a client offers its person by name - design a part for a printer, change a mesh file,
//! change an exact one - each the order of tools this server has for it, with the person's own words filled in.

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
         3. A mesh has no exact faces: it can be moved, measured and written out, not filleted or drilled as it is. What is added is built as bodies of their own beside it - primitives, sketches extruded - and placed by measuring the mesh.\n\
         4. When the change needs exact work on the mesh itself, say so to the person: the way is to model the shape anew from its measurements.\n\
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

const PROMPTS: [Prompt; 4] = [
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
        description: "Bring in an STL (or another mesh), change it as far as a mesh allows, and write it out again.",
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

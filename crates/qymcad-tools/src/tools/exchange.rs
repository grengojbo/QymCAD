//! FILES IN AND OUT: a mesh (STL, OBJ, PLY, glTF, 3MF, AMF) or an exact solid (STEP, IGES) brought into the document as
//! one step, and the document - or one part of it - written out as either.
//!
//! A mesh file in STL, OBJ or PLY carries no unit, so its numbers are millimetres only by guess; the answer gives the
//! size the file has it at and the size it came in at, and says when that size is past what a printed part measures.

use qymcad_doc::export::{ExportTarget, MeshDetail};
use qymcad_doc::import::{self, MeshFormat};
use qymcad_io::{FileUnit, Format};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::paths::{self, Existing};
use crate::tool::{self, Answer, Ctx, Tool};
use crate::tools::doc::{answer, outcome};

/// The unit the numbers of a file are in.
#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Unit {
    Micron,
    Mm,
    Cm,
    M,
    Inch,
    Foot,
}

impl Unit {
    fn file_unit(self) -> FileUnit {
        match self {
            Unit::Micron => FileUnit::Micron,
            Unit::Mm => FileUnit::Millimetre,
            Unit::Cm => FileUnit::Centimetre,
            Unit::M => FileUnit::Metre,
            Unit::Inch => FileUnit::Inch,
            Unit::Foot => FileUnit::Foot,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportArgs {
    path: String,
    /// The unit the file's numbers are in; left out, they are taken as they are.
    unit: Option<Unit>,
    /// A factor on top of the unit.
    scale: Option<f64>,
}

/// How fine a mesh file is written: a named detail or a deflection in mm.
#[derive(Deserialize, Clone, Copy)]
#[serde(untagged)]
enum Quality {
    Named(Detail),
    Deflection { deflection: f64 },
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Detail {
    Draft,
    Standard,
    High,
    Max,
}

impl Quality {
    fn deflection(self) -> f64 {
        match self {
            Quality::Named(Detail::Draft) => MeshDetail::Draft.deflection(),
            Quality::Named(Detail::Standard) => MeshDetail::Standard.deflection(),
            Quality::Named(Detail::High) => MeshDetail::High.deflection(),
            Quality::Named(Detail::Max) => MeshDetail::Max.deflection(),
            Quality::Deflection { deflection } => deflection,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportArgs {
    path: String,
    /// One part or assembly by its key; the whole document when left out.
    part: Option<u64>,
    quality: Option<Quality>,
    #[serde(default)]
    overwrite: bool,
}

fn extensions(formats: &[Format]) -> Vec<&'static str> {
    formats.iter().flat_map(|f| f.extensions().iter().copied()).collect()
}

const MESH_FORMATS: [Format; 6] = [Format::Stl, Format::Obj, Format::Ply, Format::Gltf, Format::ThreeMf, Format::Amf];
const EXACT_FORMATS: [Format; 2] = [Format::Step, Format::Iges];

/// Bring `path` in at the factor its unit and scale make; one step either way.
fn import(ctx: &mut Ctx, a: ImportArgs, formats: &[Format]) -> Result<Answer, tool::Refusal> {
    let path = paths::resolve(&a.path);
    paths::has_extension(&path, &extensions(formats))?;
    let factor = a.unit.map_or(1.0, |u| u.file_unit().mm()) * a.scale.unwrap_or(1.0);
    if !(factor.is_finite() && factor > 0.0) {
        return Err(tool::Refusal::new("bad-scale", &format!("The scale comes to {factor}; it has to be a number above zero."), tool::Stage::Validate));
    }
    let path = path.to_string_lossy().into_owned();
    let got = ctx.doc.import(&path, factor, &qymcad_i18n::name).map_err(tool::doc_refusal)?;
    let size: Vec<f64> = got.span.iter().map(|s| s * factor).collect();
    let mut a = answer("imported", json!({ "path": path, "root": got.root, "bodies": got.bodies, "unitless": got.unitless, "file_size": got.span, "size_mm": size, "factor": factor }));
    if import::out_of_size([size[0], size[1], size[2]]) {
        let note = format!(
            "The part came in {:.3} mm at its largest side - outside {}..{} mm, what a printed part usually measures. The file's unit may not be millimetres: undo and import again with unit or scale.",
            size.iter().copied().fold(0.0, f64::max),
            import::SMALLEST_MM,
            import::LARGEST_MM
        );
        a.insert("warning".into(), json!({ "code": "scale-suspicious", "message": note }));
    }
    Ok(outcome(ctx, a))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportCadArgs {
    path: String,
    part: Option<u64>,
    #[serde(default)]
    overwrite: bool,
}

fn target(part: Option<u64>) -> ExportTarget {
    part.map_or(ExportTarget::Project, ExportTarget::Component)
}

fn written(path: &str, out: &qymcad_doc::Exported) -> Answer {
    let p = &out.plan;
    answer("written", json!({ "path": path, "bodies": out.written.bodies, "failed": out.written.failed, "exact": p.brep.len(), "mesh_only": p.mesh_only.len(), "stale": p.stale.len() }))
}

const IMPORT_SCHEMA: fn() -> Value = || {
    json!({ "type": "object", "properties": {
        "path": { "type": "string" },
        "unit": { "type": "string", "enum": ["micron", "mm", "cm", "m", "inch", "foot"], "description": "The unit the file's numbers are in. STL, OBJ and PLY carry none." },
        "scale": { "type": "number", "exclusiveMinimum": 0, "description": "A factor on top of the unit." },
    }, "required": ["path"], "additionalProperties": false })
};

pub const IMPORT_MESH: Tool = Tool {
    name: "import_mesh",
    description: "Bring a mesh file (STL, OBJ, PLY, glTF/GLB, 3MF, AMF) into the document as one step, as its tree of parts. A mesh has no exact faces: it can be moved, measured and written out, not filleted. Answers with the size it came in at and a warning when that size looks wrong for its unit.",
    schema: IMPORT_SCHEMA,
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: ImportArgs = tool::args(arguments)?;
        import(ctx, a, &MESH_FORMATS)
    },
};

pub const IMPORT_CAD: Tool = Tool {
    name: "import_cad",
    description: "Bring an exact solid file (STEP, IGES) into the document as one step, as its tree of parts. The bodies are exact: they can be filleted, cut and measured like modelled ones.",
    schema: IMPORT_SCHEMA,
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: ImportArgs = tool::args(arguments)?;
        import(ctx, a, &EXACT_FORMATS)
    },
};

pub const EXPORT_MESH: Tool = Tool {
    name: "export_mesh",
    description: "Write the visible bodies of the document, or of one part, as a mesh file - the format by the extension: .stl, .obj, .ply, .glb, .3mf, .amf. 3MF and glTF keep the parts and their colours. A path without a folder is written beside the document's own file, or into the person's Downloads while the document has none; the answer names the absolute path - tell it to the person. quality is draft (0.2 mm), standard (0.05), high (0.02), max (0.005) or {\"deflection\": mm}.",
    schema: || {
        json!({ "type": "object", "properties": {
            "path": { "type": "string" },
            "part": { "type": "integer", "description": "A part or assembly key from get_document; the whole document when left out." },
            "quality": { "oneOf": [
                { "type": "string", "enum": ["draft", "standard", "high", "max"] },
                { "type": "object", "properties": { "deflection": { "type": "number", "exclusiveMinimum": 0 } }, "required": ["deflection"], "additionalProperties": false },
            ], "default": "standard" },
            "overwrite": { "type": "boolean", "default": false },
        }, "required": ["path"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: ExportArgs = tool::args(arguments)?;
        let path = paths::to_write(&a.path, &paths::Places::here(ctx.path.as_deref()))?;
        paths::has_extension(&path, &extensions(&MESH_FORMATS))?;
        paths::may_write(&path, Existing::from_overwrite(a.overwrite))?;
        paths::may_write_in(&path)?;
        let deflection = a.quality.unwrap_or(Quality::Named(Detail::Standard)).deflection();
        if !(deflection.is_finite() && deflection > 0.0) {
            return Err(tool::Refusal::new("bad-deflection", &format!("The deflection {deflection} has to be a number of millimetres above zero."), tool::Stage::Validate));
        }
        let target_path = path.to_string_lossy().into_owned();
        let format = MeshFormat::of_path(&target_path).expect("the extension was checked against the mesh formats");
        let out = ctx.doc.export_mesh(&target_path, format, target(a.part), deflection, &qymcad_i18n::name).map_err(tool::doc_refusal)?;
        Ok(written(&target_path, &out))
    },
};

pub const EXPORT_CAD: Tool = Tool {
    name: "export_cad",
    description: "Write the visible exact bodies of the document, or of one part, as STEP (.step, .stp - with the tree of parts) or IGES (.igs, .iges). A body that is only a mesh cannot go into an exact file; the answer counts what went and what did not. A path without a folder is written beside the document's own file, or into the person's Downloads while the document has none; the answer names the absolute path - tell it to the person.",
    schema: || {
        json!({ "type": "object", "properties": {
            "path": { "type": "string" },
            "part": { "type": "integer", "description": "A part or assembly key from get_document; the whole document when left out." },
            "overwrite": { "type": "boolean", "default": false },
        }, "required": ["path"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: ExportCadArgs = tool::args(arguments)?;
        let path = paths::to_write(&a.path, &paths::Places::here(ctx.path.as_deref()))?;
        paths::has_extension(&path, &extensions(&EXACT_FORMATS))?;
        paths::may_write(&path, Existing::from_overwrite(a.overwrite))?;
        paths::may_write_in(&path)?;
        let target_path = path.to_string_lossy().into_owned();
        let format = match Format::of_path(&target_path) {
            Some(Format::Iges) => qymcad_kernel::ExactFormat::Iges,
            _ => qymcad_kernel::ExactFormat::Step,
        };
        let out = ctx.doc.export_exact(&target_path, format, target(a.part), &qymcad_i18n::name).map_err(tool::doc_refusal)?;
        Ok(written(&target_path, &out))
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScaleArgs {
    body: Option<crate::args::BodyRef>,
    factor: f64,
}

pub const SCALE: Tool = Tool {
    name: "scale",
    description: "Take a body brought in from a file - by import_mesh or import_cad - at factor times the size it stands at, as the window's question about the scale does: every body of that file with it, the parts the file placed about the file's zero. 25.4 takes a file drawn in inches to mm, 0.1 a file read as mm that was cm. One step undo takes back. The answer gives the factor it then stands at from the file's own numbers and its size. A body built in the document is refused: change the sketches and features its size is in, or a parameter they follow.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": crate::tools::features::body_field(),
            "factor": { "type": "number", "exclusiveMinimum": 0, "maximum": 100000, "description": "Times the size it stands at." },
        }, "required": ["factor"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: ScaleArgs = tool::args(arguments)?;
        if !(a.factor.is_finite() && a.factor > 0.0) {
            return Err(tool::Refusal::new("bad-factor", &format!("A factor of {} takes nothing: it is a number over 0.", a.factor), tool::Stage::Validate));
        }
        let body = crate::tools::features::target(ctx.doc.project(), a.body)?;
        if ctx.doc.project().import_scale(body).is_none() {
            return Err(tool::Refusal::new("scale-built-body", &format!("Body {body} was built in the document, not brought in from a file: it has no factor to take it at."), tool::Stage::Validate)
                .with_hint("Change the sketches and features its size is in, or a parameter they follow (set_parameter)."));
        }
        let to = ctx.doc.rescale(body, a.factor).map_err(tool::doc_refusal)?;
        let mut out = Answer::new();
        out.insert("body".into(), json!(body));
        out.insert("factor".into(), json!(to));
        out.insert("size".into(), json!(import::world_span(ctx.doc.project(), &[body]).map(crate::tools::query::round)));
        Ok(out)
    },
};

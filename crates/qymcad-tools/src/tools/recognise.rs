//! EVERY FEATURE OF A BODY MADE EXACT AT ONCE: the holes, roundings and chamfers found on a body, in groups of one size,
//! listed or laid as one node - the window's "Recognise all".

use qymcad_core::model::{group_key, Exact, Reach, RemakeGroup};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::args::{amount_schema, Amount, BodyRef};
use crate::tool::{self, Ctx, Refusal, Stage, Tool};

/// A kind of feature, as the answer and the arguments name it.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Hole,
    Rounding,
    Chamfer,
}

fn kind_of(e: &Exact) -> Kind {
    match e {
        Exact::Hole { .. } => Kind::Hole,
        Exact::Round { .. } => Kind::Rounding,
        Exact::Chamfer { .. } => Kind::Chamfer,
    }
}

/// What the call does with what it found: lists it, or lays it.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Then {
    List,
    #[default]
    Lay,
}

/// A new size for a group, by its number in the list.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupSize {
    group: usize,
    size: Option<Amount>,
    depth: Option<Amount>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecogniseAllArgs {
    body: Option<BodyRef>,
    kinds: Option<Vec<Kind>>,
    #[serde(default)]
    skip: Vec<usize>,
    #[serde(default)]
    sizes: Vec<GroupSize>,
    #[serde(default)]
    then: Then,
}

/// A group of the list as the answer tells it.
fn told(g: usize, group: &RemakeGroup) -> Value {
    let Some(first) = group.items.first() else { return json!({ "group": g }) };
    let kind = match kind_of(&first.exact) {
        Kind::Hole => "hole",
        Kind::Rounding => "rounding",
        Kind::Chamfer => "chamfer",
    };
    let mut v = json!({ "group": g, "kind": kind, "count": group.items.len() });
    match first.exact {
        Exact::Hole { diameter, depth, reach, .. } => {
            v["size"] = json!(diameter);
            v["through"] = json!(reach == Reach::Through);
            if reach == Reach::Blind {
                v["depth"] = json!(depth);
            }
        }
        Exact::Round { radius, .. } => v["size"] = json!(radius),
        Exact::Chamfer { leg, .. } => v["size"] = json!(leg),
    }
    v
}

/// A SIZE OF A GROUP SET: a number, or an expression kept on the node under the key of the group as the node numbers
/// it.
struct Typed {
    key: String,
    expr: Option<String>,
}

pub const RECOGNISE_ALL: Tool = Tool {
    name: "recognise_all",
    description: "Find every hole, rounding and chamfer of a body - a STEP, a recognised body or a body made of a mesh - in groups of one kind and size (a blind hole one depth as well), numbered from 0: holes, then chamfers, then roundings, each from the smaller up. then \"list\" answers the groups and lays nothing; \"lay\" (the default) lays one node: the faces of all the groups taken removed at once and each feature made again by its own tool, the sizes brought to what was drawn. kinds takes only those kinds; skip leaves out groups by number; sizes sets a group's size (the diameter, radius or leg) and the depth of a blind hole. The node's sizes are one per group: diameter, radius or dist for the first group laid, diameter.2 for the second and so on. A group the kernel cannot lay is refused with the group named: skip it and call again.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": crate::tools::features::body_field(),
            "kinds": { "type": "array", "items": { "enum": ["hole", "rounding", "chamfer"] }, "description": "The kinds to take; all of them by default." },
            "skip": { "type": "array", "items": { "type": "integer", "minimum": 0 }, "description": "The numbers of the groups to leave as they are." },
            "sizes": { "type": "array", "items": { "type": "object", "properties": {
                "group": { "type": "integer", "minimum": 0 },
                "size": amount_schema("The diameter, radius or leg of the group, mm"),
                "depth": amount_schema("The depth of a group of blind holes, mm"),
            }, "required": ["group"], "additionalProperties": false } },
            "then": { "enum": ["list", "lay"], "description": "list answers the groups; lay (the default) lays them." },
        }, "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        use qymcad_meshfit::feature::{features_of, grouped};
        let a: RecogniseAllArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let src = crate::tools::features::target(project, a.body)?;
        let b = project.mesh_index(src).and_then(|mi| project.bodies.get(mi)).ok_or_else(|| Refusal::new("no-body", &format!("Body {src} is not built."), Stage::Resolve))?;
        let everything = features_of(&b.mesh, &b.faces, 1.0);
        let mut groups = grouped(&everything.found, &b.faces);
        let wanted = |g: usize, group: &RemakeGroup| !a.skip.contains(&g) && group.items.first().is_some_and(|r| a.kinds.as_ref().is_none_or(|k| k.contains(&kind_of(&r.exact))));
        if let Some(bad) = a.skip.iter().chain(a.sizes.iter().map(|s| &s.group)).find(|&&g| g >= groups.len()) {
            return Err(Refusal::new("bad-group", &format!("There is no group {bad}: the body gives {} groups.", groups.len()), Stage::Validate));
        }
        let list: Vec<Value> = groups
            .iter()
            .enumerate()
            .map(|(g, group)| {
                let mut v = told(g, group);
                v["taken"] = json!(wanted(g, group));
                v
            })
            .collect();
        if a.then == Then::List {
            let mut out = serde_json::Map::new();
            out.insert("groups".into(), json!(list));
            out.insert("unfound".into(), json!(everything.unfound.len()));
            return Ok(out);
        }
        // the sizes asked written into their groups, and the expressions kept for the node under the keys it gives them
        let mut typed: Vec<Typed> = Vec::new();
        let mut k = 0u32;
        for (g, group) in groups.iter_mut().enumerate() {
            if !wanted(g, group) {
                continue;
            }
            for s in a.sizes.iter().filter(|s| s.group == g) {
                let size = s.size.as_ref().map(|v| crate::tools::modify::positive(project, v, "size")).transpose()?;
                let depth = s.depth.as_ref().map(|v| crate::tools::modify::positive(project, v, "depth")).transpose()?;
                for r in group.items.iter_mut() {
                    match &mut r.exact {
                        Exact::Hole { diameter, depth: d, .. } => {
                            *diameter = size.as_ref().map_or(*diameter, |s| s.value);
                            *d = depth.as_ref().map_or(*d, |s| s.value);
                        }
                        Exact::Round { radius, .. } => *radius = size.as_ref().map_or(*radius, |s| s.value),
                        Exact::Chamfer { leg, .. } => *leg = size.as_ref().map_or(*leg, |s| s.value),
                    }
                }
                let base = group.items.first().map_or("diameter", |r| match r.exact {
                    Exact::Hole { .. } => "diameter",
                    Exact::Round { .. } => "radius",
                    Exact::Chamfer { .. } => "dist",
                });
                typed.push(Typed { key: group_key(base, k).into_owned(), expr: size.and_then(|s| s.expr) });
                typed.push(Typed { key: group_key("depth", k).into_owned(), expr: depth.and_then(|s| s.expr) });
            }
            k += 1;
        }
        let laid: Vec<RemakeGroup> = groups.into_iter().enumerate().filter(|(g, group)| wanted(*g, group)).map(|(_, group)| group).collect();
        if laid.is_empty() {
            return Err(Refusal::new("nothing-taken", "No group is taken: the body gives none, or kinds and skip left them all out.", Stage::Validate).with_hint("Call with then \"list\" to see the groups."));
        }
        let node = ctx
            .doc
            .edit("f-recognise-all", |p| {
                // the node stands in the body's own part, as the window works in the part it entered
                let part = p.body_owner(src);
                p.set_active_component(part);
                let node = p.add_remade_features(src, laid);
                for t in &typed {
                    p.write_feat_formula(&qymcad_core::model::FeatNumber { node, key: &t.key }, &t.expr.clone().unwrap_or_default(), qymcad_core::model::PlainNumber::ClearsFormula).map_err(|e| e.to_string())?;
                }
                Ok(node)
            })
            .map_err(tool::doc_refusal)?;
        // a group that does not lay is named, as the window names it beside its row
        let refused = ctx.doc.project().regen_errors.get(&node).cloned();
        if let Some(e) = refused {
            let taken: Vec<usize> = list.iter().filter(|v| v["taken"] == json!(true)).filter_map(|v| v["group"].as_u64().map(|g| g as usize)).collect();
            let named = e.group().and_then(|k| taken.get(k as usize).copied());
            let words = qymcad_i18n::error_words::error_text(&e);
            let r = match named {
                Some(g) => Refusal::new("group-refused", &format!("Group {g} does not lay: {words}"), Stage::Kernel).with_hint(&format!("Call again with skip [{g}].")),
                None => Refusal::new("refused", &words, Stage::Kernel),
            };
            ctx.doc.undo();
            return Err(r);
        }
        let mut out = crate::tools::modify::laid_node(ctx, node);
        out.insert("groups".into(), json!(list));
        out.insert("unfound".into(), json!(everything.unfound.len()));
        Ok(out)
    },
};

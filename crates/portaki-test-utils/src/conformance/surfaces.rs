//! Every declared surface renders in its shell with an empty mock, into contract primitives.

use portaki_sdk::sdui::component::Component;
use portaki_sdk::sdui::primitives::Select;
use portaki_sdk::sdui::surface::Surface;
use portaki_sdk::wasm::registry::{HandlerDeclaration, HandlerKind};
use serde_json::Value;

use super::invoke::{describe, empty_params, invoke, surface_mock, Invocation, Outcome};
use super::{Findings, Module, MANIFEST_FILE, NO_DECLARATIONS};
use crate::SurfaceAssertions;

pub(super) fn check(module: &Module) -> Result<(), Findings> {
    Findings::of("surfaces", problems(module))
}

/// Every declared surface, rendered once with an empty mock in its own shell.
pub(super) fn render_all(module: &Module) -> Vec<(&'static HandlerDeclaration, Invocation)> {
    let module_id = module_id(module);
    module
        .declarations()
        .into_iter()
        .filter(|declaration| declaration.kind == HandlerKind::Surface)
        .map(|declaration| {
            let mock = surface_mock(declaration, module_id.as_deref());
            (declaration, invoke(declaration, mock, empty_params()))
        })
        .collect()
}

/// The manifest's `id`, when it can be read.
pub(super) fn module_id(module: &Module) -> Option<String> {
    module
        .manifest()
        .ok()
        .flatten()
        .and_then(|manifest| manifest.get("id")?.as_str().map(str::to_string))
}

fn problems(module: &Module) -> Vec<String> {
    let declarations = module.declarations();
    if declarations.is_empty() {
        return vec![NO_DECLARATIONS.to_string()];
    }

    let mut problems = Vec::new();
    for (declaration, invocation) in render_all(module) {
        let what = describe(declaration);
        match invocation.outcome {
            Outcome::Panicked(message) => {
                problems.push(format!("{what} panicked with an empty mock: {message}"))
            }
            Outcome::Failed(error) => problems.push(format!(
                "{what} failed with an empty mock — a first install has no data either, render an \
                 empty state: {error}"
            )),
            Outcome::Answered(tree) => problems.extend(
                contract_problems(&tree)
                    .into_iter()
                    .map(|problem| format!("{what} {problem}")),
            ),
        }
    }

    problems.extend(undeclared_guest_routes(module, &declarations));
    problems
}

/// What the shell would refuse in the JSON a surface sends.
pub(super) fn contract_problems(tree: &Value) -> Vec<String> {
    let surface: Surface = match serde_json::from_value(tree.clone()) {
        Ok(surface) => surface,
        Err(error) => {
            return vec![format!(
                "sent a tree that does not parse as SDUI primitives of the contract: {error}"
            )]
        }
    };

    // Typed Rust cannot build a node outside the contract; this guards the wire instead — a
    // tree that re-parsed into a variant the contract does not list would be a generator bug.
    let mut unknown: Vec<String> = Vec::new();
    let mut problems = Vec::new();
    for node in SurfaceAssertions::new(&surface).nodes() {
        let name = node.type_name();
        if !Component::TYPE_NAMES.contains(&name) {
            unknown.push(name.to_string());
        }
        if let Component::Select(select) = node {
            problems.extend(select_problems(select));
        }
    }
    if !unknown.is_empty() {
        problems.push(format!(
            "rendered nodes outside the SDUI contract: {}",
            unknown.join(", ")
        ));
    }
    problems
}

/// A `Select` the host cannot use: nothing to pick, or a value none of its options carry.
///
/// An empty `value` means « nothing picked yet » and passes.
fn select_problems(select: &Select) -> Vec<String> {
    let name = select.name.as_deref().unwrap_or("?");
    let options = select.options.as_deref().unwrap_or_default();
    if options.is_empty() {
        return vec![format!("rendered Select `{name}` without options")];
    }
    match select.value.as_deref() {
        Some(value) if !value.is_empty() && !options.iter().any(|o| o.value == value) => {
            vec![format!(
                "rendered Select `{name}` with value `{value}`, which none of its options carries"
            )]
        }
        _ => Vec::new(),
    }
}

/// A manifest route to a surface the module does not serve: the booklet link leads nowhere.
fn undeclared_guest_routes(module: &Module, declarations: &[&HandlerDeclaration]) -> Vec<String> {
    let Ok(Some(manifest)) = module.manifest() else {
        return Vec::new();
    };
    let routes = manifest
        .get("guestSurfaces")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    routes
        .iter()
        .filter_map(|route| route.get("surfaceId")?.as_str())
        .filter(|surface_id| {
            !declarations.iter().any(|declaration| {
                declaration.kind == HandlerKind::Surface
                    && declaration.context == "guest"
                    && declaration.name == *surface_id
            })
        })
        .map(|surface_id| {
            format!(
                "{MANIFEST_FILE} routes guestSurfaces `{surface_id}`, but no \
                 #[surface(guest, id = \"{surface_id}\")] is declared"
            )
        })
        .collect()
}

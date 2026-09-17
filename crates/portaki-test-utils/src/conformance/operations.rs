//! Every declared command and query survives empty input.

use portaki_sdk::wasm::registry::{HandlerDeclaration, HandlerKind};

use super::invoke::{describe, empty_params, invoke, with_module_id, Invocation, Outcome};
use super::surfaces::module_id;
use super::{Findings, Module, NO_DECLARATIONS};
use crate::MockContextBuilder;

pub(super) fn check(module: &Module) -> Result<(), Findings> {
    Findings::of("operations", problems(module))
}

/// Every command and query, dispatched with `{}` in a guest mock then a host mock.
pub(super) fn dispatch_all(
    module: &Module,
) -> Vec<(&'static HandlerDeclaration, &'static str, Invocation)> {
    let module_id = module_id(module);
    let mut all = Vec::new();
    for declaration in module.declarations() {
        if declaration.kind == HandlerKind::Surface {
            continue;
        }
        for (shell, mock) in [
            ("guest", MockContextBuilder::guest()),
            ("host", MockContextBuilder::host()),
        ] {
            let mut mock = with_module_id(mock, module_id.as_deref());
            mock.context.surface = None;
            all.push((
                declaration,
                shell,
                invoke(declaration, mock, empty_params()),
            ));
        }
    }
    all
}

fn problems(module: &Module) -> Vec<String> {
    if module.declarations().is_empty() {
        return vec![NO_DECLARATIONS.to_string()];
    }

    dispatch_all(module)
        .into_iter()
        .filter_map(
            |(declaration, shell, invocation)| match invocation.outcome {
                // An error is a legitimate answer to input that says nothing.
                Outcome::Answered(_) | Outcome::Failed(_) => None,
                Outcome::Panicked(message) => Some(format!(
                    "{} panicked on {{}} in a {shell} mock — a panic aborts the whole Wasm \
                 invocation, return an Err instead: {message}",
                    describe(declaration)
                )),
            },
        )
        .collect()
}

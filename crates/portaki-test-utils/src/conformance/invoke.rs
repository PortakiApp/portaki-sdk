//! Calling a declared handler the way the runtime would, and noting what happened.

use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;

use portaki_sdk::wasm::registry::{HandlerDeclaration, HandlerKind, WasmHandlerFn};
use portaki_sdk::Context;
use serde_json::Value;

use crate::{LogLine, MockContextBuilder, MockHostFunctions};

/// How one invocation ended.
pub(crate) enum Outcome {
    /// The handler answered — its JSON result.
    Answered(Value),
    /// The handler returned an error.
    Failed(String),
    /// The handler panicked — the message, when it carried one.
    Panicked(String),
}

/// One invocation, and what the module asked of the host along the way.
pub(crate) struct Invocation {
    pub outcome: Outcome,
    pub translated_keys: Vec<String>,
    pub logs: Vec<LogLine>,
}

/// Runs `declaration` with `params` inside `mock`, the way the Wasm dispatcher calls the shim.
pub(crate) fn invoke(
    declaration: &HandlerDeclaration,
    mock: MockContextBuilder,
    params: Value,
) -> Invocation {
    let (ctx, host) = mock.build();
    invoke_in(declaration, ctx, host, params)
}

/// Runs a bare dispatch shim with `params` inside `mock`.
pub(crate) fn invoke_dispatch(
    dispatch: WasmHandlerFn,
    mock: MockContextBuilder,
    params: Value,
) -> Invocation {
    let (ctx, host) = mock.build();
    run(dispatch, ctx, host, params)
}

/// Same, on a host already built — so a second call sees what the first one stored.
pub(crate) fn invoke_in(
    declaration: &HandlerDeclaration,
    ctx: Context,
    host: Arc<MockHostFunctions>,
    params: Value,
) -> Invocation {
    run(declaration.dispatch, ctx, host, params)
}

fn run(
    dispatch: WasmHandlerFn,
    ctx: Context,
    host: Arc<MockHostFunctions>,
    params: Value,
) -> Invocation {
    let backend = Arc::clone(&host);
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        portaki_sdk::host::with_host(backend, ctx.clone(), || dispatch(ctx, params))
    }));

    let outcome = match result {
        Ok(Ok(value)) => Outcome::Answered(value),
        Ok(Err(error)) => Outcome::Failed(error.to_string()),
        Err(payload) => Outcome::Panicked(panic_message(payload.as_ref())),
    };
    Invocation {
        outcome,
        translated_keys: host.translated_keys(),
        logs: host.logs(),
    }
}

/// `called Option::unwrap() on a None value`, not `Any { .. }`.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "a panic without a message".to_string()
    }
}

/// `guest surface `home.card` (render_home_card)`, `command `updateConfig` (update_config)`.
pub(crate) fn describe(declaration: &HandlerDeclaration) -> String {
    let what = match declaration.kind {
        HandlerKind::Surface => format!("{} surface", declaration.context),
        HandlerKind::Query => "query".to_string(),
        HandlerKind::Command => "command".to_string(),
    };
    format!("{what} `{}` ({})", declaration.name, declaration.fn_name)
}

/// The mock a surface renders in: its own shell, its own surface id.
pub(crate) fn surface_mock(
    declaration: &HandlerDeclaration,
    module_id: Option<&str>,
) -> MockContextBuilder {
    let mut mock = if declaration.context == "host" {
        MockContextBuilder::host()
    } else {
        MockContextBuilder::guest()
    };
    mock.context.surface = Some(declaration.name.to_string());
    with_module_id(mock, module_id)
}

/// The manifest's module id on the context, when there is one — actions a surface builds carry it.
pub(crate) fn with_module_id(
    mut mock: MockContextBuilder,
    module_id: Option<&str>,
) -> MockContextBuilder {
    if let Some(id) = module_id {
        mock.context.module_id = portaki_sdk::ids::ModuleId::new(id);
    }
    mock
}

/// Input as an empty host form or a bare dispatch sends it.
pub(crate) fn empty_params() -> Value {
    Value::Object(serde_json::Map::new())
}

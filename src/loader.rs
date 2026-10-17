//! Background loading for API-backed views.

use anyhow::Result;
use gpui_kit::{Context, Task};

use crate::api::ApiClient;

/// The lifecycle of one piece of remote data owned by a view.
#[derive(Debug, Clone, Default)]
pub enum Loadable<T> {
    #[default]
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

impl<T> Loadable<T> {
    pub fn ready(&self) -> Option<&T> {
        match self {
            Loadable::Ready(value) => Some(value),
            _ => None,
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Loadable::Loading)
    }

    pub fn from_result(result: Result<T>) -> Self {
        match result {
            Ok(value) => Loadable::Ready(value),
            Err(error) => Loadable::Failed(describe(&error)),
        }
    }
}

/// A user-facing message for a failed request.
pub fn describe(error: &anyhow::Error) -> String {
    if crate::api::is_not_found(error) {
        return "No record was found.".into();
    }
    let root = error.root_cause().to_string();
    let top = error.to_string();
    if root == top {
        top
    } else {
        format!("{top} ({root})")
    }
}

/// Runs `fetch` on the background executor and hands its result to `apply` on
/// the UI thread. Dropping the returned task cancels delivery, so a view keeps
/// only its newest task and a superseded response never overwrites newer state.
pub fn fetch<V, T>(
    cx: &mut Context<V>,
    fetch: impl FnOnce(&ApiClient) -> Result<T> + Send + 'static,
    apply: impl FnOnce(&mut V, Result<T>, &mut Context<V>) + 'static,
) -> Task<()>
where
    V: 'static,
    T: Send + 'static,
{
    let work = cx
        .background_executor()
        .spawn(async move { fetch(ApiClient::global()) });
    cx.spawn(async move |this, cx| {
        let result = work.await;
        this.update(cx, |view, cx| {
            apply(view, result, cx);
            cx.notify();
        })
        .ok();
    })
}

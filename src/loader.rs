//! Background loading for API-backed views.

use std::collections::HashMap;
use std::time::SystemTime;

use anyhow::Result;
use gpui_kit::{App, Context, EntityId, Global, Task};

use crate::api::{ApiClient, track_stale};

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

/// Views showing cached data the service could not refresh, keyed by view and
/// the kind of data, with when that data was fetched. The workspace shows a
/// notice for the current page while it has an entry.
#[derive(Default)]
pub struct StaleData(HashMap<(EntityId, &'static str), SystemTime>);

impl Global for StaleData {}

impl StaleData {
    /// When the oldest stale data `view` shows was fetched.
    pub fn oldest(view: EntityId, cx: &App) -> Option<SystemTime> {
        let stale = cx.try_global::<StaleData>()?;
        stale
            .0
            .iter()
            .filter(|((id, _), _)| *id == view)
            .map(|(_, fetched)| *fetched)
            .min()
    }

    /// Forgets views for which `keep` is false.
    pub fn retain(cx: &mut App, keep: impl Fn(EntityId) -> bool) {
        if cx
            .try_global::<StaleData>()
            .is_some_and(|stale| stale.0.keys().any(|(id, _)| !keep(*id)))
        {
            cx.global_mut::<StaleData>()
                .0
                .retain(|(id, _), _| keep(*id));
        }
    }

    fn record(view: EntityId, kind: &'static str, stale: Option<SystemTime>, cx: &mut App) {
        let key = (view, kind);
        match stale {
            Some(fetched) => {
                cx.default_global::<StaleData>().0.insert(key, fetched);
            }
            None => {
                if cx
                    .try_global::<StaleData>()
                    .is_some_and(|s| s.0.contains_key(&key))
                {
                    cx.global_mut::<StaleData>().0.remove(&key);
                }
            }
        }
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
        .spawn(async move { track_stale(|| fetch(ApiClient::global())) });
    cx.spawn(async move |this, cx| {
        let (result, stale) = work.await;
        this.update(cx, |view, cx| {
            // A result of the same kind replaces the last one, so a fresh
            // load clears the notice for that data.
            let stale = stale.filter(|_| result.is_ok());
            StaleData::record(cx.entity_id(), std::any::type_name::<T>(), stale, cx);
            apply(view, result, cx);
            cx.notify();
        })
        .ok();
    })
}

//! In-app navigation handle handed to every page.

use gpui_kit::component::{
    Sizable as _,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::*;

use crate::route::Route;
use crate::workspace::Workspace;

/// A cloneable, weak handle to the workspace that pages use to navigate.
///
/// Navigation is deferred to the end of the current effect cycle, so it is
/// safe to call from any click or event handler, including ones running while
/// the workspace itself is being updated.
#[derive(Clone)]
pub struct Navigator {
    workspace: WeakEntity<Workspace>,
}

impl Navigator {
    pub fn new(workspace: WeakEntity<Workspace>) -> Self {
        Self { workspace }
    }

    pub fn go(&self, route: Route, window: &mut Window, cx: &mut App) {
        let workspace = self.workspace.clone();
        window.defer(cx, move |window, cx| {
            workspace
                .update(cx, |workspace, cx| workspace.navigate(route, window, cx))
                .ok();
        });
    }

    /// A click handler that opens `route`.
    pub fn to(&self, route: Route) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        let nav = self.clone();
        move |_, window, cx| nav.go(route.clone(), window, cx)
    }

    /// A quiet inline Button that opens `route`, for names inside tables and cards.
    pub fn button(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        route: Route,
    ) -> Button {
        Button::new(id)
            .ghost()
            .small()
            .compact()
            .label(label)
            .on_click(self.to(route))
    }
}

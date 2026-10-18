//! Managers: every manager by career wins or by name, with a name filter.

use std::time::Duration;

use gpui_kit::component::{
    Selectable as _, Sizable as _,
    button::{Button, ButtonGroup, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::{ManagerSummary, Paged};
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::grid::{Col, grid};
use crate::ui::widgets;

const PAGE_SIZE: u32 = 50;

pub struct ManagersPage {
    nav: Navigator,
    scroll: ScrollHandle,
    query: Option<String>,
    by_name: bool,
    page: u32,
    name: Entity<InputState>,
    managers: Loadable<Paged<ManagerSummary>>,
    _task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl ManagersPage {
    pub fn new(nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Filter by name…"));
        let subscriptions =
            vec![
                cx.subscribe_in(&name, window, |this, state, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        let query = state.read(cx).value().trim().to_string();
                        this.query_changed(query, cx);
                    }
                }),
            ];
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            query: None,
            by_name: false,
            page: 1,
            name,
            managers: Loadable::Idle,
            _task: None,
            _subscriptions: subscriptions,
        };
        page.reload(window, cx);
        page
    }

    /// Debounces the name filter like the web (300 ms), then re-queries.
    fn query_changed(&mut self, query: String, cx: &mut Context<Self>) {
        let query = (!query.is_empty()).then_some(query);
        if query == self.query {
            return;
        }
        self.query = query;
        self.page = 1;
        self.managers = Loadable::Loading;
        self._task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(300))
                .await;
            let Ok(task) = this.update(cx, |this, cx| this.fetch(cx)) else {
                return;
            };
            task.await;
        }));
        cx.notify();
    }

    fn fetch(&mut self, cx: &mut Context<Self>) -> Task<()> {
        let (query, by_name, page) = (self.query.clone(), self.by_name, self.page);
        loader::fetch(
            cx,
            move |api| api.managers(query.as_deref(), by_name, page, PAGE_SIZE),
            |this: &mut Self, result, _| this.managers = Loadable::from_result(result),
        )
    }

    fn set_sort(&mut self, by_name: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.by_name != by_name {
            self.by_name = by_name;
            self.page = 1;
            self.reload(window, cx);
        }
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.query = None;
        self.page = 1;
        self.name
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.reload(window, cx);
    }
}

impl Page for ManagersPage {
    fn title(&self, _: &App) -> SharedString {
        "Managers".into()
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.managers = Loadable::Loading;
        self._task = Some(self.fetch(cx));
        cx.notify();
    }
}

impl Render for ManagersPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let subtitle = self
            .managers
            .ready()
            .map(|p| format!("{} managers", format::int(Some(p.total_count as i64))).into());
        let sort = ButtonGroup::new("managers-sort")
            .small()
            .outline()
            .child(
                Button::new("sort-wins")
                    .label("By Wins")
                    .selected(!self.by_name),
            )
            .child(
                Button::new("sort-name")
                    .label("By Name")
                    .selected(self.by_name),
            )
            .on_click(cx.listener(|this, selected: &Vec<usize>, window, cx| {
                this.set_sort(selected.first() == Some(&1), window, cx)
            }));
        let filters = widgets::filter_bar(cx)
            .child(widgets::field(
                "Name",
                div()
                    .w(px(240.))
                    .child(Input::new(&self.name).small().cleanable(true)),
                cx,
            ))
            .child(widgets::field("Sort", sort, cx))
            .when(self.query.is_some(), |this| {
                this.child(
                    Button::new("managers-clear")
                        .small()
                        .ghost()
                        .label("Clear Filters")
                        .on_click(cx.listener(|this, _, window, cx| this.clear(window, cx))),
                )
            });
        let body: AnyElement = match &self.managers {
            Loadable::Ready(p) if p.data.is_empty() => widgets::empty_state(
                "No Managers Found",
                "Try adjusting your search criteria.",
                cx,
            ),
            Loadable::Ready(p) => {
                let nav = self.nav.clone();
                let cols = vec![
                    Col::element("Manager", 260., move |m: &ManagerSummary, _| {
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(nav.button(
                                SharedString::from(format!("mgr-{}", m.player_id)),
                                m.full_name.clone(),
                                Route::Manager {
                                    id: m.player_id.clone(),
                                },
                            ))
                            .when(m.is_hall_of_famer, |this| this.child(widgets::hof_tag()))
                    }),
                    Col::text("Years", 110., |m: &ManagerSummary| {
                        format::year_span(m.first_year, m.last_year)
                    }),
                    Col::num("Seasons", 72., |m: &ManagerSummary| format::int(m.seasons)),
                    Col::num("G", 72., |m: &ManagerSummary| format::int(m.games)),
                    Col::num("W", 72., |m: &ManagerSummary| format::int(m.wins)),
                    Col::num("L", 72., |m: &ManagerSummary| format::int(m.losses)),
                    Col::num("PCT", 64., |m: &ManagerSummary| format::rate(m.win_pct)),
                ];
                let this = cx.entity().downgrade();
                v_flex()
                    .gap_3()
                    .child(widgets::section("Managers", cx).child(grid(
                        "managers-table",
                        cols,
                        &p.data,
                        cx,
                    )))
                    .child(widgets::pager(
                        "managers-pager",
                        p.page,
                        p.total_pages,
                        p.total_count,
                        move |page, window, cx| {
                            this.update(cx, |this, cx| {
                                this.page = page;
                                this.scroll.set_offset(point(px(0.), px(0.)));
                                this.reload(window, cx);
                            })
                            .ok();
                        },
                        cx,
                    ))
                    .into_any_element()
            }
            Loadable::Failed(message) => widgets::error_state(
                "managers-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            _ => widgets::loading_state("Loading managers…", cx),
        };
        widgets::page_body("managers", &self.scroll).child(
            widgets::page_column()
                .child(widgets::page_header(
                    Some("Stats"),
                    "Managers",
                    subtitle,
                    vec![],
                    cx,
                ))
                .child(filters)
                .child(body),
        )
    }
}

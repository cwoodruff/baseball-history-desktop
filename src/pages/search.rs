//! Full search results for players and teams.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Sizable as _,
    button::Button,
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::{SearchFranchise, SearchPlayer, SearchResults};
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::grid::{Col, grid};
use crate::ui::widgets;

/// The most results one search returns; players are then paged locally.
const LIMIT: u32 = 100;
const PAGE_SIZE: usize = 25;

pub struct SearchPage {
    nav: Navigator,
    scroll: ScrollHandle,
    input: Entity<InputState>,
    query: String,
    page: u32,
    results: Loadable<SearchResults>,
    _task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl SearchPage {
    pub fn new(query: String, nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Search players and teams…");
            state.set_value(query.clone(), window, cx);
            state
        });
        input.update(cx, |state, cx| state.focus(window, cx));
        let subscription = cx.subscribe_in(
            &input,
            window,
            |this, state, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    this.query = state.read(cx).value().trim().to_string();
                    this.page = 1;
                    // Debounce typing like the web search box.
                    this._task = Some(cx.spawn_in(window, async move |this, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(300))
                            .await;
                        this.update_in(cx, |this, window, cx| this.reload(window, cx))
                            .ok();
                    }));
                }
                InputEvent::PressEnter { .. } => {
                    this.query = state.read(cx).value().trim().to_string();
                    this.page = 1;
                    this.reload(window, cx);
                }
                _ => {}
            },
        );
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            input,
            query: query.trim().to_string(),
            page: 1,
            results: Loadable::Idle,
            _task: None,
            _subscriptions: vec![subscription],
        };
        page.reload(window, cx);
        page
    }
}

impl Page for SearchPage {
    fn title(&self, _: &App) -> SharedString {
        if self.query.is_empty() {
            "Search".into()
        } else {
            format!("Search: {}", self.query).into()
        }
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.query.chars().count() < 2 {
            self.results = Loadable::Idle;
            self._task = None;
            cx.notify();
            return;
        }
        let query = self.query.clone();
        self.results = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| api.search(&query, LIMIT),
            |this: &mut Self, result, _| this.results = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for SearchPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let input = div().w(px(480.)).child(
            Input::new(&self.input)
                .cleanable(true)
                .prefix(gpui_kit::component::Icon::new(IconName::Search).small()),
        );
        let body: AnyElement = match &self.results {
            Loadable::Idle => widgets::note(
                "Enter at least two characters to search players and teams.",
                cx,
            ),
            Loadable::Ready(r) if r.players.is_empty() && r.franchises.is_empty() => v_flex()
                .gap_3()
                .child(widgets::empty_state(
                    "No Results",
                    &format!(
                        "No results found for “{}”. Check the spelling or try just a last name.",
                        self.query
                    ),
                    cx,
                ))
                .child(
                    h_flex().justify_center().child(
                        Button::new("browse-players")
                            .outline()
                            .label("Browse all players")
                            .on_click(self.nav.to(Route::Players { letter: 'A' })),
                    ),
                )
                .into_any_element(),
            Loadable::Ready(r) => {
                let total_pages = r.players.len().div_ceil(PAGE_SIZE) as u32;
                let page = self.page.min(total_pages.max(1));
                let rows: Vec<SearchPlayer> = r
                    .players
                    .iter()
                    .skip((page as usize - 1) * PAGE_SIZE)
                    .take(PAGE_SIZE)
                    .cloned()
                    .collect();
                let nav = self.nav.clone();
                let player_cols = vec![
                    Col::element("Player", 320., move |p: &SearchPlayer, _| {
                        widgets::player_link(
                            SharedString::from(format!("sr-{}", p.player_id)),
                            &p.player_id,
                            p.full_name.clone().unwrap_or_else(|| p.player_id.clone()),
                            p.is_hall_of_famer,
                            &nav,
                        )
                    }),
                    Col::text("Career", 140., |p: &SearchPlayer| {
                        format::year_span(p.debut_year, p.final_year)
                    }),
                ];
                let nav = self.nav.clone();
                let team_cols = vec![
                    Col::element("Team", 320., move |f: &SearchFranchise, _| {
                        nav.button(
                            SharedString::from(format!("sr-team-{}", f.franchise_id)),
                            f.franchise_name
                                .clone()
                                .unwrap_or_else(|| f.franchise_id.clone()),
                            Route::Franchise {
                                id: f.franchise_id.clone(),
                            },
                        )
                    }),
                    Col::text("Status", 160., |f: &SearchFranchise| {
                        if f.is_active {
                            "Active Franchise".into()
                        } else {
                            "Historical Franchise".into()
                        }
                    }),
                    Col::text("Years", 120., |f: &SearchFranchise| {
                        format::year_span(f.first_year, f.last_year)
                    }),
                ];
                let more = r.total_player_count as usize > r.players.len();
                let this = cx.entity().downgrade();
                v_flex()
                    .gap_4()
                    .when(!r.franchises.is_empty(), |this| {
                        this.child(widgets::section_with("Teams", widgets::note(format!("{} teams", r.total_franchise_count), cx), cx).child(grid("search-teams", team_cols, &r.franchises, cx)))
                    })
                    .when(!r.players.is_empty(), |el| {
                        el.child(
                            widgets::section_with("Players", widgets::note(format!("{} players", format::int(Some(r.total_player_count as i64))), cx), cx)
                                .child(grid("search-players", player_cols, &rows, cx)),
                        )
                        .child(widgets::pager(
                            "search-pager",
                            page,
                            total_pages,
                            r.players.len() as u32,
                            move |page, _, cx| {
                                this.update(cx, |this, cx| {
                                    this.page = page;
                                    cx.notify();
                                })
                                .ok();
                            },
                            cx,
                        ))
                    })
                    .when(more, |this| {
                        this.child(widgets::note(
                            format!("Showing the first {} matches. Refine the search to narrow the list.", r.players.len()),
                            cx,
                        ))
                    })
                    .into_any_element()
            }
            Loadable::Failed(message) => widgets::error_state(
                "search-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            Loadable::Loading => widgets::loading_state("Searching…", cx),
        };
        widgets::page_body("search", &self.scroll).child(
            widgets::page_column()
                .child(widgets::page_header(
                    Some("Tools"),
                    "Search",
                    None,
                    vec![],
                    cx,
                ))
                .child(input)
                .child(body),
        )
    }
}

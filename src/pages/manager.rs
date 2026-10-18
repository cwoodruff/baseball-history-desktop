//! Manager detail: career summary, manager awards, season history and split seasons.

use gpui_kit::component::{Sizable as _, StyledExt as _, button::Button, h_flex, tag::Tag, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::{ManagerDetail, ManagerSeason, ManagerSplitHalf};
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::grid::{Col, grid};
use crate::ui::widgets;

pub struct ManagerPage {
    id: String,
    nav: Navigator,
    scroll: ScrollHandle,
    manager: Loadable<ManagerDetail>,
    _task: Option<Task<()>>,
}

/// `1 Pennant`, `3 Pennants`: the label for a count, singular when it is 1.
fn plural(count: Option<i64>, one: &'static str, many: &'static str) -> &'static str {
    if count == Some(1) { one } else { many }
}

fn team_link(
    nav: &Navigator,
    id: String,
    team_id: &str,
    team_name: Option<&str>,
    lg_id: Option<&str>,
    year: i32,
) -> AnyElement {
    let label = team_name.unwrap_or(team_id).to_string();
    match lg_id.filter(|lg| !lg.is_empty()) {
        Some(lg) => nav
            .button(
                SharedString::from(id),
                label,
                Route::TeamSeason {
                    team_id: team_id.to_string(),
                    lg_id: lg.to_string(),
                    year,
                },
            )
            .into_any_element(),
        None => label.into_any_element(),
    }
}

/// The table of team-seasons a manager ran. `results` adds the team's
/// Pennant and WS flags (the Manager page); the player's Managing tab shows
/// only the player-manager marker.
pub fn seasons_grid(
    id: &str,
    rows: &[ManagerSeason],
    results: bool,
    nav: &Navigator,
    cx: &App,
) -> AnyElement {
    let nav = nav.clone();
    let key = id.to_string();
    let cols = vec![
        Col::text("Year", 64., |s: &ManagerSeason| s.year.to_string()),
        Col::element("Team", 220., move |s: &ManagerSeason, ix| {
            team_link(
                &nav,
                format!("{key}-team-{ix}"),
                &s.team_id,
                s.team_name.as_deref(),
                s.lg_id.as_deref(),
                s.year,
            )
        }),
        Col::text("League", 64., |s: &ManagerSeason| {
            format::text(s.lg_id.as_deref())
        }),
        Col::num("G", 56., |s: &ManagerSeason| format::int(s.games)),
        Col::num("W", 56., |s: &ManagerSeason| format::int(s.wins)),
        Col::num("L", 56., |s: &ManagerSeason| format::int(s.losses)),
        Col::num("PCT", 64., |s: &ManagerSeason| format::rate(s.win_pct)),
        Col::num("Finish", 64., |s: &ManagerSeason| format::int(s.rank)),
        Col::element("", 200., move |s: &ManagerSeason, _| {
            let mut row = h_flex().gap_1();
            if results && s.won_world_series {
                row = row.child(Tag::success().xsmall().child("WS"));
            } else if results && s.won_pennant {
                row = row.child(Tag::primary().xsmall().child("Pennant"));
            }
            if s.is_player_manager {
                row = row.child(
                    div()
                        .id(SharedString::from(format!("pm-{}-{}", s.year, s.team_id)))
                        .tooltip(|window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new("Player-manager")
                                .build(window, cx)
                        })
                        .child(Tag::secondary().xsmall().child("Player-Mgr")),
                );
            }
            row
        }),
    ];
    grid(SharedString::from(id.to_string()), cols, rows, cx)
}

impl ManagerPage {
    pub fn new(id: String, nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            id,
            nav,
            scroll: ScrollHandle::new(),
            manager: Loadable::Idle,
            _task: None,
        };
        page.reload(window, cx);
        page
    }

    fn render_awards(&self, m: &ManagerDetail, cx: &App) -> Option<AnyElement> {
        if m.awards.is_empty() {
            return None;
        }
        let mut awards = m.awards.clone();
        awards.sort_by_key(|a| std::cmp::Reverse(a.year));
        Some(
            widgets::section("Manager Awards", cx)
                .child(v_flex().px_4().py_2().gap_1().children(
                    // The web's "View Race" link is left out: the API has no
                    // voting data for manager awards.
                    awards.into_iter().map(|a| {
                        let league = a.lg_id.clone().unwrap_or_default();
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().font_bold().child(a.year.to_string()))
                            .child(a.award_id.clone())
                            .when(!league.is_empty(), |this| {
                                this.child(widgets::note(format!("({league})"), cx))
                            })
                    }),
                ))
                .into_any_element(),
        )
    }

    fn render_splits(&self, m: &ManagerDetail, cx: &App) -> Option<AnyElement> {
        if m.split_season_halves.is_empty() {
            return None;
        }
        let mut halves = m.split_season_halves.clone();
        halves.sort_by(|a, b| b.year.cmp(&a.year).then(a.half.cmp(&b.half)));
        let nav = self.nav.clone();
        let cols = vec![
            Col::text("Year", 64., |s: &ManagerSplitHalf| s.year.to_string()),
            Col::element("Team", 220., move |s: &ManagerSplitHalf, ix| {
                team_link(
                    &nav,
                    format!("split-team-{ix}"),
                    &s.team_id,
                    s.team_name.as_deref(),
                    s.lg_id.as_deref(),
                    s.year,
                )
            }),
            Col::num("Half", 56., |s: &ManagerSplitHalf| format::int(s.half)),
            Col::num("G", 56., |s: &ManagerSplitHalf| format::int(s.games)),
            Col::num("W", 56., |s: &ManagerSplitHalf| format::int(s.wins)),
            Col::num("L", 56., |s: &ManagerSplitHalf| format::int(s.losses)),
            Col::num("Finish", 64., |s: &ManagerSplitHalf| format::int(s.rank)),
        ];
        Some(
            widgets::section("Split Seasons", cx)
                .child(grid("manager-splits", cols, &halves, cx))
                .into_any_element(),
        )
    }
}

impl Page for ManagerPage {
    fn title(&self, _: &App) -> SharedString {
        match &self.manager {
            Loadable::Ready(m) => m.full_name.clone().into(),
            _ => "Manager".into(),
        }
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let id = self.id.clone();
        self.manager = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| api.manager(&id),
            |this: &mut Self, result, _| this.manager = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for ManagerPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content: AnyElement = match &self.manager {
            Loadable::Ready(m) => {
                let mut badges = vec![];
                if m.is_hall_of_famer {
                    badges.push(widgets::hof_tag().into_any_element());
                }
                if m.was_player {
                    badges.push(
                        Button::new("manager-player-page")
                            .small()
                            .outline()
                            .label("Player Page")
                            .on_click(self.nav.to(Route::Player {
                                id: m.player_id.clone(),
                            }))
                            .into_any_element(),
                    );
                }
                let mut seasons = m.season_records.clone();
                seasons.sort_by(|a, b| b.year.cmp(&a.year).then(a.order.cmp(&b.order)));
                v_flex()
                    .gap_5()
                    .child(widgets::page_header(
                        Some("Manager"),
                        m.full_name.clone(),
                        m.was_player_manager.then(|| {
                            "Served as a player-manager for at least part of this career.".into()
                        }),
                        badges,
                        cx,
                    ))
                    .child(widgets::stat_row(
                        vec![
                            ("Years", format::year_span(m.first_year, m.last_year)),
                            ("Seasons", format::int(m.seasons)),
                            ("Record", format::record(m.wins, m.losses)),
                            ("PCT", format::rate(m.win_pct)),
                            (
                                plural(m.pennants, "Pennant", "Pennants"),
                                format::int(m.pennants),
                            ),
                            (
                                plural(m.world_series_titles, "WS Title", "WS Titles"),
                                format::int(m.world_series_titles),
                            ),
                        ],
                        cx,
                    ))
                    .children(self.render_awards(m, cx))
                    .child(
                        widgets::section_with(
                            "Season History",
                            widgets::note(format!("{} seasons", seasons.len()), cx),
                            cx,
                        )
                        .child(seasons_grid(
                            "manager-seasons",
                            &seasons,
                            true,
                            &self.nav,
                            cx,
                        ))
                        .child(div().px_4().py_2().child(widgets::note(
                            "Pennant and World Series flags reflect the team's season result, which may span multiple managers.",
                            cx,
                        ))),
                    )
                    .children(self.render_splits(m, cx))
                    .into_any_element()
            }
            Loadable::Failed(message) => {
                let message = if message.starts_with("No record") {
                    "Manager Not Found".to_string()
                } else {
                    message.clone()
                };
                widgets::error_state(
                    "manager-error",
                    &message,
                    cx.listener(|this, _, window, cx| this.reload(window, cx)),
                )
            }
            _ => widgets::loading_state("Loading manager…", cx),
        };
        widgets::page_body("manager", &self.scroll).child(widgets::page_column().child(content))
    }
}

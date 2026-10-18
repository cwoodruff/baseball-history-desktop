//! Team season: record, honors, park, managers, roster leaders and other seasons.

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::{FranchiseDetail, TeamBatter, TeamPitcher, TeamSeason};
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::grid::{Col, grid};
use crate::ui::widgets;

const ROSTER_LIMIT: usize = 25;

pub struct TeamSeasonPage {
    team_id: String,
    lg_id: String,
    year: i32,
    nav: Navigator,
    scroll: ScrollHandle,
    season: Loadable<TeamSeason>,
    franchise: Loadable<FranchiseDetail>,
    _tasks: Vec<Task<()>>,
}

impl TeamSeasonPage {
    pub fn new(
        team_id: String,
        lg_id: String,
        year: i32,
        nav: Navigator,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut page = Self {
            team_id,
            lg_id,
            year,
            nav,
            scroll: ScrollHandle::new(),
            season: Loadable::Idle,
            franchise: Loadable::Idle,
            _tasks: Vec::new(),
        };
        page.reload(window, cx);
        page
    }

    fn load_franchise(&mut self, franchise_id: String, cx: &mut Context<Self>) {
        self.franchise = Loadable::Loading;
        self._tasks.push(loader::fetch(
            cx,
            move |api| api.franchise(&franchise_id),
            |this: &mut Self, result, _| this.franchise = Loadable::from_result(result),
        ));
    }

    fn render_other_seasons(&self, cx: &App) -> AnyElement {
        let Some(f) = self.franchise.ready() else {
            return div().into_any_element();
        };
        let mut seasons = f.seasons.clone();
        seasons.sort_by_key(|s| s.year);
        let Some(ix) = seasons
            .iter()
            .position(|s| s.year == self.year && s.team_id == self.team_id)
        else {
            return div().into_any_element();
        };
        let lo = ix.saturating_sub(6);
        let hi = (ix + 7).min(seasons.len());
        let route = |s: &crate::api::models::FranchiseSeason| Route::TeamSeason {
            team_id: s.team_id.clone(),
            lg_id: s.lg_id.clone().unwrap_or_default(),
            year: s.year,
        };
        let prev = ix.checked_sub(1).map(|i| &seasons[i]);
        let next = seasons.get(ix + 1);
        widgets::section("Other Seasons", cx)
            .child(
                h_flex()
                    .p_3()
                    .gap_1()
                    .flex_wrap()
                    .items_center()
                    .child(
                        Button::new("prev-season")
                            .small()
                            .outline()
                            .icon(IconName::ChevronLeft)
                            .label(
                                prev.map(|s| s.year.to_string())
                                    .unwrap_or_else(|| "Previous".into()),
                            )
                            .disabled(prev.is_none())
                            .when_some(prev, |this, s| this.on_click(self.nav.to(route(s)))),
                    )
                    .children(seasons[lo..hi].iter().map(|s| {
                        let current = s.year == self.year && s.team_id == self.team_id;
                        Button::new(SharedString::from(format!(
                            "other-{}-{}",
                            s.team_id, s.year
                        )))
                        .small()
                        .compact()
                        .when(current, |this| this.primary())
                        .when(!current, |this| this.ghost())
                        .label(s.year.to_string())
                        .on_click(self.nav.to(route(s)))
                    }))
                    .child(
                        Button::new("next-season")
                            .small()
                            .outline()
                            .icon(IconName::ChevronRight)
                            .label(
                                next.map(|s| s.year.to_string())
                                    .unwrap_or_else(|| "Next".into()),
                            )
                            .disabled(next.is_none())
                            .when_some(next, |this, s| this.on_click(self.nav.to(route(s)))),
                    ),
            )
            .into_any_element()
    }
}

use gpui_kit::component::Disableable as _;

impl Page for TeamSeasonPage {
    fn title(&self, _: &App) -> SharedString {
        match &self.season {
            Loadable::Ready(s) => {
                format!("{} {}", s.year, format::text(s.team_name.as_deref())).into()
            }
            _ => format!("{} {}", self.year, self.team_id).into(),
        }
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let (team, lg, year) = (self.team_id.clone(), self.lg_id.clone(), self.year);
        self.season = Loadable::Loading;
        self._tasks = vec![loader::fetch(
            cx,
            move |api| api.team_season(&team, &lg, year),
            |this: &mut Self, result, cx| {
                this.season = Loadable::from_result(result);
                if let Some(franchise_id) = this.season.ready().and_then(|s| s.franchise_id.clone())
                {
                    this.load_franchise(franchise_id, cx);
                }
            },
        )];
        cx.notify();
    }
}

impl Render for TeamSeasonPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content: AnyElement = match &self.season {
            Loadable::Ready(s) => {
                let team_name = format::text(s.team_name.as_deref());
                let mut badges: Vec<AnyElement> = Vec::new();
                if s.won_world_series {
                    badges.push(
                        Tag::custom(
                            widgets::hof_gold(),
                            rgb(0x1a1a1a).into(),
                            rgb(0xb8962e).into(),
                        )
                        .small()
                        .child("World Series Champions")
                        .into_any_element(),
                    );
                }
                if s.won_pennant {
                    badges.push(
                        Tag::primary()
                            .small()
                            .child(format!("{} Pennant", format::text(s.lg_id.as_deref())))
                            .into_any_element(),
                    );
                }
                if s.won_division {
                    badges.push(
                        Tag::success()
                            .small()
                            .child("Division Winners")
                            .into_any_element(),
                    );
                }
                if s.won_wild_card {
                    badges.push(Tag::info().small().child("Wild Card").into_any_element());
                }
                let franchise_button = s.franchise_id.clone().map(|id| {
                    Button::new("franchise-link")
                        .small()
                        .outline()
                        .label(format!(
                            "{} history",
                            format::text(s.franchise_name.as_deref())
                        ))
                        .on_click(self.nav.to(Route::Franchise { id }))
                });
                let header = h_flex()
                    .w_full()
                    .justify_between()
                    .items_end()
                    .gap_3()
                    .child(widgets::page_header(
                        Some(&format!("{} season", format::text(s.lg_id.as_deref()))),
                        format!("{} {}", s.year, team_name),
                        None,
                        badges,
                        cx,
                    ))
                    .children(franchise_button);
                let stats = widgets::stat_row(
                    vec![
                        ("Record", format::record(s.wins, s.losses)),
                        ("Win %", format::rate(s.win_pct)),
                        ("Rank", format::int(s.rank)),
                        ("Runs", format::int(s.batting.as_ref().and_then(|b| b.runs))),
                        (
                            "HR",
                            format::int(s.batting.as_ref().and_then(|b| b.home_runs)),
                        ),
                        (
                            "ERA",
                            format::decimal2(s.pitching.as_ref().and_then(|p| p.era)),
                        ),
                    ],
                    cx,
                );
                let park = widgets::section("Ballpark", cx).child(widgets::facts(
                    vec![
                        ("Home Park", format::text(s.park_name.as_deref())),
                        ("Attendance", format::int(s.attendance)),
                    ],
                    cx,
                ));
                let managers = widgets::section("Manager(s)", cx).child(
                    v_flex()
                        .px_3()
                        .py_2()
                        .gap_1()
                        .children(s.managers.iter().map(|m| {
                            h_flex()
                                .justify_between()
                                .gap_2()
                                .child(widgets::player_link(
                                    SharedString::from(format!("mgr-{}", m.player_id)),
                                    &m.player_id,
                                    format::text(m.full_name.as_deref()),
                                    m.is_hall_of_famer,
                                    &self.nav,
                                ))
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(widgets::note(
                                            format!(
                                                "{} ({} G)",
                                                format::record(m.wins, m.losses),
                                                format::int(m.games)
                                            ),
                                            cx,
                                        ))
                                        .child(
                                            Button::new(SharedString::from(format!(
                                                "mgr-career-{}",
                                                m.player_id
                                            )))
                                            .link()
                                            .xsmall()
                                            .label("Career")
                                            .tooltip("Full managerial record")
                                            .on_click(self.nav.to(Route::Manager {
                                                id: m.player_id.clone(),
                                            })),
                                        ),
                                )
                        })),
                );

                let mut batters = s.batters.clone();
                batters.sort_by_key(|b| std::cmp::Reverse(b.at_bats.unwrap_or(0)));
                let batter_total = batters.len();
                batters.truncate(ROSTER_LIMIT);
                let nav = self.nav.clone();
                let bat_cols = vec![
                    Col::element("Player", 240., move |b: &TeamBatter, _| {
                        widgets::player_link(
                            SharedString::from(format!("bat-{}", b.player_id)),
                            &b.player_id,
                            format::text(b.full_name.as_deref()),
                            b.is_hall_of_famer,
                            &nav,
                        )
                    }),
                    Col::num("G", 56., |b: &TeamBatter| format::int(b.games)),
                    Col::num("AB", 64., |b: &TeamBatter| format::int(b.at_bats)),
                    Col::num("H", 56., |b: &TeamBatter| format::int(b.hits)),
                    Col::num("HR", 56., |b: &TeamBatter| format::int(b.home_runs)),
                    Col::num("RBI", 56., |b: &TeamBatter| format::int(b.rbi)),
                    Col::num("AVG", 64., |b: &TeamBatter| format::rate(b.batting_average)),
                ];
                let mut pitchers = s.pitchers.clone();
                pitchers.sort_by_key(|p| std::cmp::Reverse(p.games.unwrap_or(0)));
                let pitcher_total = pitchers.len();
                pitchers.truncate(ROSTER_LIMIT);
                let nav = self.nav.clone();
                let pit_cols = vec![
                    Col::element("Player", 240., move |p: &TeamPitcher, _| {
                        widgets::player_link(
                            SharedString::from(format!("pit-{}", p.player_id)),
                            &p.player_id,
                            format::text(p.full_name.as_deref()),
                            p.is_hall_of_famer,
                            &nav,
                        )
                    }),
                    Col::num("G", 56., |p: &TeamPitcher| format::int(p.games)),
                    Col::num("W-L", 72., |p: &TeamPitcher| {
                        format::record(p.wins, p.losses)
                    }),
                    Col::num("ERA", 64., |p: &TeamPitcher| format::decimal2(p.era)),
                    Col::num("SO", 56., |p: &TeamPitcher| format::int(p.strikeouts)),
                    Col::num("SV", 56., |p: &TeamPitcher| format::int(p.saves)),
                ];
                let count_note = |shown: usize, total: usize| {
                    if total > shown {
                        format!("top {shown} of {total}")
                    } else {
                        format!("{total} players")
                    }
                };
                v_flex()
                    .gap_5()
                    .child(header)
                    .child(stats)
                    .child(
                        h_flex()
                            .w_full()
                            .gap_5()
                            .items_start()
                            .child(div().flex_1().child(park))
                            .child(div().flex_1().child(managers)),
                    )
                    .child(
                        v_flex()
                            .w_full()
                            .gap_5()
                            .child(
                                div().w_full().child(
                                    widgets::section_with(
                                        "Batting",
                                        widgets::note(count_note(batters.len(), batter_total), cx),
                                        cx,
                                    )
                                    .child(grid(
                                        "team-batters",
                                        bat_cols,
                                        &batters,
                                        cx,
                                    )),
                                ),
                            )
                            .child(
                                div().w_full().child(
                                    widgets::section_with(
                                        "Pitching",
                                        widgets::note(
                                            count_note(pitchers.len(), pitcher_total),
                                            cx,
                                        ),
                                        cx,
                                    )
                                    .child(grid(
                                        "team-pitchers",
                                        pit_cols,
                                        &pitchers,
                                        cx,
                                    )),
                                ),
                            ),
                    )
                    .child(self.render_other_seasons(cx))
                    .into_any_element()
            }
            Loadable::Failed(message) => {
                let message = if message.starts_with("No record") {
                    "Season Not Found".to_string()
                } else {
                    message.clone()
                };
                widgets::error_state(
                    "season-error",
                    &message,
                    cx.listener(|this, _, window, cx| this.reload(window, cx)),
                )
            }
            _ => widgets::loading_state("Loading season…", cx),
        };
        widgets::page_body("team-season", &self.scroll).child(widgets::page_column().child(content))
    }
}

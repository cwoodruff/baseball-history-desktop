//! Home: hero with dataset counts, This Day in Baseball, career leaders,
//! recent inductees and quick links.

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _, button::Button, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::{BattingLeader, HallOfFamer, PitchingLeader, ThisDay, ThisDayEntry};
use crate::api::{LeaderRequest, PlayerQuery};
use crate::catalog;
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::widgets;

#[derive(Clone, Default)]
struct Counts {
    players: u32,
    franchises: usize,
    hall_of_famers: u32,
}

pub struct HomePage {
    nav: Navigator,
    scroll: ScrollHandle,
    counts: Loadable<Counts>,
    hr_leaders: Loadable<Vec<BattingLeader>>,
    win_leaders: Loadable<Vec<PitchingLeader>>,
    inductees: Loadable<Vec<HallOfFamer>>,
    this_day: Loadable<ThisDay>,
    _tasks: Vec<Task<()>>,
}

fn career(stat: &str, page_size: u32) -> LeaderRequest {
    LeaderRequest {
        stat: stat.into(),
        from_year: None,
        to_year: None,
        league: None,
        qualified: false,
        minimum: None,
        single_season: false,
        page: 1,
        page_size,
    }
}

impl HomePage {
    pub fn new(nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            counts: Loadable::Idle,
            hr_leaders: Loadable::Idle,
            win_leaders: Loadable::Idle,
            inductees: Loadable::Idle,
            this_day: Loadable::Idle,
            _tasks: Vec::new(),
        };
        page.reload(window, cx);
        page
    }

    fn leaders_card<T>(
        &self,
        id: &str,
        title: &str,
        data: &Loadable<Vec<T>>,
        row: impl Fn(&T) -> (i64, String, String, bool, String),
        all: Route,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let body: AnyElement = match data {
            Loadable::Ready(rows) => v_flex()
                .py_1()
                .children(rows.iter().map(|r| {
                    let (rank, player_id, name, hof, value) = row(r);
                    h_flex()
                        .px_3()
                        .py_1()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .w(px(24.))
                                .font_bold()
                                .text_color(cx.theme().primary)
                                .child(rank.to_string()),
                        )
                        .child(div().flex_1().min_w_0().child(widgets::player_link(
                            SharedString::from(format!("{id}-{player_id}")),
                            &player_id,
                            name,
                            hof,
                            &self.nav,
                        )))
                        .child(
                            div()
                                .font_bold()
                                .font_family(cx.theme().mono_font_family.clone())
                                .child(value),
                        )
                }))
                .into_any_element(),
            Loadable::Failed(message) => div()
                .px_3()
                .child(widgets::error_state(
                    SharedString::from(format!("{id}-error")),
                    message,
                    cx.listener(|this, _, window, cx| this.reload(window, cx)),
                ))
                .into_any_element(),
            _ => widgets::loading_state("Loading stats…", cx).into_any_element(),
        };
        widgets::section(title.to_string(), cx)
            .flex_1()
            .min_w(px(300.))
            .child(body)
            .child(
                h_flex().px_3().pb_2().child(
                    Button::new(SharedString::from(format!("{id}-all")))
                        .small()
                        .outline()
                        .label("View All Leaders")
                        .on_click(self.nav.to(all)),
                ),
            )
            .into_any_element()
    }

    fn render_hero(&self, cx: &mut Context<Self>) -> AnyElement {
        let years = catalog::LAST_SEASON - catalog::FIRST_SEASON + 1;
        v_flex()
            .w_full()
            .p_6()
            .gap_3()
            .rounded(cx.theme().radius_lg)
            .bg(linear_gradient(
                135.,
                linear_color_stop(widgets::brand_navy(), 0.),
                linear_color_stop(rgb(0x1a4a8f), 1.),
            ))
            .text_color(white())
            .child(div().text_3xl().font_bold().child("⚾ Baseball History"))
            .child(div().text_base().opacity(0.9).child(format!(
                "Explore {years} years of Major League Baseball history ({} - {})",
                catalog::FIRST_SEASON,
                catalog::LAST_SEASON
            )))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("hero-players")
                            .label("Browse Players")
                            .icon(IconName::Users)
                            .on_click(self.nav.to(Route::Players { letter: 'A' })),
                    )
                    .child(
                        Button::new("hero-teams")
                            .outline()
                            .label("View Teams")
                            .icon(IconName::Flag)
                            .on_click(self.nav.to(Route::Teams)),
                    ),
            )
            .into_any_element()
    }

    fn render_counts(&self, cx: &mut Context<Self>) -> AnyElement {
        let (players, franchises, hof) = match &self.counts {
            Loadable::Ready(c) => (
                format::int(Some(c.players as i64)),
                format::int(Some(c.franchises as i64)),
                format::int(Some(c.hall_of_famers as i64)),
            ),
            _ => ("…".to_string(), "…".to_string(), "…".to_string()),
        };
        widgets::stat_row(
            vec![
                ("Players", players),
                ("Franchises", franchises),
                ("Hall of Famers", hof),
            ],
            cx,
        )
        .into_any_element()
    }

    /// Birthdays, debuts and final games on today's date. Hidden when the
    /// date has none, or when the request fails (it is supplementary).
    fn render_this_day(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let day = self.this_day.ready()?;
        if day.birthdays.is_empty() && day.debuts.is_empty() && day.final_games.is_empty() {
            return None;
        }
        let column = |key: &str, heading: &str, entries: &[ThisDayEntry], born: bool| {
            v_flex()
                .flex_1()
                .min_w(px(220.))
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_color(cx.theme().muted_foreground)
                        .child(heading.to_uppercase()),
                )
                .when(entries.is_empty(), |this| {
                    this.child(widgets::note("None on this date.", cx))
                })
                .children(entries.iter().map(|e| {
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(widgets::player_link(
                            SharedString::from(format!("today-{key}-{}", e.player_id)),
                            &e.player_id,
                            e.full_name.clone(),
                            e.is_hall_of_famer,
                            &self.nav,
                        ))
                        .when_some(e.year, |this, year| {
                            let label = if born {
                                format!("b. {year}")
                            } else {
                                year.to_string()
                            };
                            this.child(widgets::note(label, cx))
                        })
                }))
        };
        Some(
            widgets::section(format!("This Day in Baseball — {}", day.label), cx)
                .child(
                    h_flex()
                        .p_3()
                        .gap_5()
                        .flex_wrap()
                        .items_start()
                        .child(column("born", "Birthdays", &day.birthdays, true))
                        .child(column("debut", "Debuts", &day.debuts, false))
                        .child(column("final", "Final Games", &day.final_games, false)),
                )
                .into_any_element(),
        )
    }

    fn render_inductees(&self, cx: &mut Context<Self>) -> AnyElement {
        let body: AnyElement = match &self.inductees {
            Loadable::Ready(rows) => h_flex()
                .p_3()
                .gap_3()
                .flex_wrap()
                .children(rows.iter().map(|h| {
                    let name = format::full_name(h.first_name.as_deref(), h.last_name.as_deref());
                    let initials: String = [h.first_name.as_deref(), h.last_name.as_deref()]
                        .iter()
                        .filter_map(|s| s.and_then(|s| s.chars().next()))
                        .collect();
                    h_flex()
                        .w(px(250.))
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size_10()
                                .flex_shrink_0()
                                .rounded_full()
                                .bg(widgets::hof_gold())
                                .text_color(rgb(0x1a1a1a))
                                .font_bold()
                                .child(initials),
                        )
                        .child(
                            v_flex()
                                .min_w_0()
                                .items_start()
                                .child(widgets::player_link(
                                    SharedString::from(format!("hof-{}", h.player_id)),
                                    &h.player_id,
                                    name,
                                    true,
                                    &self.nav,
                                ))
                                .child(widgets::note(
                                    format!(
                                        "Inducted {} · {}",
                                        format::year(h.induction_year),
                                        format::year_span(h.debut_year, h.final_year)
                                    ),
                                    cx,
                                )),
                        )
                }))
                .into_any_element(),
            Loadable::Failed(message) => div()
                .px_3()
                .child(widgets::error_state(
                    "hof-error",
                    message,
                    cx.listener(|this, _, window, cx| this.reload(window, cx)),
                ))
                .into_any_element(),
            _ => widgets::loading_state("Loading inductees…", cx).into_any_element(),
        };
        widgets::section("Recent Hall of Fame Inductees", cx)
            .child(body)
            .child(
                h_flex().px_3().pb_2().child(
                    Button::new("hof-all")
                        .small()
                        .outline()
                        .label("View All Hall of Famers")
                        .on_click(self.nav.to(Route::HallOfFame {
                            year: None,
                            category: None,
                        })),
                ),
            )
            .into_any_element()
    }

    fn render_quick_links(&self, cx: &mut Context<Self>) -> AnyElement {
        let links = [
            (
                "ql-players",
                IconName::Users,
                "Players",
                "Browse A-Z",
                Route::Players { letter: 'A' },
            ),
            (
                "ql-teams",
                IconName::Flag,
                "Teams",
                "All Franchises",
                Route::Teams,
            ),
            (
                "ql-batting",
                IconName::ChartColumn,
                "Batting Stats",
                "Leaderboards",
                Route::BattingLeaders,
            ),
            (
                "ql-pitching",
                IconName::ChartBar,
                "Pitching Stats",
                "Leaderboards",
                Route::PitchingLeaders,
            ),
        ];
        h_flex()
            .w_full()
            .gap_3()
            .flex_wrap()
            .children(links.into_iter().map(|(id, icon, title, subtitle, route)| {
                Button::new(id)
                    .outline()
                    .flex_1()
                    .min_w(px(180.))
                    .h_auto()
                    .on_click(self.nav.to(route))
                    .child(
                        h_flex()
                            .w_full()
                            .p_2()
                            .gap_3()
                            .items_center()
                            .child(
                                gpui_kit::component::Icon::new(icon)
                                    .large()
                                    .text_color(cx.theme().primary),
                            )
                            .child(
                                v_flex()
                                    .items_start()
                                    .child(div().font_semibold().child(title))
                                    .child(widgets::note(subtitle, cx)),
                            ),
                    )
            }))
            .into_any_element()
    }
}

impl Page for HomePage {
    fn title(&self, _: &App) -> SharedString {
        "Home".into()
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.counts = Loadable::Loading;
        self.hr_leaders = Loadable::Loading;
        self.win_leaders = Loadable::Loading;
        self.inductees = Loadable::Loading;
        self.this_day = Loadable::Loading;
        self._tasks = vec![
            loader::fetch(
                cx,
                |api| {
                    // The players endpoint pages one initial letter at a time, so the
                    // total is the sum of each letter's count.
                    let mut players = 0;
                    for letter in 'A'..='Z' {
                        players += api.players(&PlayerQuery::letter(letter, 1, 1))?.total_count;
                    }
                    let franchises = api.franchises(None, false)?.len();
                    let hall_of_famers = api.hall_of_fame(None, None, 1, 1)?.total_count;
                    Ok(Counts {
                        players,
                        franchises,
                        hall_of_famers,
                    })
                },
                |this: &mut Self, result, _| this.counts = Loadable::from_result(result),
            ),
            loader::fetch(
                cx,
                |api| api.batting_leaders(&career("hr", 5)).map(|p| p.data),
                |this: &mut Self, result, _| this.hr_leaders = Loadable::from_result(result),
            ),
            loader::fetch(
                cx,
                |api| api.pitching_leaders(&career("w", 5)).map(|p| p.data),
                |this: &mut Self, result, _| this.win_leaders = Loadable::from_result(result),
            ),
            loader::fetch(
                cx,
                |api| api.hall_of_fame(None, None, 1, 6).map(|p| p.data),
                |this: &mut Self, result, _| this.inductees = Loadable::from_result(result),
            ),
            loader::fetch(
                cx,
                |api| {
                    use chrono::Datelike as _;
                    let today = chrono::Local::now().date_naive();
                    api.this_day(today.month(), today.day(), 5)
                },
                |this: &mut Self, result, _| this.this_day = Loadable::from_result(result),
            ),
        ];
        cx.notify();
    }
}

impl Render for HomePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let hr = self.leaders_card(
            "hr",
            "Career Home Run Leaders",
            &self.hr_leaders,
            |l| {
                (
                    l.rank,
                    l.player_id.clone(),
                    l.player_name.clone().unwrap_or_else(|| l.player_id.clone()),
                    l.is_hall_of_famer,
                    format::int(l.home_runs),
                )
            },
            Route::BattingLeaders,
            cx,
        );
        let wins = self.leaders_card(
            "wins",
            "Career Wins Leaders",
            &self.win_leaders,
            |l| {
                (
                    l.rank,
                    l.player_id.clone(),
                    l.player_name.clone().unwrap_or_else(|| l.player_id.clone()),
                    l.is_hall_of_famer,
                    format::int(l.wins),
                )
            },
            Route::PitchingLeaders,
            cx,
        );
        widgets::page_body("home", &self.scroll)
            .child(
                widgets::page_column()
                    .child(self.render_hero(cx))
                    .child(self.render_counts(cx))
                    .children(self.render_this_day(cx))
                    .child(
                        h_flex()
                            .w_full()
                            .gap_5()
                            .flex_wrap()
                            .items_start()
                            .child(hr)
                            .child(wins),
                    )
                    .child(self.render_inductees(cx))
                    .child(self.render_quick_links(cx)),
            )
            .into_any_element()
    }
}

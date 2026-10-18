//! All-Star Games: every season's game count and selections, and each
//! season's rosters (the AL-NL game plus the Negro Leagues' showcases).

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::{AllStarGame, AllStarPlayer, AllStarRoster, AllStarSeason, AllStarYear};
use crate::format::{self, DASH};
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::grid::{Col, grid};
use crate::ui::widgets;

/// Seasons in which two AL-NL games were played.
const TWO_GAME_YEARS: std::ops::RangeInclusive<i32> = 1959..=1962;

pub struct AllStarPage {
    nav: Navigator,
    scroll: ScrollHandle,
    seasons: Loadable<Vec<AllStarSeason>>,
    _task: Option<Task<()>>,
}

impl AllStarPage {
    pub fn new(nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            seasons: Loadable::Idle,
            _task: None,
        };
        page.reload(window, cx);
        page
    }
}

impl Page for AllStarPage {
    fn title(&self, _: &App) -> SharedString {
        "All-Star Games".into()
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.seasons = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            |api| api.all_star_seasons(),
            |this: &mut Self, result, _| this.seasons = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for AllStarPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body: AnyElement = match &self.seasons {
            Loadable::Ready(rows) if rows.is_empty() => widgets::empty_state(
                "No All-Star Games Found",
                "No All-Star rosters are on record.",
                cx,
            ),
            Loadable::Ready(rows) => {
                let nav = self.nav.clone();
                let cols = vec![
                    Col::element("Year", 80., move |s: &AllStarSeason, _| {
                        nav.button(
                            SharedString::from(format!("as-{}", s.year)),
                            s.year.to_string(),
                            Route::AllStarYear { year: s.year },
                        )
                    }),
                    Col::num("Games", 72., |s: &AllStarSeason| format::int(s.game_count)),
                    Col::num("Selections", 96., |s: &AllStarSeason| {
                        format::int(s.selections)
                    }),
                    Col::element("", 240., |s: &AllStarSeason, _| {
                        h_flex()
                            .gap_1()
                            .when(s.has_east_west_game, |this| {
                                this.child(Tag::primary().xsmall().child("East-West Game"))
                            })
                            .when(TWO_GAME_YEARS.contains(&s.year), |this| {
                                this.child(Tag::secondary().xsmall().child("Two games"))
                            })
                    }),
                ];
                widgets::section_with(
                    "Seasons",
                    widgets::note(format!("{} seasons", rows.len()), cx),
                    cx,
                )
                .child(grid("all-star-seasons", cols, rows, cx))
                .into_any_element()
            }
            Loadable::Failed(message) => widgets::error_state(
                "all-star-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            _ => widgets::loading_state("Loading All-Star Games…", cx),
        };
        widgets::page_body("all-star", &self.scroll).child(
            widgets::page_column()
                .child(widgets::page_header(
                    Some("Stats"),
                    "All-Star Games",
                    Some(
                        "Every All-Star roster since 1933 — the AL-NL Midsummer Classic alongside the Negro Leagues' East-West Game and North-South Game. From 1959 through 1962 two AL-NL games were played each season; selections are counted by season, not by game."
                            .into(),
                    ),
                    vec![],
                    cx,
                ))
                .child(body),
        )
    }
}

pub struct AllStarYearPage {
    year: i32,
    nav: Navigator,
    scroll: ScrollHandle,
    data: Loadable<AllStarYear>,
    _task: Option<Task<()>>,
}

impl AllStarYearPage {
    pub fn new(year: i32, nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            year,
            nav,
            scroll: ScrollHandle::new(),
            data: Loadable::Idle,
            _task: None,
        };
        page.reload(window, cx);
        page
    }

    fn game_title(game: &AllStarGame, several: bool) -> String {
        let mut title = game.title.clone();
        if several
            && game.game_type == "AllStar"
            && let Some(n) = game.game_num
        {
            title.push_str(&format!(" — Game {n}"));
        }
        if let Some(date) = game.game_date.as_deref() {
            title.push_str(&format!(" — {}", format::date(Some(date))));
        }
        title
    }

    fn roster(&self, key: &str, roster: &AllStarRoster, cx: &App) -> AnyElement {
        let nav = self.nav.clone();
        let team_nav = self.nav.clone();
        let year = self.year;
        let (player_key, team_key) = (key.to_string(), key.to_string());
        let cols = vec![
            Col::element("Pos", 48., |p: &AllStarPlayer, _| {
                match p.starting_position.as_deref().filter(|_| p.is_starter) {
                    Some(pos) => div().font_bold().child(pos.to_string()),
                    None => div().child(DASH),
                }
            }),
            Col::element("Player", 220., move |p: &AllStarPlayer, ix| {
                widgets::player_link(
                    SharedString::from(format!("{player_key}-p-{ix}")),
                    &p.player_id,
                    p.full_name.clone(),
                    p.is_hall_of_famer,
                    &nav,
                )
            }),
            Col::element("Team", 200., move |p: &AllStarPlayer, ix| {
                let label = p
                    .team_name
                    .clone()
                    .or_else(|| p.team_id.clone())
                    .unwrap_or_else(|| DASH.into());
                match (p.team_id.as_deref(), p.team_lg_id.as_deref()) {
                    (Some(team_id), Some(lg_id)) => team_nav
                        .button(
                            SharedString::from(format!("{team_key}-t-{ix}")),
                            label,
                            Route::TeamSeason {
                                team_id: team_id.to_string(),
                                lg_id: lg_id.to_string(),
                                year,
                            },
                        )
                        .into_any_element(),
                    _ => label.into_any_element(),
                }
            }),
            Col::text("Played", 64., |p: &AllStarPlayer| {
                if p.played { "Yes" } else { DASH }.to_string()
            }),
        ];
        widgets::section_with(
            format!("{} Roster", roster.squad_name),
            Tag::secondary()
                .xsmall()
                .child(format!("{} players", roster.players.len())),
            cx,
        )
        .flex_1()
        .min_w(px(440.))
        .child(grid(
            SharedString::from(format!("{key}-grid")),
            cols,
            &roster.players,
            cx,
        ))
        .child(div().px_4().py_2().child(widgets::note(
            "Starters listed by position; reserves follow alphabetically.",
            cx,
        )))
        .into_any_element()
    }
}

impl Page for AllStarYearPage {
    fn title(&self, _: &App) -> SharedString {
        format!("{} All-Star Game", self.year).into()
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let year = self.year;
        self.data = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| api.all_star_year(year),
            |this: &mut Self, result, _| this.data = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for AllStarYearPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (previous, next) = match &self.data {
            Loadable::Ready(d) => (d.previous_year, d.next_year),
            _ => (None, None),
        };
        let header = h_flex()
            .w_full()
            .gap_3()
            .items_end()
            .justify_between()
            .flex_wrap()
            .child(widgets::page_header(
                Some("All-Star Games"),
                format!("{} All-Star Rosters", self.year),
                None,
                vec![],
                cx,
            ))
            .child(widgets::year_steps(
                "all-star-year",
                previous,
                next,
                |year| Route::AllStarYear { year },
                &self.nav,
            ));
        let body: AnyElement = match &self.data {
            Loadable::Ready(d) if d.games.is_empty() => widgets::empty_state(
                "No All-Star Games Found",
                "No All-Star rosters are on record for this season.",
                cx,
            ),
            Loadable::Ready(d) => {
                let several = d.games.iter().filter(|g| g.game_type == "AllStar").count() > 1;
                v_flex()
                    .gap_5()
                    .when(several, |this| {
                        this.child(Alert::info(
                            "two-games",
                            "Two All-Star Games were played each season from 1959 through 1962. Players selected for both games count a single All-Star season.",
                        ))
                    })
                    .children(d.games.iter().enumerate().map(|(gx, game)| {
                        v_flex()
                            .gap_3()
                            .child(
                                div()
                                    .text_lg()
                                    .font_bold()
                                    .child(Self::game_title(game, several)),
                            )
                            .when(game.game_type == "EastWest", |this| {
                                this.child(
                                    h_flex()
                                        .gap_1()
                                        .flex_wrap()
                                        .items_center()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(
                                            "The East-West Game was the Negro Leagues' showcase, drawing crowds at Comiskey Park that rivaled the major league game.",
                                        )
                                        .child(
                                            Button::new(SharedString::from(format!("ew-hub-{gx}")))
                                                .link()
                                                .small()
                                                .label("Explore the Negro Leagues.")
                                                .on_click(self.nav.to(Route::NegroLeagues)),
                                        ),
                                )
                            })
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_4()
                                    .flex_wrap()
                                    .items_start()
                                    .children(game.rosters.iter().enumerate().map(
                                        |(rx, roster)| {
                                            self.roster(&format!("g{gx}-r{rx}"), roster, cx)
                                        },
                                    )),
                            )
                    }))
                    .into_any_element()
            }
            Loadable::Failed(message) => {
                let message = if message.starts_with("No record") {
                    "No All-Star Game is on record for this season.".to_string()
                } else {
                    message.clone()
                };
                widgets::error_state(
                    "all-star-year-error",
                    &message,
                    cx.listener(|this, _, window, cx| this.reload(window, cx)),
                )
            }
            _ => widgets::loading_state("Loading rosters…", cx),
        };
        widgets::page_body("all-star-year", &self.scroll)
            .child(widgets::page_column().child(header).child(body))
    }
}

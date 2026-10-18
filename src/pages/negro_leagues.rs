//! Negro Leagues: the hub of the seven recognized leagues (1920–1948), each
//! league's seasons and clubs, and each season's standings and leaders.

use gpui_kit::assets::IconName;
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

use crate::api::models::{
    NegroLeague, NegroLeagueClub, NegroLeagueDetail, NegroLeagueLeader, NegroLeagueSeason,
    NegroLeagueSeasonSummary, NegroLeagueStanding,
};
use crate::format::{self, DASH};
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::{ReferencePage, Route};
use crate::ui::grid::{Col, grid};
use crate::ui::widgets;

const SEAMHEADS_URL: &str = "https://www.seamheads.com/NegroLgs/";

/// A run of muted text with inline links, like the web's notes.
enum Part {
    Text(&'static str),
    Link(&'static str, Route),
    External(&'static str, &'static str),
}

fn note_with_links(id: &str, parts: Vec<Part>, nav: &Navigator, cx: &App) -> AnyElement {
    h_flex()
        .gap_1()
        .flex_wrap()
        .items_center()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .children(parts.into_iter().enumerate().map(|(ix, part)| {
            let link_id = SharedString::from(format!("{id}-link-{ix}"));
            match part {
                Part::Text(text) => div().child(text).into_any_element(),
                Part::Link(label, route) => Button::new(link_id)
                    .link()
                    .xsmall()
                    .label(label)
                    .on_click(nav.to(route))
                    .into_any_element(),
                Part::External(label, url) => Button::new(link_id)
                    .link()
                    .xsmall()
                    .label(label)
                    .icon(IconName::ExternalLink)
                    .on_click(move |_, _, cx| cx.open_url(url))
                    .into_any_element(),
            }
        }))
        .into_any_element()
}

fn data_scope() -> Part {
    Part::Link(
        "About the data scope",
        Route::Reference(ReferencePage::About),
    )
}

fn surviving_record(label: &'static str) -> Part {
    Part::Link(label, Route::Reference(ReferencePage::SurvivingRecords))
}

fn team_season(team_id: &str, lg_id: &str, year: i32) -> Route {
    Route::TeamSeason {
        team_id: team_id.to_string(),
        lg_id: lg_id.to_string(),
        year,
    }
}

// MARK: Hub

pub struct NegroLeaguesPage {
    nav: Navigator,
    scroll: ScrollHandle,
    leagues: Loadable<Vec<NegroLeague>>,
    _task: Option<Task<()>>,
}

impl NegroLeaguesPage {
    pub fn new(nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            leagues: Loadable::Idle,
            _task: None,
        };
        page.reload(window, cx);
        page
    }

    fn card(&self, league: &NegroLeague, cx: &App) -> AnyElement {
        let stat = |label: &str, value: String| {
            v_flex()
                .child(div().font_bold().child(value))
                .child(widgets::note(label.to_string(), cx))
        };
        widgets::section_with(
            league.name.clone(),
            Tag::secondary().xsmall().child(league.lg_id.clone()),
            cx,
        )
        .w(px(390.))
        .flex_grow(1.)
        .child(
            v_flex()
                .flex_1()
                .p_4()
                .gap_3()
                .child(
                    h_flex()
                        .gap_6()
                        .child(stat(
                            "Years",
                            format::year_span(league.first_year, league.last_year),
                        ))
                        .child(stat("Clubs", format::int(league.clubs)))
                        .child(stat("Team-Seasons", format::int(league.team_seasons))),
                )
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(league.summary.clone().unwrap_or_default()),
                )
                .child(
                    h_flex().child(
                        Button::new(SharedString::from(format!("nl-{}", league.lg_id)))
                            .small()
                            .outline()
                            .label(format!("Explore the {}", league.lg_id))
                            .on_click(self.nav.to(Route::NegroLeague {
                                lg_id: league.lg_id.clone(),
                            })),
                    ),
                ),
        )
        .into_any_element()
    }
}

impl Page for NegroLeaguesPage {
    fn title(&self, _: &App) -> SharedString {
        "Negro Leagues".into()
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.leagues = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            |api| api.negro_leagues(),
            |this: &mut Self, result, _| this.leagues = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for NegroLeaguesPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let callout = v_flex()
            .gap_1()
            .child(Alert::info(
                "nl-callout",
                "The record here is what researchers reconstructed from newspaper box scores — treat every total as a floor, not a measure.",
            ))
            .child(note_with_links(
                "nl-callout",
                vec![
                    surviving_record("Read the story of the surviving record"),
                    Part::Text("or see"),
                    Part::Link("the data scope", Route::Reference(ReferencePage::About)),
                ],
                &self.nav,
                cx,
            ));
        let body: AnyElement = match &self.leagues {
            Loadable::Ready(leagues) if leagues.is_empty() => widgets::empty_state(
                "No Leagues Found",
                "No Negro League records are available.",
                cx,
            ),
            Loadable::Ready(leagues) => h_flex()
                .w_full()
                .gap_4()
                .flex_wrap()
                .items_stretch()
                .children(leagues.iter().map(|l| self.card(l, cx)))
                .into_any_element(),
            Loadable::Failed(message) => widgets::error_state(
                "nl-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            _ => widgets::loading_state("Loading the Negro Leagues…", cx),
        };
        let before = widgets::section("Before the leagues", cx).child(
            v_flex()
                .p_4()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Organized Black baseball long predates 1920 — independent clubs, eastern and western circuits, and barnstorming teams whose surviving records sit outside the recognized league seasons shown here."),
                )
                .child(note_with_links(
                    "nl-before",
                    vec![
                        surviving_record("The Surviving Record"),
                        Part::Text("tells that story, and the"),
                        Part::External("Seamheads Negro Leagues Database", SEAMHEADS_URL),
                        Part::Text("continues to reconstruct it."),
                    ],
                    &self.nav,
                    cx,
                )),
        );
        widgets::page_body("negro-leagues", &self.scroll).child(
            widgets::page_column()
                .child(widgets::page_header(
                    Some("History"),
                    "The Negro Leagues",
                    Some(
                        "Seven leagues, 1920–1948, recognized by Major League Baseball as major leagues. Browse their seasons, standings, and leaders."
                            .into(),
                    ),
                    vec![],
                    cx,
                ))
                .child(callout)
                .child(body)
                .child(before),
        )
    }
}

// MARK: League

pub struct NegroLeaguePage {
    lg_id: String,
    nav: Navigator,
    scroll: ScrollHandle,
    league: Loadable<NegroLeagueDetail>,
    _task: Option<Task<()>>,
}

impl NegroLeaguePage {
    pub fn new(lg_id: String, nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            lg_id,
            nav,
            scroll: ScrollHandle::new(),
            league: Loadable::Idle,
            _task: None,
        };
        page.reload(window, cx);
        page
    }

    fn render_league(&self, l: &NegroLeagueDetail, cx: &App) -> AnyElement {
        let team_seasons: i64 = l.seasons.iter().filter_map(|s| s.team_count).sum();
        let mut seasons = l.seasons.clone();
        seasons.sort_by_key(|s| std::cmp::Reverse(s.year));

        let (year_nav, champ_nav) = (self.nav.clone(), self.nav.clone());
        let (year_lg, champ_lg) = (l.lg_id.clone(), l.lg_id.clone());
        let season_cols = vec![
            Col::element("Year", 72., move |s: &NegroLeagueSeasonSummary, _| {
                year_nav.button(
                    SharedString::from(format!("nl-season-{}", s.year)),
                    s.year.to_string(),
                    Route::NegroLeagueSeason {
                        lg_id: year_lg.clone(),
                        year: s.year,
                    },
                )
            }),
            Col::num("Teams", 64., |s: &NegroLeagueSeasonSummary| {
                format::int(s.team_count)
            }),
            Col::element(
                "Pennant",
                240.,
                move |s: &NegroLeagueSeasonSummary, _| match s.champion_team_id.as_deref() {
                    Some(team_id) => champ_nav
                        .button(
                            SharedString::from(format!("nl-champ-{}", s.year)),
                            s.champion_name
                                .clone()
                                .unwrap_or_else(|| team_id.to_string()),
                            team_season(team_id, &champ_lg, s.year),
                        )
                        .into_any_element(),
                    None => DASH.into_any_element(),
                },
            ),
        ];

        let club_nav = self.nav.clone();
        let club_lg = l.lg_id.clone();
        let club_cols = vec![
            Col::element("Club", 230., move |c: &NegroLeagueClub, _| {
                let label = c.name.clone().unwrap_or_else(|| c.team_id.clone());
                match c.last_year {
                    Some(year) => club_nav
                        .button(
                            SharedString::from(format!("nl-club-{}", c.team_id)),
                            label,
                            team_season(&c.team_id, &club_lg, year),
                        )
                        .into_any_element(),
                    None => label.into_any_element(),
                }
            }),
            Col::text("Years", 96., |c: &NegroLeagueClub| {
                format::year_span(c.first_year, c.last_year)
            }),
            Col::num("Seasons", 64., |c: &NegroLeagueClub| format::int(c.seasons)),
            Col::num("Record", 84., |c: &NegroLeagueClub| {
                format::record(c.wins, c.losses)
            }),
            Col::num("PCT", 56., |c: &NegroLeagueClub| format::rate(c.win_pct)),
            Col::num("Pennants", 72., |c: &NegroLeagueClub| match c.pennants {
                Some(0) | None => DASH.into(),
                p => format::int(p),
            }),
        ];

        let season_label = if seasons.len() == 1 {
            "Season"
        } else {
            "Seasons"
        };
        v_flex()
            .gap_5()
            .child(widgets::page_header(
                Some("Negro Leagues"),
                l.name.clone(),
                l.summary.clone().map(Into::into),
                vec![Tag::secondary().small().child(l.lg_id.clone()).into_any_element()],
                cx,
            ))
            .child(widgets::stat_row(
                vec![
                    ("Years", format::year_span(l.first_year, l.last_year)),
                    (season_label, seasons.len().to_string()),
                    ("Clubs", l.clubs.len().to_string()),
                    ("Team-Seasons", team_seasons.to_string()),
                ],
                cx,
            ))
            .child(
                h_flex()
                    .w_full()
                    .gap_5()
                    .flex_wrap()
                    .items_start()
                    .child(
                        widgets::section("Seasons", cx)
                            .flex_1()
                            .min_w(px(380.))
                            .child(grid("nl-seasons", season_cols, &seasons, cx)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(px(540.))
                            .gap_2()
                            .child(widgets::section("Clubs", cx).child(grid(
                                "nl-clubs",
                                club_cols,
                                &l.clubs,
                                cx,
                            )))
                            .child(note_with_links(
                                "nl-clubs",
                                vec![
                                    Part::Text("Wins and losses reflect documented league games; surviving Negro Leagues records cover shorter and incompletely documented seasons."),
                                    data_scope(),
                                ],
                                &self.nav,
                                cx,
                            )),
                    ),
            )
            .into_any_element()
    }
}

impl Page for NegroLeaguePage {
    fn title(&self, _: &App) -> SharedString {
        match &self.league {
            Loadable::Ready(l) => l.name.clone().into(),
            _ => self.lg_id.clone().into(),
        }
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let lg_id = self.lg_id.clone();
        self.league = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| api.negro_league(&lg_id),
            |this: &mut Self, result, _| this.league = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for NegroLeaguePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content: AnyElement = match &self.league {
            Loadable::Ready(l) => self.render_league(l, cx),
            Loadable::Failed(message) => {
                let message = if message.starts_with("No record") {
                    "League Not Found".to_string()
                } else {
                    message.clone()
                };
                widgets::error_state(
                    "nl-league-error",
                    &message,
                    cx.listener(|this, _, window, cx| this.reload(window, cx)),
                )
            }
            _ => widgets::loading_state("Loading league…", cx),
        };
        widgets::page_body("negro-league", &self.scroll)
            .child(widgets::page_column().child(content))
    }
}

// MARK: Season

pub struct NegroLeagueSeasonPage {
    lg_id: String,
    year: i32,
    nav: Navigator,
    scroll: ScrollHandle,
    season: Loadable<NegroLeagueSeason>,
    _task: Option<Task<()>>,
}

/// Games behind: blank for the leader, whole numbers without a decimal.
fn games_behind(gb: Option<f64>) -> String {
    match gb {
        Some(v) if v > 0. && v.fract() == 0. => format!("{v:.0}"),
        Some(v) if v > 0. => format!("{v:.1}"),
        _ => DASH.into(),
    }
}

impl NegroLeagueSeasonPage {
    pub fn new(
        lg_id: String,
        year: i32,
        nav: Navigator,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut page = Self {
            lg_id,
            year,
            nav,
            scroll: ScrollHandle::new(),
            season: Loadable::Idle,
            _task: None,
        };
        page.reload(window, cx);
        page
    }

    fn leaders_card(
        &self,
        key: &str,
        title: &str,
        subtitle: &str,
        rows: &[NegroLeagueLeader],
        value: fn(Option<f64>) -> String,
        cx: &App,
    ) -> AnyElement {
        widgets::section_with(
            title.to_string(),
            widgets::note(subtitle.to_string(), cx),
            cx,
        )
        .w(px(420.))
        .flex_grow(1.)
        .child(
            v_flex()
                .px_3()
                .py_2()
                .gap_1()
                .when(rows.is_empty(), |this| {
                    this.child(widgets::note("No surviving records.", cx))
                })
                .children(rows.iter().enumerate().map(|(ix, r)| {
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(widgets::note(
                            format!("{}.", r.rank.map(|n| n.to_string()).unwrap_or_default()),
                            cx,
                        ))
                        .child(div().flex_1().min_w_0().child(widgets::player_link(
                            SharedString::from(format!("{key}-{ix}")),
                            &r.player_id,
                            r.player_name.clone(),
                            r.is_hall_of_famer,
                            &self.nav,
                        )))
                        .child(
                            div()
                                .font_bold()
                                .font_family(cx.theme().mono_font_family.clone())
                                .child(value(r.value)),
                        )
                })),
        )
        .into_any_element()
    }

    fn render_season(&self, s: &NegroLeagueSeason, cx: &App) -> AnyElement {
        let nav = self.nav.clone();
        let (lg_id, year) = (s.lg_id.clone(), s.year);
        let cols = vec![
            Col::num("#", 40., |t: &NegroLeagueStanding| format::int(t.rank)),
            Col::element("Team", 280., move |t: &NegroLeagueStanding, _| {
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(nav.button(
                        SharedString::from(format!("nl-st-{}", t.team_id)),
                        t.name.clone().unwrap_or_else(|| t.team_id.clone()),
                        team_season(&t.team_id, &lg_id, year),
                    ))
                    .when(t.won_pennant, |this| {
                        this.child(Tag::primary().xsmall().child("Pennant"))
                    })
            }),
            Col::num("W", 56., |t: &NegroLeagueStanding| format::int(t.wins)),
            Col::num("L", 56., |t: &NegroLeagueStanding| format::int(t.losses)),
            Col::num("PCT", 64., |t: &NegroLeagueStanding| {
                format::rate(t.win_pct)
            }),
            Col::num("GB", 56., |t: &NegroLeagueStanding| {
                games_behind(t.games_behind)
            }),
            Col::num("G", 56., |t: &NegroLeagueStanding| format::int(t.games)),
        ];
        let int = |v: Option<f64>| format::int(v.map(|v| v.round() as i64));
        v_flex()
            .gap_5()
            .child(
                v_flex()
                    .gap_2()
                    .child(widgets::section("Standings", cx).child(grid(
                        "nl-standings",
                        cols,
                        &s.standings,
                        cx,
                    )))
                    .child(note_with_links(
                        "nl-standings",
                        vec![
                            Part::Text("Standings reflect documented league games; game counts vary by how much of each season survives."),
                            data_scope(),
                            Part::Text("·"),
                            surviving_record("The story of the surviving record"),
                        ],
                        &self.nav,
                        cx,
                    )),
            )
            .child(
                h_flex()
                    .w_full()
                    .gap_4()
                    .flex_wrap()
                    .items_start()
                    .child(self.leaders_card(
                        "nl-avg",
                        "Batting Average",
                        "Qualified leaders",
                        &s.leaders.batting_average,
                        format::rate,
                        cx,
                    ))
                    .child(self.leaders_card(
                        "nl-hr",
                        "Home Runs",
                        "All players",
                        &s.leaders.home_runs,
                        int,
                        cx,
                    ))
                    .child(self.leaders_card(
                        "nl-era",
                        "ERA",
                        "Qualified leaders",
                        &s.leaders.era,
                        format::decimal2,
                        cx,
                    ))
                    .child(self.leaders_card(
                        "nl-w",
                        "Wins",
                        "All pitchers",
                        &s.leaders.wins,
                        int,
                        cx,
                    )),
            )
            .child(note_with_links(
                "nl-scope",
                vec![
                    Part::Text("Raw totals, not context-adjusted — no park factors or era adjustments, and surviving Negro Leagues records cover shorter seasons."),
                    data_scope(),
                ],
                &self.nav,
                cx,
            ))
            .into_any_element()
    }
}

impl Page for NegroLeagueSeasonPage {
    fn title(&self, _: &App) -> SharedString {
        match &self.season {
            Loadable::Ready(s) => format!("{} {}", s.year, s.league_name).into(),
            _ => format!("{} {}", self.year, self.lg_id).into(),
        }
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let (lg_id, year) = (self.lg_id.clone(), self.year);
        self.season = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| api.negro_league_season(&lg_id, year),
            |this: &mut Self, result, _| this.season = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for NegroLeagueSeasonPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (previous, next, league_name) = match &self.season {
            Loadable::Ready(s) => (s.previous_year, s.next_year, s.league_name.clone()),
            _ => (None, None, self.lg_id.clone()),
        };
        let lg_id = self.lg_id.clone();
        let header = h_flex()
            .w_full()
            .gap_3()
            .items_end()
            .justify_between()
            .flex_wrap()
            .child(
                v_flex()
                    .gap_1()
                    .items_start()
                    .child(
                        Button::new("nl-season-league")
                            .link()
                            .small()
                            .label(league_name.clone())
                            .on_click(self.nav.to(Route::NegroLeague {
                                lg_id: self.lg_id.clone(),
                            })),
                    )
                    .child(widgets::page_header(
                        None,
                        format!("{} {}", self.year, league_name),
                        None,
                        vec![],
                        cx,
                    )),
            )
            .child(widgets::year_steps(
                "nl-season",
                previous,
                next,
                move |year| Route::NegroLeagueSeason {
                    lg_id: lg_id.clone(),
                    year,
                },
                &self.nav,
            ));
        let content: AnyElement = match &self.season {
            Loadable::Ready(s) => self.render_season(s, cx),
            Loadable::Failed(message) => {
                let message = if message.starts_with("No record") {
                    "Season Not Found".to_string()
                } else {
                    message.clone()
                };
                widgets::error_state(
                    "nl-season-error",
                    &message,
                    cx.listener(|this, _, window, cx| this.reload(window, cx)),
                )
            }
            _ => widgets::loading_state("Loading season…", cx),
        };
        widgets::page_body("negro-league-season", &self.scroll)
            .child(widgets::page_column().child(header).child(content))
    }
}

#[cfg(test)]
mod tests {
    use super::games_behind;

    #[test]
    fn games_behind_matches_web() {
        assert_eq!(games_behind(None), "—");
        assert_eq!(games_behind(Some(0.)), "—");
        assert_eq!(games_behind(Some(-1.5)), "—");
        assert_eq!(games_behind(Some(3.)), "3");
        assert_eq!(games_behind(Some(2.5)), "2.5");
    }
}

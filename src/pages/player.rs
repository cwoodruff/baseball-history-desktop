//! Player detail: biography, teams, honors, career totals and stat tabs.

use std::collections::BTreeMap;

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::*;
use crate::format::{self, DASH};
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::grid::{Col, grid};
use crate::ui::widgets;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatTab {
    Batting,
    Pitching,
    Fielding,
    Postseason,
}

impl StatTab {
    fn label(self) -> &'static str {
        match self {
            StatTab::Batting => "Batting",
            StatTab::Pitching => "Pitching",
            StatTab::Fielding => "Fielding",
            StatTab::Postseason => "Postseason",
        }
    }
}

pub struct PlayerPage {
    id: String,
    nav: Navigator,
    scroll: ScrollHandle,
    player: Loadable<PlayerDetail>,
    batting: Loadable<Vec<BattingSeason>>,
    pitching: Loadable<Vec<PitchingSeason>>,
    fielding: Loadable<Vec<FieldingSeason>>,
    awards: Loadable<Vec<PlayerAward>>,
    post_batting: Loadable<Vec<PostseasonBatting>>,
    post_pitching: Loadable<Vec<PostseasonPitching>>,
    tab: Option<StatTab>,
    show_all_awards: bool,
    _tasks: Vec<Task<()>>,
}

/// Lahman innings (200.2 = 200⅔) as a true number of innings.
fn true_innings(ip: Option<f64>) -> Option<f64> {
    let ip = ip?;
    let whole = ip.trunc();
    let outs = ((ip - whole) * 10.).round();
    Some(whole + outs / 3.)
}

fn per_nine(count: Option<i64>, ip: Option<f64>) -> Option<f64> {
    let innings = true_innings(ip)?;
    (innings > 0.).then(|| count.unwrap_or(0) as f64 * 9. / innings)
}

fn whip(walks: Option<i64>, hits: Option<i64>, ip: Option<f64>) -> Option<f64> {
    let innings = true_innings(ip)?;
    (innings > 0.).then(|| (walks.unwrap_or(0) + hits.unwrap_or(0)) as f64 / innings)
}

fn fielding_pct(po: Option<i64>, a: Option<i64>, e: Option<i64>) -> Option<f64> {
    let (po, a, e) = (po.unwrap_or(0), a.unwrap_or(0), e.unwrap_or(0));
    let chances = po + a + e;
    (chances > 0).then(|| (po + a) as f64 / chances as f64)
}

#[derive(Clone, Default)]
struct PositionTotals {
    position: String,
    games: i64,
    put_outs: i64,
    assists: i64,
    errors: i64,
    double_plays: i64,
}

fn career_by_position(seasons: &[FieldingSeason]) -> Vec<PositionTotals> {
    let mut map: BTreeMap<String, PositionTotals> = BTreeMap::new();
    for s in seasons {
        let pos = s.position.clone().unwrap_or_else(|| DASH.into());
        let t = map.entry(pos.clone()).or_insert_with(|| PositionTotals {
            position: pos,
            ..Default::default()
        });
        t.games += s.games.unwrap_or(0);
        t.put_outs += s.put_outs.unwrap_or(0);
        t.assists += s.assists.unwrap_or(0);
        t.errors += s.errors.unwrap_or(0);
        t.double_plays += s.double_plays.unwrap_or(0);
    }
    let mut rows: Vec<_> = map.into_values().collect();
    rows.sort_by_key(|t| std::cmp::Reverse(t.games));
    rows
}

impl PlayerPage {
    pub fn new(id: String, nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            id,
            nav,
            scroll: ScrollHandle::new(),
            player: Loadable::Idle,
            batting: Loadable::Idle,
            pitching: Loadable::Idle,
            fielding: Loadable::Idle,
            awards: Loadable::Idle,
            post_batting: Loadable::Idle,
            post_pitching: Loadable::Idle,
            tab: None,
            show_all_awards: false,
            _tasks: Vec::new(),
        };
        page.reload(window, cx);
        page
    }

    fn name(&self) -> String {
        match &self.player {
            Loadable::Ready(p) => format::full_name(p.first_name.as_deref(), p.last_name.as_deref()),
            _ => self.id.clone(),
        }
    }

    /// Tabs with data, pitchers' Pitching tab first, as on the web.
    fn tabs(&self) -> Vec<StatTab> {
        let has = |l: Option<usize>| l.is_some_and(|n| n > 0);
        let batting = has(self.batting.ready().map(Vec::len));
        let pitching = has(self.pitching.ready().map(Vec::len));
        let fielding = has(self.fielding.ready().map(Vec::len));
        let post = has(self.post_batting.ready().map(Vec::len)) || has(self.post_pitching.ready().map(Vec::len));
        let is_pitcher = self.player.ready().is_some_and(|p| {
            let pg = p.career_pitching.as_ref().and_then(|c| c.games).unwrap_or(0);
            let bg = p.career_batting.as_ref().and_then(|c| c.games).unwrap_or(0);
            pg > 0 && pg * 2 >= bg
        });
        let mut tabs = Vec::new();
        if is_pitcher && pitching {
            tabs.push(StatTab::Pitching);
        }
        if batting {
            tabs.push(StatTab::Batting);
        }
        if !is_pitcher && pitching {
            tabs.push(StatTab::Pitching);
        }
        if fielding {
            tabs.push(StatTab::Fielding);
        }
        if post {
            tabs.push(StatTab::Postseason);
        }
        tabs
    }

    fn team_route(team: Option<&str>, lg: Option<&str>, year: i32) -> Option<Route> {
        Some(Route::TeamSeason {
            team_id: team?.to_string(),
            lg_id: lg?.to_string(),
            year,
        })
    }

    fn team_cell(&self, id: String, team: Option<&str>, label: Option<&str>, lg: Option<&str>, year: i32) -> AnyElement {
        let label = format::text(label.or(team));
        match Self::team_route(team, lg, year) {
            Some(route) => self.nav.button(SharedString::from(id), label, route).into_any_element(),
            None => label.into_any_element(),
        }
    }

    fn render_info(&self, p: &PlayerDetail, cx: &App) -> AnyElement {
        let born_place = [p.birth_city.as_deref(), p.birth_state.as_deref(), p.birth_country.as_deref()]
            .iter()
            .flatten()
            .filter(|s| !s.trim().is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        let born = match (p.birth_date.as_deref(), born_place.is_empty()) {
            (Some(d), false) => format!("{} in {born_place}", format::date(Some(d))),
            (Some(d), true) => format::date(Some(d)),
            (None, false) => born_place,
            (None, true) => DASH.into(),
        };
        let size = match (p.height.as_deref(), p.weight.as_deref()) {
            (h, Some(w)) if !w.trim().is_empty() => format!("{} / {w} lbs", format::height(h)),
            (h, _) => format::height(h),
        };
        let career = match (p.debut_year, p.final_year) {
            (Some(d), Some(f)) => format!("{} years", f - d + 1),
            _ => DASH.into(),
        };
        let mut rows = vec![
            ("Full Name", format::text(p.given_name.as_deref().map(|g| format!("{g} {}", p.last_name.clone().unwrap_or_default())).as_deref())),
            ("Born", born),
        ];
        if p.death_date.as_deref().is_some_and(|d| !d.trim().is_empty()) {
            rows.push(("Died", format::date(p.death_date.as_deref())));
        }
        rows.extend([
            ("Size", size),
            ("B/T", format!("{} / {}", format::bats_throws(p.bats.as_deref()), format::bats_throws(p.throws.as_deref()))),
            ("Debut", format::year(p.debut_year)),
            ("Final Game", format::year(p.final_year)),
            ("Career", career),
        ]);
        widgets::section("Player Info", cx).child(widgets::facts(rows, cx)).into_any_element()
    }

    fn render_teams(&self, p: &PlayerDetail, cx: &App) -> AnyElement {
        widgets::section("Teams", cx)
            .child(
                v_flex().px_4().py_2().gap_1().children(p.teams.iter().map(|t| {
                    h_flex()
                        .gap_2()
                        .justify_between()
                        .text_sm()
                        .child(div().min_w_0().truncate().child(format::text(t.team_name.as_deref().or(Some(&t.team_id)))))
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_color(cx.theme().muted_foreground)
                                .child(format::year_span(t.first_year, t.last_year)),
                        )
                })),
            )
            .when(p.teams.is_empty(), |this| this.child(div().p_4().child(widgets::note("No team history.", cx))))
            .into_any_element()
    }

    fn render_honors(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(awards) = self.awards.ready() else {
            return widgets::section("Honors", cx)
                .child(widgets::loading_state("Loading awards…", cx))
                .into_any_element();
        };
        if awards.is_empty() {
            return widgets::section("Honors", cx)
                .child(div().px_4().py_3().child(widgets::note("No awards on record.", cx)))
                .into_any_element();
        }
        let mut sorted = awards.clone();
        sorted.sort_by(|a, b| a.year.cmp(&b.year).then(a.award_id.cmp(&b.award_id)));
        let shown = if self.show_all_awards { sorted.len() } else { sorted.len().min(10) };
        widgets::section_with(
            "Honors",
            widgets::note(format!("{} awards", sorted.len()), cx),
            cx,
        )
        .child(
            v_flex().px_4().py_2().gap_1().children(sorted.iter().take(shown).map(|a| {
                h_flex()
                    .gap_2()
                    .text_sm()
                    .child(div().w(px(40.)).flex_shrink_0().font_bold().child(a.year.to_string()))
                    .child(div().flex_1().min_w_0().child(match a.lg_id.as_deref() {
                        Some(lg) if !lg.is_empty() => format!("{} ({lg})", a.award_id),
                        _ => a.award_id.clone(),
                    }))
            })),
        )
        .when(sorted.len() > 10, |this| {
            this.child(
                h_flex().px_3().pb_2().child(
                    Button::new("toggle-awards")
                        .ghost()
                        .xsmall()
                        .label(if self.show_all_awards {
                            "Show fewer".to_string()
                        } else {
                            format!("Show all {}", sorted.len())
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_all_awards = !this.show_all_awards;
                            cx.notify();
                        })),
                ),
            )
        })
        .into_any_element()
    }

    fn render_career(&self, p: &PlayerDetail, cx: &App) -> AnyElement {
        let mut col = v_flex().gap_4().w_full();
        if let Some(b) = p.career_batting.as_ref().filter(|b| b.at_bats.unwrap_or(0) > 0) {
            col = col.child(widgets::section("Career Batting", cx).child(
                div().p_3().child(widgets::stat_row(
                    vec![
                        ("G", format::int(b.games)),
                        ("AVG", format::rate(b.batting_average)),
                        ("H", format::int(b.hits)),
                        ("HR", format::int(b.home_runs)),
                        ("RBI", format::int(b.rbi)),
                        ("OPS", format::rate(b.ops)),
                    ],
                    cx,
                )),
            ));
        }
        if let Some(pi) = p.career_pitching.as_ref().filter(|c| c.innings_pitched.unwrap_or(0.) > 0.) {
            col = col.child(widgets::section("Career Pitching", cx).child(
                div().p_3().child(widgets::stat_row(
                    vec![
                        ("W-L", format::record(pi.wins, pi.losses)),
                        ("ERA", format::decimal2(pi.era)),
                        ("G", format::int(pi.games)),
                        ("SO", format::int(pi.strikeouts)),
                        ("SV", format::int(pi.saves)),
                        ("WHIP", format::decimal2(pi.whip)),
                    ],
                    cx,
                )),
            ));
        }
        col.into_any_element()
    }

    fn render_batting(&self, cx: &App) -> AnyElement {
        let mut rows = self.batting.ready().cloned().unwrap_or_default();
        rows.sort_by_key(|s| s.year);
        let columns = vec![
            Col::text("Year", 64., |s: &BattingSeason| s.year.to_string()),
            Col::element("Team", 200., |s: &BattingSeason, ix| {
                self.team_cell(format!("bat-team-{}-{ix}", s.year), s.team_id.as_deref(), s.team_name.as_deref(), s.lg_id.as_deref(), s.year)
            }),
            Col::num("G", 56., |s: &BattingSeason| format::int(s.games)),
            Col::num("AB", 64., |s: &BattingSeason| format::int(s.at_bats)),
            Col::num("H", 56., |s: &BattingSeason| format::int(s.hits)),
            Col::num("2B", 56., |s: &BattingSeason| format::int(s.doubles)),
            Col::num("3B", 56., |s: &BattingSeason| format::int(s.triples)),
            Col::num("HR", 56., |s: &BattingSeason| format::int(s.home_runs)),
            Col::num("R", 56., |s: &BattingSeason| format::int(s.runs)),
            Col::num("AVG", 64., |s: &BattingSeason| format::rate(s.batting_average)),
        ];
        widgets::section("Batting Season by Season", cx)
            .child(grid("batting-seasons", columns, &rows, cx))
            .into_any_element()
    }

    fn render_pitching(&self, cx: &App) -> AnyElement {
        let mut rows = self.pitching.ready().cloned().unwrap_or_default();
        rows.sort_by_key(|s| s.year);
        let seasons = vec![
            Col::text("Year", 64., |s: &PitchingSeason| s.year.to_string()),
            Col::element("Team", 200., |s: &PitchingSeason, ix| {
                self.team_cell(format!("pit-team-{}-{ix}", s.year), s.team_id.as_deref(), s.team_name.as_deref(), s.lg_id.as_deref(), s.year)
            }),
            Col::num("W", 48., |s: &PitchingSeason| format::int(s.wins)),
            Col::num("L", 48., |s: &PitchingSeason| format::int(s.losses)),
            Col::num("ERA", 64., |s: &PitchingSeason| format::decimal2(s.era)),
            Col::num("G", 48., |s: &PitchingSeason| format::int(s.games)),
            Col::num("GS", 48., |s: &PitchingSeason| format::int(s.games_started)),
            Col::num("SV", 48., |s: &PitchingSeason| format::int(s.saves)),
            Col::num("IP", 72., |s: &PitchingSeason| format::innings(s.innings_pitched)),
            Col::num("SO", 56., |s: &PitchingSeason| format::int(s.strikeouts)),
            Col::num("BB", 56., |s: &PitchingSeason| format::int(s.walks)),
        ];
        let advanced = vec![
            Col::text("Year", 64., |s: &PitchingSeason| s.year.to_string()),
            Col::text("Lg", 56., |s: &PitchingSeason| format::text(s.lg_id.as_deref())),
            Col::num("IP", 72., |s: &PitchingSeason| format::innings(s.innings_pitched)),
            Col::num("K/9", 64., |s: &PitchingSeason| format::decimal2(per_nine(s.strikeouts, s.innings_pitched))),
            Col::num("BB/9", 64., |s: &PitchingSeason| format::decimal2(per_nine(s.walks, s.innings_pitched))),
            Col::num("WHIP", 72., |s: &PitchingSeason| {
                whip(s.walks, s.hits, s.innings_pitched)
                    .map(|v| format!("{v:.3}"))
                    .unwrap_or_else(|| DASH.into())
            }),
        ];
        v_flex()
            .gap_4()
            .child(widgets::section("Pitching Season by Season", cx).child(grid("pitching-seasons", seasons, &rows, cx)))
            .child(
                widgets::section("Advanced Pitching", cx)
                    .child(grid("pitching-advanced", advanced, &rows, cx))
                    .child(div().px_4().py_2().child(widgets::note(
                        "K/9 and BB/9: strikeouts and walks per nine innings. WHIP: walks plus hits per inning pitched.",
                        cx,
                    ))),
            )
            .into_any_element()
    }

    fn render_fielding(&self, cx: &App) -> AnyElement {
        let mut rows = self.fielding.ready().cloned().unwrap_or_default();
        rows.sort_by_key(|s| s.year);
        let totals = career_by_position(&rows);
        let career = vec![
            Col::text("Pos", 64., |t: &PositionTotals| t.position.clone()),
            Col::num("G", 64., |t: &PositionTotals| format::int(Some(t.games))),
            Col::num("PO", 64., |t: &PositionTotals| format::int(Some(t.put_outs))),
            Col::num("A", 64., |t: &PositionTotals| format::int(Some(t.assists))),
            Col::num("E", 56., |t: &PositionTotals| format::int(Some(t.errors))),
            Col::num("DP", 56., |t: &PositionTotals| format::int(Some(t.double_plays))),
            Col::num("FPct", 64., |t: &PositionTotals| {
                format::rate(fielding_pct(Some(t.put_outs), Some(t.assists), Some(t.errors)))
            }),
        ];
        let seasons = vec![
            Col::text("Year", 64., |s: &FieldingSeason| s.year.to_string()),
            Col::element("Team", 96., |s: &FieldingSeason, ix| {
                self.team_cell(format!("fld-team-{}-{ix}", s.year), s.team_id.as_deref(), None, s.lg_id.as_deref(), s.year)
            }),
            Col::text("Pos", 56., |s: &FieldingSeason| format::text(s.position.as_deref())),
            Col::num("G", 56., |s: &FieldingSeason| format::int(s.games)),
            Col::num("PO", 64., |s: &FieldingSeason| format::int(s.put_outs)),
            Col::num("A", 56., |s: &FieldingSeason| format::int(s.assists)),
            Col::num("E", 56., |s: &FieldingSeason| format::int(s.errors)),
            Col::num("DP", 56., |s: &FieldingSeason| format::int(s.double_plays)),
            Col::num("FPct", 64., |s: &FieldingSeason| format::rate(fielding_pct(s.put_outs, s.assists, s.errors))),
        ];
        v_flex()
            .gap_4()
            .child(widgets::section("Career Fielding by Position", cx).child(grid("fielding-career", career, &totals, cx)))
            .child(widgets::section("Fielding Season by Season", cx).child(grid("fielding-seasons", seasons, &rows, cx)))
            .into_any_element()
    }

    fn render_postseason(&self, cx: &App) -> AnyElement {
        let mut batting = self.post_batting.ready().cloned().unwrap_or_default();
        batting.sort_by_key(|s| s.year);
        let mut pitching = self.post_pitching.ready().cloned().unwrap_or_default();
        pitching.sort_by_key(|s| s.year);
        let year_cell = |prefix: &'static str| {
            move |year: i32, ix: usize| {
                self.nav
                    .button(SharedString::from(format!("{prefix}-{year}-{ix}")), year.to_string(), Route::Postseason { year: Some(year) })
                    .into_any_element()
            }
        };
        let bat_year = year_cell("psb-year");
        let pit_year = year_cell("psp-year");
        let bat_cols = vec![
            Col::element("Year", 72., move |s: &PostseasonBatting, ix| bat_year(s.year, ix)),
            Col::text("Round", 72., |s: &PostseasonBatting| format::text(s.round.as_deref())),
            Col::element("Team", 80., |s: &PostseasonBatting, ix| {
                self.team_cell(format!("psb-team-{}-{ix}", s.year), s.team_id.as_deref(), None, s.lg_id.as_deref(), s.year)
            }),
            Col::num("G", 48., |s: &PostseasonBatting| format::int(s.games)),
            Col::num("AB", 56., |s: &PostseasonBatting| format::int(s.at_bats)),
            Col::num("H", 48., |s: &PostseasonBatting| format::int(s.hits)),
            Col::num("HR", 48., |s: &PostseasonBatting| format::int(s.home_runs)),
            Col::num("RBI", 56., |s: &PostseasonBatting| format::int(s.rbi)),
            Col::num("AVG", 64., |s: &PostseasonBatting| format::rate(s.batting_average)),
        ];
        let pit_cols = vec![
            Col::element("Year", 72., move |s: &PostseasonPitching, ix| pit_year(s.year, ix)),
            Col::text("Round", 72., |s: &PostseasonPitching| format::text(s.round.as_deref())),
            Col::element("Team", 80., |s: &PostseasonPitching, ix| {
                self.team_cell(format!("psp-team-{}-{ix}", s.year), s.team_id.as_deref(), None, s.lg_id.as_deref(), s.year)
            }),
            Col::num("G", 48., |s: &PostseasonPitching| format::int(s.games)),
            Col::num("W-L", 64., |s: &PostseasonPitching| format::record(s.wins, s.losses)),
            Col::num("SV", 48., |s: &PostseasonPitching| format::int(s.saves)),
            Col::num("IP", 64., |s: &PostseasonPitching| format::innings(s.innings_pitched)),
            Col::num("SO", 48., |s: &PostseasonPitching| format::int(s.strikeouts)),
            Col::num("ERA", 64., |s: &PostseasonPitching| format::decimal2(s.era)),
        ];
        v_flex()
            .gap_4()
            .when(!batting.is_empty(), |this| {
                this.child(widgets::section("Postseason Batting", cx).child(grid("post-batting", bat_cols, &batting, cx)))
            })
            .when(!pitching.is_empty(), |this| {
                this.child(widgets::section("Postseason Pitching", cx).child(grid("post-pitching", pit_cols, &pitching, cx)))
            })
            .into_any_element()
    }

    fn render_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        let still_loading = [
            self.batting.is_loading(),
            self.pitching.is_loading(),
            self.fielding.is_loading(),
            self.post_batting.is_loading(),
            self.post_pitching.is_loading(),
        ]
        .contains(&true);
        let tabs = self.tabs();
        if tabs.is_empty() {
            return if still_loading {
                widgets::loading_state("Loading statistics…", cx)
            } else {
                widgets::empty_state("No Statistics Available", "No season statistics are on record for this player.", cx)
            };
        }
        let active = self.tab.filter(|t| tabs.contains(t)).unwrap_or(tabs[0]);
        let active_ix = tabs.iter().position(|t| *t == active).unwrap_or(0);
        let content = match active {
            StatTab::Batting => self.render_batting(cx),
            StatTab::Pitching => self.render_pitching(cx),
            StatTab::Fielding => self.render_fielding(cx),
            StatTab::Postseason => self.render_postseason(cx),
        };
        v_flex()
            .gap_3()
            .when(tabs.len() > 1, |this| {
                let tab_list = tabs.clone();
                this.child(
                    TabBar::new("player-tabs")
                        .underline()
                        .selected_index(active_ix)
                        .children(tabs.iter().map(|t| Tab::new().label(t.label())))
                        .on_click(cx.listener(move |this, ix: &usize, _, cx| {
                            this.tab = tab_list.get(*ix).copied();
                            cx.notify();
                        })),
                )
            })
            .child(content)
            .into_any_element()
    }
}

impl Page for PlayerPage {
    fn title(&self, _: &App) -> SharedString {
        self.name().into()
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let id = self.id.clone();
        self.player = Loadable::Loading;
        self.batting = Loadable::Loading;
        self.pitching = Loadable::Loading;
        self.fielding = Loadable::Loading;
        self.awards = Loadable::Loading;
        self.post_batting = Loadable::Loading;
        self.post_pitching = Loadable::Loading;
        // Seven independent requests, issued in parallel.
        macro_rules! load {
            ($field:ident, $call:ident) => {{
                let id = id.clone();
                loader::fetch(
                    cx,
                    move |api| api.$call(&id),
                    |this: &mut Self, result, _| this.$field = Loadable::from_result(result),
                )
            }};
        }
        self._tasks = vec![
            load!(player, player),
            load!(batting, player_batting),
            load!(pitching, player_pitching),
            load!(fielding, player_fielding),
            load!(awards, player_awards),
            load!(post_batting, player_postseason_batting),
            load!(post_pitching, player_postseason_pitching),
        ];
        cx.notify();
    }
}

impl Render for PlayerPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content: AnyElement = match &self.player {
            Loadable::Ready(p) => {
                let mut badges: Vec<AnyElement> = Vec::new();
                if p.is_partial_record {
                    badges.push(widgets::partial_record_chip("partial-record", &self.nav));
                }
                if p.is_hall_of_famer {
                    let year = p.hof_induction_year;
                    badges.push(
                        Button::new("hof-year")
                            .ghost()
                            .compact()
                            .xsmall()
                            .tooltip("Hall of Fame class")
                            .child(widgets::hof_year_tag(year))
                            .on_click(self.nav.to(Route::HallOfFame { year, category: None }))
                            .into_any_element(),
                    );
                }
                let compare = Button::new("compare-player")
                    .small()
                    .outline()
                    .label("Compare")
                    .on_click(self.nav.to(Route::Compare { first: Some(p.player_id.clone()), second: None }));
                let header = h_flex()
                    .w_full()
                    .justify_between()
                    .items_end()
                    .gap_3()
                    .child(widgets::page_header(Some("Player"), self.name(), Some(format::year_span(p.debut_year, p.final_year).into()), badges, cx))
                    .child(compare);
                let left = v_flex()
                    .w(px(340.))
                    .flex_shrink_0()
                    .gap_4()
                    .child(self.render_info(p, cx))
                    .child(self.render_teams(p, cx))
                    .child(self.render_honors(cx));
                let right = v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_4()
                    .child(self.render_career(p, cx))
                    .child(self.render_tabs(cx));
                v_flex()
                    .gap_5()
                    .child(header)
                    .child(h_flex().w_full().gap_5().items_start().child(left).child(right))
                    .into_any_element()
            }
            Loadable::Failed(message) => {
                let message = message.clone();
                v_flex()
                    .child(widgets::page_header(Some("Player"), self.id.clone(), None, vec![], cx))
                    .child(widgets::error_state("player-error", &message, cx.listener(|this, _, window, cx| this.reload(window, cx))))
                    .into_any_element()
            }
            _ => widgets::loading_state("Loading player…", cx),
        };
        widgets::page_body("player", &self.scroll).child(widgets::page_column().child(content))
    }
}

#[cfg(test)]
mod tests {
    use super::{fielding_pct, per_nine, true_innings, whip};

    #[test]
    fn innings_notation_converts_thirds() {
        assert_eq!(true_innings(Some(200.2)).map(|v| (v * 3.).round()), Some(602.));
        assert_eq!(per_nine(Some(9), Some(9.0)), Some(9.0));
        assert_eq!(whip(Some(3), Some(12), Some(9.0)).map(|v| (v * 1000.).round()), Some(1667.));
        assert_eq!(fielding_pct(Some(39), Some(1), Some(2)).map(|v| (v * 1000.).round()), Some(952.));
    }
}

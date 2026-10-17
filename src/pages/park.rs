//! Park detail: summary, attendance chart, home teams and season history.

use std::collections::BTreeMap;

use gpui_kit::component::{ActiveTheme as _, chart::LineChart, v_flex};
use gpui_kit::*;

use crate::api::models::{ParkDetail, ParkSeason};
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::grid::{Col, grid};
use crate::ui::widgets;

pub struct ParkPage {
    key: String,
    nav: Navigator,
    scroll: ScrollHandle,
    park: Loadable<ParkDetail>,
    _task: Option<Task<()>>,
}

/// One home team's tenure at a park.
#[derive(Clone)]
struct Tenant {
    team_id: String,
    lg_id: String,
    first: i32,
    last: i32,
    seasons: usize,
    games: i64,
}

fn tenants(seasons: &[ParkSeason]) -> Vec<Tenant> {
    let mut map: BTreeMap<(String, String), Tenant> = BTreeMap::new();
    for s in seasons {
        let team = s.team_id.clone().unwrap_or_default();
        let lg = s.lg_id.clone().unwrap_or_default();
        let t = map.entry((team.clone(), lg.clone())).or_insert(Tenant {
            team_id: team,
            lg_id: lg,
            first: s.year,
            last: s.year,
            seasons: 0,
            games: 0,
        });
        t.first = t.first.min(s.year);
        t.last = t.last.max(s.year);
        t.seasons += 1;
        t.games += s.games.unwrap_or(0);
    }
    let mut rows: Vec<_> = map.into_values().collect();
    rows.sort_by_key(|t| t.first);
    rows
}

impl ParkPage {
    pub fn new(key: String, nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self { key, nav, scroll: ScrollHandle::new(), park: Loadable::Idle, _task: None };
        page.reload(window, cx);
        page
    }
}

impl Page for ParkPage {
    fn title(&self, _: &App) -> SharedString {
        match &self.park {
            Loadable::Ready(p) => format::text(p.park_name.as_deref()).into(),
            _ => self.key.clone().into(),
        }
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let key = self.key.clone();
        self.park = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| {
                let mut park = api.park(&key)?;
                park.seasons.sort_by_key(|s| s.year);
                Ok(park)
            },
            |this: &mut Self, result, _| this.park = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for ParkPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content: AnyElement = match &self.park {
            Loadable::Ready(p) => {
                let location = [p.city.as_deref(), p.state.as_deref(), p.country.as_deref()]
                    .iter()
                    .flatten()
                    .filter(|s| !s.is_empty())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ");
                let tenants = tenants(&p.seasons);
                let first = p.seasons.first().map(|s| s.year);
                let last = p.seasons.last().map(|s| s.year);
                let distinct_years = p.seasons.iter().map(|s| s.year).collect::<std::collections::BTreeSet<_>>().len();
                let games: i64 = p.seasons.iter().filter_map(|s| s.games).sum();
                let attendance: i64 = p.seasons.iter().filter_map(|s| s.attendance).sum();
                let peak = p.seasons.iter().filter(|s| s.attendance.unwrap_or(0) > 0).max_by_key(|s| s.attendance.unwrap_or(0));
                let stats = widgets::stat_row(
                    vec![
                        ("Years Active", format::year_span(first, last)),
                        ("Seasons", format::int(Some(distinct_years as i64))),
                        ("Home Team(s)", format::int(Some(tenants.len() as i64))),
                        ("Home Games", format::int(Some(games))),
                        ("Total Attendance", format::int(Some(attendance))),
                        ("Peak Year", peak.map(|s| format!("{} ({})", s.year, format::int(s.attendance))).unwrap_or_else(|| format::DASH.into())),
                    ],
                    cx,
                );

                // Attendance by year, combining tenants that shared the park.
                let mut by_year: BTreeMap<i32, i64> = BTreeMap::new();
                for s in &p.seasons {
                    *by_year.entry(s.year).or_default() += s.attendance.unwrap_or(0);
                }
                let points: Vec<(i32, f64)> = by_year.into_iter().filter(|(_, a)| *a > 0).map(|(y, a)| (y, a as f64)).collect();
                let chart: Option<AnyElement> = (points.len() > 1).then(|| {
                    let ticks = (points.len() / 8).max(1);
                    widgets::section("Season Attendance", cx)
                        .child(
                            div().h(px(260.)).p_4().child(
                                LineChart::new(points)
                                    .id("attendance-chart")
                                    .x(|(y, _)| SharedString::from(y.to_string()))
                                    .y(|(_, a)| *a)
                                    .stroke(cx.theme().primary)
                                    .name("Attendance")
                                    .tick_margin(ticks)
                                    .y_axis(true)
                                    .y_tick_format(|v| format::int(Some(v as i64)))
                                    .tooltip_value(|_, v| format::int(Some(v as i64)).into()),
                            ),
                        )
                        .into_any_element()
                });

                let nav = self.nav.clone();
                let tenant_cols = vec![
                    Col::element("Team", 120., move |t: &Tenant, _| {
                        nav.button(
                            SharedString::from(format!("tenant-{}-{}", t.team_id, t.lg_id)),
                            t.team_id.clone(),
                            Route::TeamSeason { team_id: t.team_id.clone(), lg_id: t.lg_id.clone(), year: t.last },
                        )
                    }),
                    Col::text("League", 72., |t: &Tenant| t.lg_id.clone()),
                    Col::text("Years", 120., |t: &Tenant| format::year_span(Some(t.first), Some(t.last))),
                    Col::num("Seasons", 80., |t: &Tenant| t.seasons.to_string()),
                    Col::num("Home Games", 104., |t: &Tenant| format::int(Some(t.games))),
                ];
                let mut seasons = p.seasons.clone();
                seasons.reverse();
                let nav = self.nav.clone();
                let season_cols = vec![
                    Col::element("Year", 80., move |s: &ParkSeason, ix| match (s.team_id.as_deref(), s.lg_id.as_deref()) {
                        (Some(t), Some(l)) => nav
                            .button(
                                SharedString::from(format!("park-season-{}-{t}-{ix}", s.year)),
                                s.year.to_string(),
                                Route::TeamSeason { team_id: t.into(), lg_id: l.into(), year: s.year },
                            )
                            .into_any_element(),
                        _ => s.year.to_string().into_any_element(),
                    }),
                    Col::text("Team", 80., |s: &ParkSeason| format::text(s.team_id.as_deref())),
                    Col::text("League", 72., |s: &ParkSeason| format::text(s.lg_id.as_deref())),
                    Col::num("Games", 72., |s: &ParkSeason| format::int(s.games)),
                    Col::num("Attendance", 120., |s: &ParkSeason| format::int(s.attendance)),
                ];
                let season_body: AnyElement = if seasons.is_empty() {
                    widgets::empty_state("No Game History", "No home-game records are available for this ballpark.", cx)
                } else {
                    grid("park-seasons", season_cols, &seasons, cx)
                };
                v_flex()
                    .gap_5()
                    .child(widgets::page_header(Some("Ballpark"), format::text(p.park_name.as_deref()), Some(location.into()), vec![], cx))
                    .child(stats)
                    .children(chart)
                    .child(widgets::section("Home Teams", cx).child(grid("park-tenants", tenant_cols, &tenants, cx)))
                    .child(widgets::section("Season History", cx).child(season_body))
                    .into_any_element()
            }
            Loadable::Failed(message) => widgets::error_state(
                "park-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            _ => widgets::loading_state("Loading ballpark…", cx),
        };
        widgets::page_body("park", &self.scroll).child(widgets::page_column().child(content))
    }
}

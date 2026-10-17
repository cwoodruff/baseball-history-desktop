//! Franchise: totals and season-by-season history.

use gpui_kit::component::{Sizable as _, h_flex, tag::Tag, v_flex};
use gpui_kit::*;

use crate::api::models::{FranchiseDetail, FranchiseSeason};
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::grid::{Col, grid};
use crate::ui::widgets;

pub struct FranchisePage {
    id: String,
    nav: Navigator,
    scroll: ScrollHandle,
    franchise: Loadable<FranchiseDetail>,
    _task: Option<Task<()>>,
}

/// The postseason badges shown in a season row.
pub fn season_badges(
    won_ws: bool,
    won_pennant: bool,
    won_division: bool,
    lg: Option<&str>,
) -> AnyElement {
    let mut row = h_flex().gap_1();
    if won_ws {
        row = row.child(
            Tag::custom(
                widgets::hof_gold(),
                rgb(0x1a1a1a).into(),
                rgb(0xb8962e).into(),
            )
            .xsmall()
            .child("WS Champs"),
        );
    }
    if won_pennant {
        let label = match lg {
            Some(lg) if !lg.is_empty() => format!("{lg} Pennant"),
            _ => "Pennant".into(),
        };
        row = row.child(Tag::primary().xsmall().child(label));
    }
    if won_division {
        row = row.child(Tag::success().outline().xsmall().child("Division"));
    }
    row.into_any_element()
}

impl FranchisePage {
    pub fn new(id: String, nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            id,
            nav,
            scroll: ScrollHandle::new(),
            franchise: Loadable::Idle,
            _task: None,
        };
        page.reload(window, cx);
        page
    }
}

impl Page for FranchisePage {
    fn title(&self, _: &App) -> SharedString {
        match &self.franchise {
            Loadable::Ready(f) => f
                .franchise_name
                .clone()
                .unwrap_or_else(|| self.id.clone())
                .into(),
            _ => self.id.clone().into(),
        }
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let id = self.id.clone();
        self.franchise = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| api.franchise(&id),
            |this: &mut Self, result, _| this.franchise = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for FranchisePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content: AnyElement = match &self.franchise {
            Loadable::Ready(f) => {
                let name = f
                    .franchise_name
                    .clone()
                    .unwrap_or_else(|| f.franchise_id.clone());
                let badge: AnyElement = if f.is_active {
                    Tag::success().small().child("Active").into_any_element()
                } else {
                    Tag::secondary().small().child("Defunct").into_any_element()
                };
                let years = if f.is_active {
                    format!("{} - Present", format::year(f.first_year))
                } else {
                    format::year_span(f.first_year, f.last_year)
                };
                let nav = self.nav.clone();
                let columns = vec![
                    Col::element("Year", 80., move |s: &FranchiseSeason, _| {
                        nav.button(
                            SharedString::from(format!("season-{}-{}", s.year, s.team_id)),
                            s.year.to_string(),
                            Route::TeamSeason {
                                team_id: s.team_id.clone(),
                                lg_id: s.lg_id.clone().unwrap_or_default(),
                                year: s.year,
                            },
                        )
                    }),
                    Col::text("Team", 240., |s: &FranchiseSeason| {
                        format::text(s.team_name.as_deref())
                    }),
                    Col::text("Lg", 56., |s: &FranchiseSeason| {
                        format::text(s.lg_id.as_deref())
                    }),
                    Col::text("Div", 56., |s: &FranchiseSeason| {
                        format::text(s.div_id.as_deref())
                    }),
                    Col::num("W", 56., |s: &FranchiseSeason| format::int(s.wins)),
                    Col::num("L", 56., |s: &FranchiseSeason| format::int(s.losses)),
                    Col::num("Pct", 64., |s: &FranchiseSeason| format::rate(s.win_pct)),
                    Col::num("Rank", 56., |s: &FranchiseSeason| format::int(s.rank)),
                    Col::element("Postseason", 260., |s: &FranchiseSeason, _| {
                        season_badges(
                            s.won_world_series,
                            s.won_pennant,
                            s.won_division,
                            s.lg_id.as_deref(),
                        )
                    }),
                ];
                let seasons: AnyElement = if f.seasons.is_empty() {
                    widgets::empty_state(
                        "No Seasons Found",
                        "This franchise has no recorded seasons.",
                        cx,
                    )
                } else {
                    grid("franchise-seasons", columns, &f.seasons, cx)
                };
                v_flex()
                    .gap_5()
                    .child(widgets::page_header(
                        Some("Franchise"),
                        name,
                        Some(years.into()),
                        vec![badge],
                        cx,
                    ))
                    .child(widgets::stat_row(
                        vec![
                            ("Seasons", format::int(f.total_seasons)),
                            ("Wins", format::int(f.total_wins)),
                            ("Losses", format::int(f.total_losses)),
                            ("Win %", format::rate(f.win_pct)),
                            ("Pennants", format::int(f.pennant_wins)),
                            ("Championships", format::int(f.world_series_wins)),
                        ],
                        cx,
                    ))
                    .child(
                        widgets::section_with(
                            "Season History",
                            widgets::note(format!("{} seasons", f.seasons.len()), cx),
                            cx,
                        )
                        .child(seasons),
                    )
                    .into_any_element()
            }
            Loadable::Failed(message) => {
                let message = if message.starts_with("No record") {
                    "Franchise Not Found".to_string()
                } else {
                    message.clone()
                };
                widgets::error_state(
                    "franchise-error",
                    &message,
                    cx.listener(|this, _, window, cx| this.reload(window, cx)),
                )
            }
            _ => widgets::loading_state("Loading franchise…", cx),
        };
        widgets::page_body("franchise", &self.scroll).child(widgets::page_column().child(content))
    }
}

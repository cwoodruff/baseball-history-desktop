//! Postseason results by year and round.

use gpui_kit::component::{
    Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::{Paged, PostseasonSeries};
use crate::catalog;
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::grid::{Col, grid_with_row_style};
use crate::ui::picker::{self, Opt, Picker, PickerEvent};
use crate::ui::widgets;

const PAGE_SIZE: u32 = 50;
const YEARS_SHOWN: i32 = 60;

pub struct PostseasonPage {
    nav: Navigator,
    scroll: ScrollHandle,
    year: Option<i32>,
    round: Option<String>,
    page: u32,
    year_picker: Picker,
    round_picker: Picker,
    series: Loadable<Paged<PostseasonSeries>>,
    _task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl PostseasonPage {
    pub fn new(
        year: Option<i32>,
        nav: Navigator,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let first = (catalog::LAST_SEASON - YEARS_SHOWN + 1).min(year.unwrap_or(i32::MAX));
        let year_picker = picker::picker(
            picker::years("All Years", first, catalog::LAST_SEASON),
            &year.map(|y| y.to_string()).unwrap_or_default(),
            true,
            window,
            cx,
        );
        let round_picker = picker::picker(
            std::iter::once(Opt::any("All Rounds"))
                .chain(
                    catalog::POSTSEASON_ROUNDS
                        .iter()
                        .map(|r| Opt::new(r.value, format!("{} — {}", r.value, r.label))),
                )
                .collect(),
            "",
            false,
            window,
            cx,
        );
        let subscriptions = vec![
            cx.subscribe_in(
                &year_picker,
                window,
                |this, _, event: &PickerEvent, window, cx| {
                    this.year = picker::value_of(event).and_then(|v| v.parse().ok());
                    this.page = 1;
                    this.reload(window, cx);
                },
            ),
            cx.subscribe_in(
                &round_picker,
                window,
                |this, _, event: &PickerEvent, window, cx| {
                    this.round = picker::value_of(event);
                    this.page = 1;
                    this.reload(window, cx);
                },
            ),
        ];
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            year,
            round: None,
            page: 1,
            year_picker,
            round_picker,
            series: Loadable::Idle,
            _task: None,
            _subscriptions: subscriptions,
        };
        page.reload(window, cx);
        page
    }

    fn set_year(&mut self, year: Option<i32>, window: &mut Window, cx: &mut Context<Self>) {
        self.year = year;
        self.page = 1;
        picker::select_value(
            &self.year_picker,
            &year.map(|y| y.to_string()).unwrap_or_default(),
            window,
            cx,
        );
        self.reload(window, cx);
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.round = None;
        picker::select_value(&self.round_picker, "", window, cx);
        self.set_year(None, window, cx);
    }
}

fn team_button(
    nav: &Navigator,
    id: String,
    team: Option<&str>,
    lg: Option<&str>,
    year: i32,
) -> AnyElement {
    let label = match (team, lg) {
        (Some(t), Some(l)) if !l.is_empty() => format!("{t} ({l})"),
        (Some(t), _) => t.to_string(),
        _ => format::DASH.to_string(),
    };
    match (team, lg) {
        (Some(t), Some(l)) if !l.is_empty() => nav
            .button(
                SharedString::from(id),
                label,
                Route::TeamSeason {
                    team_id: t.to_string(),
                    lg_id: l.to_string(),
                    year,
                },
            )
            .into_any_element(),
        _ => label.into_any_element(),
    }
}

impl Page for PostseasonPage {
    fn title(&self, _: &App) -> SharedString {
        match self.year {
            Some(y) => format!("Postseason {y}").into(),
            None => "Postseason".into(),
        }
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let (year, round, page) = (self.year, self.round.clone(), self.page);
        self.series = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| {
                let mut result = api.postseason_series(year, round.as_deref(), page, PAGE_SIZE)?;
                result.data.sort_by(|a, b| {
                    b.year
                        .cmp(&a.year)
                        .then(catalog::round_order(&a.round).cmp(&catalog::round_order(&b.round)))
                        .then(a.round.cmp(&b.round))
                });
                Ok(result)
            },
            |this: &mut Self, result, _| this.series = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for PostseasonPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let filters = widgets::filter_bar(cx)
            .child(widgets::field(
                "Year",
                picker::dropdown(&self.year_picker, 120.),
                cx,
            ))
            .child(widgets::field(
                "Round",
                picker::dropdown(&self.round_picker, 280.),
                cx,
            ))
            .when(self.year.is_some() || self.round.is_some(), |this| {
                this.child(
                    Button::new("ps-clear")
                        .small()
                        .ghost()
                        .label("Clear Filters")
                        .on_click(cx.listener(|this, _, window, cx| this.clear(window, cx))),
                )
            });
        let body: AnyElement = match &self.series {
            Loadable::Ready(p) if p.data.is_empty() => widgets::empty_state(
                "No Postseason Data Found",
                "No series match the selected filters.",
                cx,
            ),
            Loadable::Ready(p) => {
                let this = cx.entity().downgrade();
                let year_this = this.clone();
                let (nav_w, nav_l) = (self.nav.clone(), self.nav.clone());
                let cols = vec![
                    Col::element("Year", 72., move |s: &PostseasonSeries, ix| {
                        let this = year_this.clone();
                        let year = s.year;
                        Button::new(SharedString::from(format!(
                            "ps-year-{}-{}-{ix}",
                            s.year, s.round
                        )))
                        .ghost()
                        .xsmall()
                        .compact()
                        .label(s.year.to_string())
                        .tooltip(format!("Show only {year}"))
                        .on_click(move |_, window, cx| {
                            this.update(cx, |this, cx| this.set_year(Some(year), window, cx))
                                .ok();
                        })
                    }),
                    Col::element("Round", 220., |s: &PostseasonSeries, _| {
                        let label = catalog::round_label(&s.round);
                        if s.round == "WS" {
                            Tag::custom(
                                widgets::hof_gold(),
                                rgb(0x1a1a1a).into(),
                                rgb(0xb8962e).into(),
                            )
                            .xsmall()
                            .child(label)
                        } else {
                            Tag::secondary().xsmall().child(label)
                        }
                    }),
                    Col::element("Winner", 160., move |s: &PostseasonSeries, ix| {
                        team_button(
                            &nav_w,
                            format!("ps-w-{}-{}-{ix}", s.year, s.round),
                            s.winner_team_id.as_deref(),
                            s.winner_lg_id.as_deref(),
                            s.year,
                        )
                    }),
                    Col::num("Result", 80., |s: &PostseasonSeries| {
                        match s.ties.unwrap_or(0) {
                            0 => format!("{}-{}", s.wins.unwrap_or(0), s.losses.unwrap_or(0)),
                            t => format!("{}-{}-{t}", s.wins.unwrap_or(0), s.losses.unwrap_or(0)),
                        }
                    }),
                    Col::element("Loser", 160., move |s: &PostseasonSeries, ix| {
                        team_button(
                            &nav_l,
                            format!("ps-l-{}-{}-{ix}", s.year, s.round),
                            s.loser_team_id.as_deref(),
                            s.loser_lg_id.as_deref(),
                            s.year,
                        )
                    }),
                ];
                let gold = widgets::hof_gold().opacity(0.15);
                v_flex()
                    .gap_3()
                    .child(
                        widgets::section_with(
                            "Series",
                            widgets::note(
                                format!("{} series", format::int(Some(p.total_count as i64))),
                                cx,
                            ),
                            cx,
                        )
                        .child(grid_with_row_style(
                            "postseason-table",
                            cols,
                            &p.data,
                            |s| (s.round == "WS").then_some(gold),
                            cx,
                        )),
                    )
                    .child(widgets::pager(
                        "ps-pager",
                        p.page,
                        p.total_pages,
                        p.total_count,
                        move |page, window, cx| {
                            this.update(cx, |this, cx| {
                                this.page = page;
                                this.reload(window, cx);
                            })
                            .ok();
                        },
                        cx,
                    ))
                    .into_any_element()
            }
            Loadable::Failed(message) => widgets::error_state(
                "ps-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            _ => widgets::loading_state("Loading postseason results…", cx),
        };
        widgets::page_body("postseason", &self.scroll).child(
            widgets::page_column()
                .child(widgets::page_header(
                    Some("Stats"),
                    "Postseason Results",
                    None,
                    vec![],
                    cx,
                ))
                .child(filters)
                .child(body)
                .child(h_flex()),
        )
    }
}

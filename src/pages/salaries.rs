//! Salary Explorer: top salaries by year, and a team's payroll for one season.

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::{Paged, SalaryEntry, TeamPayroll};
use crate::catalog;
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::ui::grid::{Col, grid};
use crate::ui::picker::{self, Opt, Picker, PickerEvent};
use crate::ui::widgets;

const PAGE_SIZE: u32 = 50;

enum Data {
    Leaders(Paged<SalaryEntry>),
    Payroll(TeamPayroll),
}

pub struct SalariesPage {
    nav: Navigator,
    scroll: ScrollHandle,
    year: Option<i32>,
    team: Option<String>,
    page: u32,
    year_picker: Picker,
    team_picker: Picker,
    data: Loadable<Data>,
    _task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

fn team_options(year: Option<i32>) -> Vec<Opt> {
    std::iter::once(Opt::any("All Teams"))
        .chain(
            catalog::SALARY_TEAMS
                .iter()
                .filter(|(_, _, first, last)| year.is_none_or(|y| (*first..=*last).contains(&y)))
                .map(|(id, name, _, _)| Opt::new(*id, format!("{name} ({id})"))),
        )
        .collect()
}

fn team_name(id: &str) -> String {
    catalog::SALARY_TEAMS
        .iter()
        .find(|(t, ..)| *t == id)
        .map(|(_, name, ..)| name.to_string())
        .unwrap_or_else(|| id.to_string())
}

impl SalariesPage {
    pub fn new(nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let year_picker = picker::picker(
            picker::years(
                "All Years",
                catalog::FIRST_SALARY_SEASON,
                catalog::LAST_SALARY_SEASON,
            ),
            "",
            true,
            window,
            cx,
        );
        let team_picker = picker::picker(team_options(None), "", true, window, cx);
        let subscriptions = vec![
            cx.subscribe_in(
                &year_picker,
                window,
                |this, _, event: &PickerEvent, window, cx| {
                    this.year = picker::value_of(event).and_then(|v| v.parse().ok());
                    // Keep the team only if it played that year.
                    let options = team_options(this.year);
                    let keep = this.team.clone().filter(|t| {
                        catalog::SALARY_TEAMS.iter().any(|(id, _, first, last)| {
                            id == t && this.year.is_none_or(|y| (*first..=*last).contains(&y))
                        })
                    });
                    this.team_picker
                        .update(cx, |state, cx| state.set_items(options, window, cx));
                    picker::select_value(
                        &this.team_picker,
                        keep.as_deref().unwrap_or(""),
                        window,
                        cx,
                    );
                    this.team = keep;
                    this.page = 1;
                    this.reload(window, cx);
                },
            ),
            cx.subscribe_in(
                &team_picker,
                window,
                |this, _, event: &PickerEvent, window, cx| {
                    this.team = picker::value_of(event);
                    this.page = 1;
                    this.reload(window, cx);
                },
            ),
        ];
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            year: None,
            team: None,
            page: 1,
            year_picker,
            team_picker,
            data: Loadable::Idle,
            _task: None,
            _subscriptions: subscriptions,
        };
        page.reload(window, cx);
        page
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.year = None;
        self.team = None;
        self.page = 1;
        picker::select_value(&self.year_picker, "", window, cx);
        self.team_picker.update(cx, |state, cx| {
            state.set_items(team_options(None), window, cx)
        });
        picker::select_value(&self.team_picker, "", window, cx);
        self.reload(window, cx);
    }

    fn table(&self, rows: &[SalaryEntry], offset: usize, cx: &App) -> AnyElement {
        let nav = self.nav.clone();
        let mut cols = vec![
            Col::element("#", 56., move |_: &SalaryEntry, ix| {
                (ix + 1 + offset).to_string()
            })
            .right(),
            Col::element("Player", 260., move |e: &SalaryEntry, ix| {
                widgets::player_link(
                    SharedString::from(format!("sal-{}-{}-{ix}", e.player_id, e.year)),
                    &e.player_id,
                    e.player_name.clone().unwrap_or_else(|| e.player_id.clone()),
                    false,
                    &nav,
                )
            }),
        ];
        if self.year.is_none() {
            cols.push(Col::text("Year", 64., |e: &SalaryEntry| e.year.to_string()));
        }
        cols.push(Col::text("Team", 240., |e: &SalaryEntry| {
            e.team_id
                .as_deref()
                .map(team_name)
                .unwrap_or_else(|| format::DASH.into())
        }));
        cols.push(Col::num("Salary", 140., |e: &SalaryEntry| {
            format::money(e.salary)
        }));
        grid("salary-table", cols, rows, cx)
    }
}

impl Page for SalariesPage {
    fn title(&self, _: &App) -> SharedString {
        "Salary Explorer".into()
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let (year, team, page) = (self.year, self.team.clone(), self.page);
        self.data = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| match (team, year) {
                (Some(team), Some(year)) => {
                    let mut payroll = api.team_payroll(&team, year)?;
                    payroll.players.sort_by_key(|p| std::cmp::Reverse(p.salary));
                    Ok(Data::Payroll(payroll))
                }
                _ => api.salary_leaders(year, page, PAGE_SIZE).map(Data::Leaders),
            },
            |this: &mut Self, result, _| this.data = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for SalariesPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let filters = widgets::filter_bar(cx)
            .child(widgets::field(
                "Year",
                picker::dropdown(&self.year_picker, 120.),
                cx,
            ))
            .child(widgets::field(
                "Team",
                picker::dropdown(&self.team_picker, 300.),
                cx,
            ))
            .child(
                Button::new("salary-clear")
                    .small()
                    .ghost()
                    .label("Clear Filters")
                    .on_click(cx.listener(|this, _, window, cx| this.clear(window, cx))),
            );
        let hint = (self.team.is_some() && self.year.is_none())
            .then(|| widgets::note("Choose a year to see that team's payroll.", cx));
        let body: AnyElement = match &self.data {
            Loadable::Ready(Data::Leaders(p)) if p.data.is_empty() => widgets::empty_state(
                "No Salary Data Found",
                "No salaries match these filters. Salary data is available from 1985.",
                cx,
            ),
            Loadable::Ready(Data::Payroll(p)) if p.players.is_empty() => widgets::empty_state(
                "No Salary Data Found",
                "No salaries are on record for this team and year. Salary data is available from 1985.",
                cx,
            ),
            Loadable::Ready(Data::Leaders(p)) => {
                let this = cx.entity().downgrade();
                let offset = ((p.page.max(1) - 1) * PAGE_SIZE) as usize;
                v_flex()
                    .gap_3()
                    .child(
                        widgets::section("Top Salaries", cx).child(self.table(&p.data, offset, cx)),
                    )
                    .child(widgets::pager(
                        "salary-pager",
                        p.page,
                        p.total_pages,
                        p.total_count,
                        move |page, window, cx| {
                            this.update(cx, |this, cx| {
                                this.page = page;
                                this.scroll.set_offset(point(px(0.), px(0.)));
                                this.reload(window, cx);
                            })
                            .ok();
                        },
                        cx,
                    ))
                    .into_any_element()
            }
            Loadable::Ready(Data::Payroll(p)) => v_flex()
                .gap_4()
                .child(
                    v_flex()
                        .w_full()
                        .p_5()
                        .gap_1()
                        .rounded(cx.theme().radius_lg)
                        .bg(widgets::brand_navy())
                        .text_color(white())
                        .child(div().text_sm().opacity(0.85).child(format!(
                            "{} — {} Total Payroll",
                            team_name(&p.team_id),
                            p.year
                        )))
                        .child(
                            div()
                                .text_3xl()
                                .font_bold()
                                .child(format::money(p.total_payroll)),
                        )
                        .child(
                            div()
                                .text_sm()
                                .opacity(0.85)
                                .child(format!("{} players", format::int(p.player_count))),
                        ),
                )
                .child(widgets::section("Roster Salaries", cx).child(self.table(&p.players, 0, cx)))
                .into_any_element(),
            Loadable::Failed(message) => widgets::error_state(
                "salary-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            _ => widgets::loading_state("Loading salaries…", cx),
        };
        widgets::page_body("salaries", &self.scroll).child(
            widgets::page_column()
                .child(widgets::page_header(
                    Some("Stats"),
                    "Salary Explorer",
                    Some("Player salaries from 1985 to 2016.".into()),
                    vec![],
                    cx,
                ))
                .child(filters)
                .children(hint)
                .child(body)
                .when(false, |this| this),
        )
    }
}

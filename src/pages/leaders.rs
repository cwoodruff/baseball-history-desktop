//! Batting and Pitching Leaders: one page, two kinds.

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex, v_flex,
};
use gpui_kit::*;

use crate::api::LeaderRequest;
use crate::api::models::{BattingLeader, Paged, PitchingLeader};
use crate::catalog::{self, StatChoice};
use crate::format::{self, DASH};
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::{ReferencePage, Route};
use crate::ui::grid::{Col, grid};
use crate::ui::picker::{self, Opt, Picker, PickerEvent};
use crate::ui::widgets;

const PAGE_SIZE: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Batting,
    Pitching,
}

impl Kind {
    fn stats(self) -> &'static [StatChoice] {
        match self {
            Kind::Batting => catalog::BATTING_STATS,
            Kind::Pitching => catalog::PITCHING_STATS,
        }
    }

    fn minimums(self) -> &'static [(Option<i32>, &'static str)] {
        match self {
            Kind::Batting => catalog::MIN_AB,
            Kind::Pitching => catalog::MIN_IP,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Kind::Batting => "Batting Leaders",
            Kind::Pitching => "Pitching Leaders",
        }
    }
}

enum Rows {
    Batting(Paged<BattingLeader>),
    Pitching(Paged<PitchingLeader>),
}

impl Rows {
    fn meta(&self) -> (u32, u32, u32) {
        match self {
            Rows::Batting(p) => (p.page, p.total_pages, p.total_count),
            Rows::Pitching(p) => (p.page, p.total_pages, p.total_count),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Rows::Batting(p) => p.data.is_empty(),
            Rows::Pitching(p) => p.data.is_empty(),
        }
    }
}

pub struct LeadersPage {
    kind: Kind,
    nav: Navigator,
    scroll: ScrollHandle,
    request: LeaderRequest,
    stat_picker: Picker,
    from_picker: Picker,
    to_picker: Picker,
    league_picker: Picker,
    min_picker: Picker,
    rows: Loadable<Rows>,
    _task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

fn min_key(min: Option<i32>) -> String {
    min.map(|m| m.to_string()).unwrap_or_else(|| "q".into())
}

impl LeadersPage {
    pub fn new(kind: Kind, nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let stat = kind.stats()[0];
        let request = LeaderRequest {
            stat: stat.value.into(),
            from_year: None,
            to_year: None,
            league: None,
            qualified: stat.rate,
            minimum: if stat.rate { None } else { Some(0) },
            single_season: false,
            page: 1,
            page_size: PAGE_SIZE,
        };
        let stat_picker = picker::picker(
            kind.stats().iter().map(|s| Opt::new(s.value, s.label)).collect(),
            stat.value,
            false,
            window,
            cx,
        );
        let from_picker = picker::picker(picker::years("All Time", catalog::FIRST_SEASON, catalog::LAST_SEASON), "", true, window, cx);
        let to_picker = picker::picker(picker::years("Present", catalog::FIRST_SEASON, catalog::LAST_SEASON), "", true, window, cx);
        let league_picker = picker::picker(
            std::iter::once(Opt::any("All Leagues"))
                .chain(catalog::LEAGUES.iter().map(|l| Opt::new(l.value, l.label)))
                .collect(),
            "",
            false,
            window,
            cx,
        );
        let min_picker = picker::picker(
            kind.minimums().iter().map(|(v, label)| Opt::new(min_key(*v), *label)).collect(),
            &min_key(request.minimum.filter(|_| !request.qualified)),
            false,
            window,
            cx,
        );
        if !stat.rate {
            picker::select_value(&min_picker, "0", window, cx);
        }
        let subscriptions = vec![
            cx.subscribe_in(&stat_picker, window, |this, _, event: &PickerEvent, window, cx| {
                if let Some(stat) = picker::value_of(event) {
                    this.set_stat(&stat, window, cx);
                }
            }),
            cx.subscribe_in(&from_picker, window, |this, _, event: &PickerEvent, window, cx| {
                this.request.from_year = picker::value_of(event).and_then(|v| v.parse().ok());
                this.refetch(window, cx);
            }),
            cx.subscribe_in(&to_picker, window, |this, _, event: &PickerEvent, window, cx| {
                this.request.to_year = picker::value_of(event).and_then(|v| v.parse().ok());
                this.refetch(window, cx);
            }),
            cx.subscribe_in(&league_picker, window, |this, _, event: &PickerEvent, window, cx| {
                this.request.league = picker::value_of(event);
                this.refetch(window, cx);
            }),
            cx.subscribe_in(&min_picker, window, |this, _, event: &PickerEvent, window, cx| {
                this.set_minimum(picker::value_of(event).as_deref(), window, cx);
            }),
        ];
        let mut page = Self {
            kind,
            nav,
            scroll: ScrollHandle::new(),
            request,
            stat_picker,
            from_picker,
            to_picker,
            league_picker,
            min_picker,
            rows: Loadable::Idle,
            _task: None,
            _subscriptions: subscriptions,
        };
        page.reload(window, cx);
        page
    }

    fn stat(&self) -> StatChoice {
        self.kind
            .stats()
            .iter()
            .copied()
            .find(|s| s.value == self.request.stat)
            .unwrap_or(self.kind.stats()[0])
    }

    fn set_stat(&mut self, stat: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.request.stat == stat {
            return;
        }
        self.request.stat = stat.to_string();
        // Rate stats default to qualified, counting stats to no minimum.
        let rate = self.stat().rate;
        self.request.qualified = rate;
        self.request.minimum = if rate { None } else { Some(0) };
        picker::select_value(&self.stat_picker, stat, window, cx);
        picker::select_value(&self.min_picker, if rate { "q" } else { "0" }, window, cx);
        self.refetch(window, cx);
    }

    fn set_minimum(&mut self, key: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        match key {
            Some("q") | None => {
                self.request.qualified = true;
                self.request.minimum = None;
            }
            Some("0") => {
                self.request.qualified = false;
                self.request.minimum = None;
            }
            Some(n) => {
                self.request.qualified = false;
                self.request.minimum = n.parse().ok();
            }
        }
        self.refetch(window, cx);
    }

    /// Re-queries from the first page after a filter change.
    fn refetch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.request.page = 1;
        self.reload(window, cx);
    }

    fn set_page(&mut self, page: u32, window: &mut Window, cx: &mut Context<Self>) {
        self.request.page = page;
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.reload(window, cx);
    }

    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let stat = self.kind.stats()[0];
        self.request.from_year = None;
        self.request.to_year = None;
        self.request.league = None;
        self.request.single_season = false;
        picker::select_value(&self.from_picker, "", window, cx);
        picker::select_value(&self.to_picker, "", window, cx);
        picker::select_value(&self.league_picker, "", window, cx);
        self.request.stat.clear();
        self.set_stat(stat.value, window, cx);
    }

    /// A header handler that re-ranks the board by `stat`.
    fn rank_by(&self, stat: &'static str, cx: &mut Context<Self>) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        cx.listener(move |this, _, window, cx| this.set_stat(stat, window, cx))
    }

    fn sort_col<'a, T>(&self, col: Col<'a, T>, stat: &'static str, cx: &mut Context<Self>) -> Col<'a, T> {
        let active = self.request.stat == stat;
        let label = self.kind.stats().iter().find(|s| s.value == stat).map(|s| s.label).unwrap_or(stat);
        col.on_header_click(self.rank_by(stat, cx))
            .active(active)
            .tooltip(format!("Rank by {label}"))
    }

    fn batting_table(&self, page: &Paged<BattingLeader>, cx: &mut Context<Self>) -> AnyElement {
        let single = self.request.single_season;
        let nav = self.nav.clone();
        let mut cols: Vec<Col<BattingLeader>> = vec![
            Col::num("Rank", 56., |r: &BattingLeader| r.rank.to_string()),
            Col::element("Player", 220., move |r: &BattingLeader, _| {
                widgets::player_link(
                    SharedString::from(format!("bl-{}-{}-{}", r.player_id, r.year.unwrap_or(0), r.rank)),
                    &r.player_id,
                    r.player_name.clone().unwrap_or_else(|| r.player_id.clone()),
                    r.is_hall_of_famer,
                    &nav,
                )
            }),
        ];
        if single {
            cols.push(Col::text("Year", 56., |r: &BattingLeader| format::year(r.year)));
            cols.push(Col::text("Team", 180., |r: &BattingLeader| format::text(r.team_name.as_deref().or(r.team_id.as_deref()))));
        }
        let extra: Option<Col<BattingLeader>> = match self.request.stat.as_str() {
            "obp" => Some(Col::num("OBP", 64., |r: &BattingLeader| format::rate(r.obp))),
            "slg" => Some(Col::num("SLG", 64., |r: &BattingLeader| format::rate(r.slg))),
            "tb" => Some(Col::num("TB", 64., |r: &BattingLeader| {
                format::int(Some(
                    r.hits.unwrap_or(0) + r.doubles.unwrap_or(0) + 2 * r.triples.unwrap_or(0) + 3 * r.home_runs.unwrap_or(0),
                ))
            })),
            _ => None,
        };
        cols.extend([
            self.sort_col(Col::num("G", 56., |r: &BattingLeader| format::int(r.games)), "g", cx),
            Col::num("AB", 64., |r: &BattingLeader| format::int(r.at_bats)),
            self.sort_col(Col::num("H", 56., |r: &BattingLeader| format::int(r.hits)), "h", cx),
            self.sort_col(Col::num("2B", 56., |r: &BattingLeader| format::int(r.doubles)), "2b", cx),
            self.sort_col(Col::num("3B", 56., |r: &BattingLeader| format::int(r.triples)), "3b", cx),
            self.sort_col(Col::num("HR", 56., |r: &BattingLeader| format::int(r.home_runs)), "hr", cx),
            self.sort_col(Col::num("RBI", 60., |r: &BattingLeader| format::int(r.rbi)), "rbi", cx),
            self.sort_col(Col::num("R", 56., |r: &BattingLeader| format::int(r.runs)), "r", cx),
            self.sort_col(Col::num("SB", 56., |r: &BattingLeader| format::int(r.stolen_bases)), "sb", cx),
            self.sort_col(Col::num("BB", 56., |r: &BattingLeader| format::int(r.walks)), "bb", cx),
            self.sort_col(Col::num("AVG", 64., |r: &BattingLeader| format::rate(r.batting_average)), "avg", cx),
            self.sort_col(Col::num("OPS", 64., |r: &BattingLeader| format::rate(r.ops)), "ops", cx),
        ]);
        if let Some(extra) = extra {
            cols.push(extra.active(true));
        }
        grid("batting-leaders", cols, &page.data, cx)
    }

    fn pitching_table(&self, page: &Paged<PitchingLeader>, cx: &mut Context<Self>) -> AnyElement {
        let single = self.request.single_season;
        let nav = self.nav.clone();
        let mut cols: Vec<Col<PitchingLeader>> = vec![
            Col::num("Rank", 56., |r: &PitchingLeader| r.rank.to_string()),
            Col::element("Player", 220., move |r: &PitchingLeader, _| {
                widgets::player_link(
                    SharedString::from(format!("pl-{}-{}-{}", r.player_id, r.year.unwrap_or(0), r.rank)),
                    &r.player_id,
                    r.player_name.clone().unwrap_or_else(|| r.player_id.clone()),
                    r.is_hall_of_famer,
                    &nav,
                )
            }),
        ];
        if single {
            cols.push(Col::text("Year", 56., |r: &PitchingLeader| format::year(r.year)));
            cols.push(Col::text("Team", 180., |r: &PitchingLeader| format::text(r.team_name.as_deref().or(r.team_id.as_deref()))));
        }
        let extra: Option<Col<PitchingLeader>> = match self.request.stat.as_str() {
            "cg" => Some(Col::num("CG", 56., |r: &PitchingLeader| format::int(r.complete_games))),
            "sho" => Some(Col::num("SHO", 56., |r: &PitchingLeader| format::int(r.shutouts))),
            "k9" => Some(Col::num("K/9", 64., |r: &PitchingLeader| {
                let ip = r.innings_pitched.unwrap_or(0.);
                let innings = ip.trunc() + ((ip - ip.trunc()) * 10.).round() / 3.;
                if innings > 0. { format!("{:.1}", r.strikeouts.unwrap_or(0) as f64 * 9. / innings) } else { DASH.into() }
            })),
            "wpct" => Some(Col::num("W%", 64., |r: &PitchingLeader| {
                let (w, l) = (r.wins.unwrap_or(0), r.losses.unwrap_or(0));
                format::rate((w + l > 0).then(|| w as f64 / (w + l) as f64))
            })),
            _ => None,
        };
        cols.extend([
            self.sort_col(Col::num("G", 56., |r: &PitchingLeader| format::int(r.games)), "g", cx),
            self.sort_col(Col::num("GS", 56., |r: &PitchingLeader| format::int(r.games_started)), "gs", cx),
            self.sort_col(Col::num("W", 56., |r: &PitchingLeader| format::int(r.wins)), "w", cx),
            self.sort_col(Col::num("L", 56., |r: &PitchingLeader| format::int(r.losses)), "l", cx),
            self.sort_col(Col::num("SV", 56., |r: &PitchingLeader| format::int(r.saves)), "sv", cx),
            self.sort_col(Col::num("IP", 72., |r: &PitchingLeader| format::innings(r.innings_pitched)), "ip", cx),
            self.sort_col(Col::num("SO", 64., |r: &PitchingLeader| format::int(r.strikeouts)), "so", cx),
            Col::num("BB", 56., |r: &PitchingLeader| format::int(r.walks)),
            self.sort_col(Col::num("ERA", 64., |r: &PitchingLeader| format::decimal2(r.era)), "era", cx),
            self.sort_col(Col::num("WHIP", 64., |r: &PitchingLeader| format::decimal2(r.whip)), "whip", cx),
        ]);
        if let Some(extra) = extra {
            cols.push(extra.active(true));
        }
        grid("pitching-leaders", cols, &page.data, cx)
    }
}

impl Page for LeadersPage {
    fn title(&self, _: &App) -> SharedString {
        self.kind.label().into()
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let request = self.request.clone();
        let kind = self.kind;
        self.rows = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| match kind {
                Kind::Batting => api.batting_leaders(&request).map(Rows::Batting),
                Kind::Pitching => api.pitching_leaders(&request).map(Rows::Pitching),
            },
            |this: &mut Self, result, _| this.rows = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for LeadersPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stat = self.stat();
        let unit = if self.kind == Kind::Batting { "AB" } else { "IP" };
        let filters = widgets::filter_bar(cx)
            .child(widgets::field("Stat", picker::dropdown(&self.stat_picker, 200.), cx))
            .child(widgets::field("From Year", picker::dropdown(&self.from_picker, 120.), cx))
            .child(widgets::field("To Year", picker::dropdown(&self.to_picker, 120.), cx))
            .child(widgets::field("League", picker::dropdown(&self.league_picker, 220.), cx))
            .child(widgets::field(&format!("Min {unit}"), picker::dropdown(&self.min_picker, 140.), cx))
            .child(
                div().pb_1().child(
                    Checkbox::new("single-season")
                        .label("Single Season")
                        .checked(self.request.single_season)
                        .on_click(cx.listener(|this, checked: &bool, window, cx| {
                            this.request.single_season = *checked;
                            this.refetch(window, cx);
                        })),
                ),
            )
            .child(
                Button::new("reset-leaders")
                    .small()
                    .ghost()
                    .label("Reset")
                    .on_click(cx.listener(|this, _, window, cx| this.reset(window, cx))),
            );

        let scope_note = h_flex()
            .gap_1()
            .items_center()
            .flex_wrap()
            .child(widgets::note(
                "Raw totals, not context-adjusted — no park factors or era adjustments, and surviving Negro Leagues records cover shorter seasons.",
                cx,
            ))
            .child(
                Button::new("scope-link")
                    .ghost()
                    .xsmall()
                    .compact()
                    .label("About the data scope")
                    .on_click(self.nav.to(Route::Reference(ReferencePage::About))),
            );

        let qualified_note = self.request.qualified.then(|| {
            let rule = if self.kind == Kind::Batting {
                "Qualified: at least 3.1 plate appearances per team game in each season counted (career boards sum each season's threshold)."
            } else {
                "Qualified: at least 1 inning pitched per team game in each season counted (career boards sum each season's threshold)."
            };
            div()
                .px_3()
                .py_2()
                .rounded(cx.theme().radius)
                .bg(cx.theme().info.opacity(0.12))
                .text_sm()
                .child(rule)
        });

        let (body, meta): (AnyElement, Option<(u32, u32, u32)>) = match &self.rows {
            Loadable::Ready(rows) if rows.is_empty() => (
                widgets::empty_state("No Statistics Available", "No statistics match the selected criteria.", cx),
                None,
            ),
            Loadable::Ready(Rows::Batting(page)) => {
                let page = page.clone();
                (self.batting_table(&page, cx), Some((page.page, page.total_pages, page.total_count)))
            }
            Loadable::Ready(Rows::Pitching(page)) => {
                let page = page.clone();
                (self.pitching_table(&page, cx), Some((page.page, page.total_pages, page.total_count)))
            }
            Loadable::Failed(message) => (
                widgets::error_state("leaders-error", message, cx.listener(|this, _, window, cx| this.reload(window, cx))),
                None,
            ),
            _ => (widgets::loading_state("Loading stats…", cx), None),
        };
        let count = self.rows.ready().map(|r| r.meta().2);
        let card = widgets::section_with(
            format!("{} - {}", self.kind.label(), stat.label),
            widgets::note(count.map(|c| format!("{} players", format::int(Some(c as i64)))).unwrap_or_default(), cx),
            cx,
        )
        .child(body);
        let pager = meta.map(|(page, pages, total)| {
            let this = cx.entity().downgrade();
            widgets::pager(
                "leaders-pager",
                page,
                pages,
                total,
                move |page, window, cx| {
                    this.update(cx, |this, cx| this.set_page(page, window, cx)).ok();
                },
                cx,
            )
        });

        widgets::page_body("leaders", &self.scroll).child(
            widgets::page_column()
                .child(widgets::page_header(Some("Stats"), self.kind.label(), None, vec![], cx))
                .child(filters)
                .child(scope_note)
                .children(qualified_note)
                .child(card)
                .children(pager)
                .child(v_flex()),
        )
    }
}

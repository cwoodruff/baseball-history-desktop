//! Teams & Franchises: league filter, name and era filters, active and historical groups.

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    tab::TabBar,
    h_flex,
    input::{Input, InputEvent, InputState},
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::FranchiseSummary;
use crate::catalog;
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::picker::{self, Opt, Picker, PickerEvent};
use crate::ui::widgets;

pub struct TeamsPage {
    nav: Navigator,
    scroll: ScrollHandle,
    league: Option<&'static str>,
    era: Option<i32>,
    name: Entity<InputState>,
    era_picker: Picker,
    franchises: Loadable<Vec<FranchiseSummary>>,
    _task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl TeamsPage {
    pub fn new(nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Filter by name…"));
        let eras = std::iter::once(Opt::any("All Eras"))
            .chain(catalog::decades().into_iter().map(|d| Opt::new(d.to_string(), format!("{d}s"))))
            .collect();
        let era_picker = picker::picker(eras, "", false, window, cx);
        let subscriptions = vec![
            cx.subscribe_in(&name, window, |_, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.subscribe_in(&era_picker, window, |this, _, event: &PickerEvent, _, cx| {
                this.era = picker::value_of(event).and_then(|v| v.parse().ok());
                cx.notify();
            }),
        ];
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            league: None,
            era: None,
            name,
            era_picker,
            franchises: Loadable::Idle,
            _task: None,
            _subscriptions: subscriptions,
        };
        page.reload(window, cx);
        page
    }

    fn set_league(&mut self, league: Option<&'static str>, window: &mut Window, cx: &mut Context<Self>) {
        if self.league != league {
            self.league = league;
            self.reload(window, cx);
        }
    }

    fn clear_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.name.update(cx, |state, cx| state.set_value("", window, cx));
        picker::select_value(&self.era_picker, "", window, cx);
        self.era = None;
        self.set_league(None, window, cx);
        cx.notify();
    }

    fn visible(&self, cx: &App) -> Vec<FranchiseSummary> {
        let query = self.name.read(cx).value().trim().to_lowercase();
        let Some(all) = self.franchises.ready() else { return vec![] };
        all.iter()
            .filter(|f| {
                query.is_empty()
                    || f.franchise_name.as_deref().unwrap_or("").to_lowercase().contains(&query)
                    || f.franchise_id.to_lowercase().contains(&query)
            })
            .filter(|f| match self.era {
                // A franchise belongs to a decade when its active span overlaps it.
                Some(decade) => {
                    let first = f.first_year.unwrap_or(i32::MIN);
                    let last = f.last_year.unwrap_or(i32::MAX);
                    first <= decade + 9 && last >= decade
                }
                None => true,
            })
            .cloned()
            .collect()
    }

    fn card(&self, f: &FranchiseSummary, cx: &App) -> AnyElement {
        let name = f.franchise_name.clone().unwrap_or_else(|| f.franchise_id.clone());
        let initials: String = name.split_whitespace().take(2).filter_map(|w| w.chars().next()).collect();
        let years = if f.is_active {
            format!("{} - Present", format::year(f.first_year))
        } else {
            format::year_span(f.first_year, f.last_year)
        };
        Button::new(SharedString::from(format!("franchise-{}", f.franchise_id)))
            .outline()
            .w(px(320.))
            .h_auto()
            .accessibility_label(name.clone())
            .on_click(self.nav.to(Route::Franchise { id: f.franchise_id.clone() }))
            .child(
                v_flex()
                    .w_full()
                    .p_2()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size_10()
                                    .flex_shrink_0()
                                    .rounded(cx.theme().radius)
                                    .bg(if f.is_active { widgets::brand_navy() } else { cx.theme().muted_foreground })
                                    .text_color(white())
                                    .font_bold()
                                    .child(initials),
                            )
                            .child(
                                v_flex()
                                    .min_w_0()
                                    .flex_1()
                                    .items_start()
                                    .child(div().font_semibold().truncate().child(name))
                                    .child(widgets::note(years, cx)),
                            )
                            .child(if f.is_active {
                                Tag::success().xsmall().child("Active")
                            } else {
                                Tag::secondary().xsmall().child("Defunct")
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_wrap()
                            .text_xs()
                            .items_center()
                            .child(format!("{} seasons", format::int(f.total_seasons)))
                            .child(format!("Win % {}", format::rate(f.win_pct)))
                            .when(f.world_series_wins.unwrap_or(0) > 0, |this| {
                                this.child(Tag::custom(widgets::hof_gold(), rgb(0x1a1a1a).into(), rgb(0xb8962e).into()).xsmall().child(format!("{} WS", f.world_series_wins.unwrap_or(0))))
                            })
                            .when(f.pennant_wins.unwrap_or(0) > 0, |this| {
                                this.child(Tag::primary().outline().xsmall().child(match f.pennant_wins.unwrap_or(0) {
                                    1 => "1 Pennant".to_string(),
                                    n => format!("{n} Pennants"),
                                }))
                            }),
                    ),
            )
            .into_any_element()
    }

    fn group(&self, title: &str, rows: &[FranchiseSummary], cx: &App) -> AnyElement {
        v_flex()
            .gap_3()
            .child(div().text_lg().font_semibold().child(format!("{title} ({})", rows.len())))
            .child(h_flex().gap_3().flex_wrap().children(rows.iter().map(|f| self.card(f, cx))))
            .into_any_element()
    }
}

impl Page for TeamsPage {
    fn title(&self, _: &App) -> SharedString {
        "Teams & Franchises".into()
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let league = self.league;
        self.franchises = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| api.franchises(league, false),
            |this: &mut Self, result, _| this.franchises = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for TeamsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = self.visible(cx);
        let subtitle = self
            .franchises
            .ready()
            .map(|_| format!("{} franchises", visible.len()).into());
        let league_ix = match self.league {
            Some("AL") => 1,
            Some("NL") => 2,
            _ => 0,
        };
        let league_buttons = TabBar::new("league-filter")
            .segmented()
            .small()
            .selected_index(league_ix)
            .children(["All", "AL", "NL"])
            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                let league = match ix {
                    1 => Some("AL"),
                    2 => Some("NL"),
                    _ => None,
                };
                this.set_league(league, window, cx);
            }));
        let filters = widgets::filter_bar(cx)
            .child(widgets::field("League", league_buttons, cx))
            .child(widgets::field("Name", div().w(px(260.)).child(Input::new(&self.name).small().cleanable(true)), cx))
            .child(widgets::field("Era", picker::dropdown(&self.era_picker, 160.), cx))
            .child(
                Button::new("clear-team-filters")
                    .small()
                    .ghost()
                    .label("Clear Filters")
                    .on_click(cx.listener(|this, _, window, cx| this.clear_filters(window, cx))),
            );
        let body: AnyElement = match &self.franchises {
            Loadable::Ready(_) if visible.is_empty() => {
                widgets::empty_state("No Teams Found", "Try adjusting your search criteria.", cx)
            }
            Loadable::Ready(_) => {
                let (active, historical): (Vec<_>, Vec<_>) = visible.into_iter().partition(|f| f.is_active);
                v_flex()
                    .gap_6()
                    .when(!active.is_empty(), |this| this.child(self.group("Active Franchises", &active, cx)))
                    .when(!historical.is_empty(), |this| this.child(self.group("Historical Franchises", &historical, cx)))
                    .into_any_element()
            }
            Loadable::Failed(message) => widgets::error_state(
                "teams-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            _ => widgets::loading_state("Loading franchises…", cx),
        };
        widgets::page_body("teams", &self.scroll).child(
            widgets::page_column()
                .child(widgets::page_header(None, "Teams & Franchises", subtitle, vec![], cx))
                .child(filters)
                .child(body),
        )
    }
}

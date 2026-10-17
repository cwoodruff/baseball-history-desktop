//! Hall of Fame: category cards, year and category filters, inductee table.

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Selectable as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::{HallOfFamer, Paged};
use crate::catalog;
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::ui::grid::{Col, grid};
use crate::ui::picker::{self, Opt, Picker, PickerEvent};
use crate::ui::widgets;

const PAGE_SIZE: u32 = 50;
const FIRST_CLASS: i32 = 1936;

pub struct HallOfFamePage {
    nav: Navigator,
    scroll: ScrollHandle,
    year: Option<i32>,
    category: Option<String>,
    page: u32,
    year_picker: Picker,
    category_picker: Picker,
    inductees: Loadable<Paged<HallOfFamer>>,
    /// Inductee totals: overall, then one per category card.
    totals: Loadable<(u32, Vec<u32>)>,
    _tasks: Vec<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

fn category_tag(category: Option<&str>) -> AnyElement {
    let label = format::text(category);
    match category {
        Some("Player") => Tag::primary().xsmall().child(label),
        Some("Manager") => Tag::success().xsmall().child(label),
        Some("Umpire") => Tag::warning().xsmall().child(label),
        _ => Tag::secondary().xsmall().child(label),
    }
    .into_any_element()
}

impl HallOfFamePage {
    pub fn new(
        year: Option<i32>,
        category: Option<String>,
        nav: Navigator,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let year_picker = picker::picker(
            picker::years("All Years", FIRST_CLASS, catalog::LAST_SEASON + 1),
            &year.map(|y| y.to_string()).unwrap_or_default(),
            true,
            window,
            cx,
        );
        let category_picker = picker::picker(
            std::iter::once(Opt::any("All Categories"))
                .chain(catalog::HOF_CATEGORY_VALUES.iter().map(|c| Opt::new(*c, *c)))
                .collect(),
            category.as_deref().unwrap_or(""),
            false,
            window,
            cx,
        );
        let subscriptions = vec![
            cx.subscribe_in(&year_picker, window, |this, _, event: &PickerEvent, window, cx| {
                this.year = picker::value_of(event).and_then(|v| v.parse().ok());
                this.page = 1;
                this.load_inductees(window, cx);
            }),
            cx.subscribe_in(&category_picker, window, |this, _, event: &PickerEvent, window, cx| {
                this.category = picker::value_of(event);
                this.page = 1;
                this.load_inductees(window, cx);
            }),
        ];
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            year,
            category,
            page: 1,
            year_picker,
            category_picker,
            inductees: Loadable::Idle,
            totals: Loadable::Idle,
            _tasks: Vec::new(),
            _subscriptions: subscriptions,
        };
        page.reload(window, cx);
        page
    }

    fn load_inductees(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let (year, category, page) = (self.year, self.category.clone(), self.page);
        self.inductees = Loadable::Loading;
        let task = loader::fetch(
            cx,
            move |api| api.hall_of_fame(year, category.as_deref(), page, PAGE_SIZE),
            |this: &mut Self, result, _| this.inductees = Loadable::from_result(result),
        );
        if self._tasks.is_empty() {
            self._tasks.push(task);
        } else {
            self._tasks[0] = task;
        }
        cx.notify();
    }

    fn set_category(&mut self, category: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let category = if self.category.as_deref() == category { None } else { category.map(str::to_string) };
        picker::select_value(&self.category_picker, category.as_deref().unwrap_or(""), window, cx);
        self.category = category;
        self.page = 1;
        self.load_inductees(window, cx);
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.year = None;
        self.category = None;
        self.page = 1;
        picker::select_value(&self.year_picker, "", window, cx);
        picker::select_value(&self.category_picker, "", window, cx);
        self.load_inductees(window, cx);
    }

    fn render_banner(&self, cx: &App) -> AnyElement {
        let total = self.totals.ready().map(|(t, _)| format::int(Some(*t as i64))).unwrap_or_else(|| "…".into());
        v_flex()
            .w_full()
            .p_6()
            .gap_1()
            .rounded(cx.theme().radius_lg)
            .bg(linear_gradient(
                135.,
                linear_color_stop(rgb(0xd4af37), 0.),
                linear_color_stop(rgb(0xf4d03f), 1.),
            ))
            .text_color(rgb(0x1a1a1a))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(gpui_kit::component::Icon::new(IconName::Landmark).large())
                    .child(div().text_2xl().font_bold().child("Baseball Hall of Fame")),
            )
            .child(div().child(format!("{total} inductees enshrined in Cooperstown")))
            .into_any_element()
    }

    fn render_cards(&self, cx: &mut Context<Self>) -> AnyElement {
        let counts = self.totals.ready().map(|(_, c)| c.clone());
        h_flex()
            .w_full()
            .gap_3()
            .flex_wrap()
            .children(catalog::HOF_CATEGORIES.iter().enumerate().map(|(ix, choice)| {
                let selected = self.category.as_deref() == Some(choice.value);
                let count = counts.as_ref().and_then(|c| c.get(ix)).map(|n| format::int(Some(*n as i64))).unwrap_or_else(|| "…".into());
                let value = choice.value;
                Button::new(SharedString::from(format!("hof-cat-{ix}")))
                    .outline()
                    .flex_1()
                    .min_w(px(160.))
                    .h_auto()
                    .selected(selected)
                    .accessibility_label(format!("Show {}", choice.label))
                    .on_click(cx.listener(move |this, _, window, cx| this.set_category(Some(value), window, cx)))
                    .child(
                        v_flex()
                            .w_full()
                            .py_2()
                            .items_center()
                            .child(div().text_2xl().font_bold().text_color(cx.theme().primary).child(count))
                            .child(div().text_sm().child(choice.label)),
                    )
            }))
            .into_any_element()
    }
}

impl Page for HallOfFamePage {
    fn title(&self, _: &App) -> SharedString {
        "Hall of Fame".into()
    }

    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._tasks.clear();
        self.load_inductees(window, cx);
        self.totals = Loadable::Loading;
        self._tasks.push(loader::fetch(
            cx,
            |api| {
                let total = api.hall_of_fame(None, None, 1, 1)?.total_count;
                let mut per = Vec::new();
                for choice in catalog::HOF_CATEGORIES {
                    per.push(api.hall_of_fame(None, Some(choice.value), 1, 1)?.total_count);
                }
                Ok((total, per))
            },
            |this: &mut Self, result, _| this.totals = Loadable::from_result(result),
        ));
    }
}

impl Render for HallOfFamePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let filters = widgets::filter_bar(cx)
            .child(widgets::field("Induction Year", picker::dropdown(&self.year_picker, 140.), cx))
            .child(widgets::field("Category", picker::dropdown(&self.category_picker, 200.), cx))
            .when(self.year.is_some() || self.category.is_some(), |this| {
                this.child(
                    Button::new("hof-clear")
                        .small()
                        .ghost()
                        .label("Clear Filters")
                        .on_click(cx.listener(|this, _, window, cx| this.clear(window, cx))),
                )
            });
        let nav = self.nav.clone();
        let body: AnyElement = match &self.inductees {
            Loadable::Ready(p) if p.data.is_empty() => widgets::empty_state(
                "No Inductees Found",
                "No Hall of Fame inductees match your criteria.",
                cx,
            ),
            Loadable::Ready(p) => {
                let cols = vec![
                    Col::element("Name", 240., move |h: &HallOfFamer, _| {
                        widgets::player_link(
                            SharedString::from(format!("hof-row-{}", h.player_id)),
                            &h.player_id,
                            format::full_name(h.first_name.as_deref(), h.last_name.as_deref()),
                            true,
                            &nav,
                        )
                    }),
                    Col::text("Year", 64., |h: &HallOfFamer| format::year(h.induction_year)),
                    Col::element("Category", 150., |h: &HallOfFamer, _| category_tag(h.category.as_deref())),
                    Col::text("Voted By", 260., |h: &HallOfFamer| format::text(h.voted_by.as_deref())),
                    Col::num("Vote %", 80., |h: &HallOfFamer| format::percent(h.vote_percentage)),
                    Col::text("Career", 120., |h: &HallOfFamer| format::year_span(h.debut_year, h.final_year)),
                ];
                let this = cx.entity().downgrade();
                v_flex()
                    .gap_3()
                    .child(widgets::section_with("Inductees", widgets::note(format!("{} inductees", format::int(Some(p.total_count as i64))), cx), cx).child(grid("hof-table", cols, &p.data, cx)))
                    .child(widgets::pager(
                        "hof-pager",
                        p.page,
                        p.total_pages,
                        p.total_count,
                        move |page, window, cx| {
                            this.update(cx, |this, cx| {
                                this.page = page;
                                this.scroll.set_offset(point(px(0.), px(0.)));
                                this.load_inductees(window, cx);
                            })
                            .ok();
                        },
                        cx,
                    ))
                    .into_any_element()
            }
            Loadable::Failed(message) => widgets::error_state(
                "hof-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            _ => widgets::loading_state("Loading inductees…", cx),
        };
        widgets::page_body("hall-of-fame", &self.scroll).child(
            widgets::page_column()
                .child(self.render_banner(cx))
                .child(self.render_cards(cx))
                .child(filters)
                .child(body),
        )
    }
}

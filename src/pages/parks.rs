//! Ballparks: every park, filtered by state (API) and by name or city (locally).

use gpui_kit::component::{
    Sizable as _,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    v_flex,
};
use gpui_kit::*;

use crate::api::models::ParkSummary;
use crate::catalog;
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::grid::{Col, grid};
use crate::ui::picker::{self, Opt, Picker, PickerEvent};
use crate::ui::widgets;

/// The API's maximum page size; the full list is four pages.
const FETCH_SIZE: u32 = 100;
const PAGE_SIZE: usize = 30;

pub struct ParksPage {
    nav: Navigator,
    scroll: ScrollHandle,
    state: Option<String>,
    page: u32,
    name: Entity<InputState>,
    state_picker: Picker,
    parks: Loadable<Vec<ParkSummary>>,
    _task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl ParksPage {
    pub fn new(nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Filter by name or city…"));
        let state_picker = picker::picker(
            std::iter::once(Opt::any("All States"))
                .chain(catalog::PARK_STATES.iter().map(|s| Opt::new(*s, *s)))
                .collect(),
            "",
            true,
            window,
            cx,
        );
        let subscriptions = vec![
            cx.subscribe_in(&name, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.page = 1;
                    cx.notify();
                }
            }),
            cx.subscribe_in(&state_picker, window, |this, _, event: &PickerEvent, window, cx| {
                this.state = picker::value_of(event);
                this.page = 1;
                this.reload(window, cx);
            }),
        ];
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            state: None,
            page: 1,
            name,
            state_picker,
            parks: Loadable::Idle,
            _task: None,
            _subscriptions: subscriptions,
        };
        page.reload(window, cx);
        page
    }

    fn visible(&self, cx: &App) -> Vec<ParkSummary> {
        let query = self.name.read(cx).value().trim().to_lowercase();
        let Some(all) = self.parks.ready() else { return vec![] };
        all.iter()
            .filter(|p| {
                query.is_empty()
                    || p.park_name.as_deref().unwrap_or("").to_lowercase().contains(&query)
                    || p.city.as_deref().unwrap_or("").to_lowercase().contains(&query)
            })
            .cloned()
            .collect()
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.name.update(cx, |state, cx| state.set_value("", window, cx));
        picker::select_value(&self.state_picker, "", window, cx);
        self.state = None;
        self.page = 1;
        self.reload(window, cx);
    }
}

impl Page for ParksPage {
    fn title(&self, _: &App) -> SharedString {
        "Ballparks".into()
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let state = self.state.clone();
        self.parks = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| {
                let mut all = Vec::new();
                let mut page = 1;
                loop {
                    let result = api.parks(state.as_deref(), page, FETCH_SIZE)?;
                    let done = page >= result.total_pages;
                    all.extend(result.data);
                    if done {
                        break;
                    }
                    page += 1;
                }
                all.sort_by(|a, b| a.park_name.cmp(&b.park_name));
                Ok(all)
            },
            |this: &mut Self, result, _| this.parks = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for ParksPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = self.visible(cx);
        let total = visible.len();
        let total_pages = total.div_ceil(PAGE_SIZE) as u32;
        let page = self.page.min(total_pages.max(1));
        let start = (page as usize - 1) * PAGE_SIZE;
        let rows: Vec<ParkSummary> = visible.into_iter().skip(start).take(PAGE_SIZE).collect();
        let subtitle = self.parks.ready().map(|_| format!("{} ballparks", format::int(Some(total as i64))).into());

        let filters = widgets::filter_bar(cx)
            .child(widgets::field("Name or City", div().w(px(280.)).child(Input::new(&self.name).small().cleanable(true)), cx))
            .child(widgets::field("State", picker::dropdown(&self.state_picker, 180.), cx))
            .child(
                Button::new("parks-clear")
                    .small()
                    .ghost()
                    .label("Clear Filters")
                    .on_click(cx.listener(|this, _, window, cx| this.clear(window, cx))),
            );
        let nav = self.nav.clone();
        let body: AnyElement = match &self.parks {
            Loadable::Ready(_) if rows.is_empty() => {
                widgets::empty_state("No Ballparks Found", "Try a different name, city or state.", cx)
            }
            Loadable::Ready(_) => {
                let cols = vec![
                    Col::element("Ballpark", 340., move |p: &ParkSummary, _| {
                        nav.button(
                            SharedString::from(format!("park-{}", p.park_key)),
                            format::text(p.park_name.as_deref()),
                            Route::Park { key: p.park_key.clone() },
                        )
                    }),
                    Col::text("Location", 280., |p: &ParkSummary| {
                        [p.city.as_deref(), p.state.as_deref(), p.country.as_deref()]
                            .iter()
                            .flatten()
                            .filter(|s| !s.is_empty())
                            .copied()
                            .collect::<Vec<_>>()
                            .join(", ")
                    }),
                    Col::text("Key", 96., |p: &ParkSummary| p.park_key.clone()),
                ];
                let this = cx.entity().downgrade();
                v_flex()
                    .gap_3()
                    .child(widgets::section("Ballparks", cx).child(grid("parks-table", cols, &rows, cx)))
                    .child(widgets::pager(
                        "parks-pager",
                        page,
                        total_pages,
                        total as u32,
                        move |page, _, cx| {
                            this.update(cx, |this, cx| {
                                this.page = page;
                                this.scroll.set_offset(point(px(0.), px(0.)));
                                cx.notify();
                            })
                            .ok();
                        },
                        cx,
                    ))
                    .into_any_element()
            }
            Loadable::Failed(message) => widgets::error_state(
                "parks-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            _ => widgets::loading_state("Loading ballparks…", cx),
        };
        widgets::page_body("parks", &self.scroll).child(
            widgets::page_column()
                .child(widgets::page_header(None, "Ballparks", subtitle, vec![], cx))
                .child(filters)
                .child(body),
        )
    }
}

//! Players: the A–Z browser with name, position, era and sort filters,
//! 48 player cards per page.

use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Selectable as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::PlayerQuery;
use crate::api::models::{Paged, PlayerSummary};
use crate::catalog;
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::picker::{self, Opt, Picker, PickerEvent};
use crate::ui::widgets;

const PAGE_SIZE: u32 = 48;

const POSITIONS: &[(&str, &str)] = &[
    ("P", "Pitcher"),
    ("C", "Catcher"),
    ("1B", "First Base"),
    ("2B", "Second Base"),
    ("3B", "Third Base"),
    ("SS", "Shortstop"),
    ("OF", "Outfield"),
    ("DH", "Designated Hitter"),
];

const SORTS: &[(&str, &str)] = &[
    ("name", "Sort: Name"),
    ("hr", "Sort: Home Runs"),
    ("hits", "Sort: Hits"),
    ("games", "Sort: Games"),
];

pub struct PlayersPage {
    nav: Navigator,
    scroll: ScrollHandle,
    request: PlayerQuery,
    name: Entity<InputState>,
    position_picker: Picker,
    era_picker: Picker,
    sort_picker: Picker,
    players: Loadable<Paged<PlayerSummary>>,
    _task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl PlayersPage {
    pub fn new(letter: char, nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Filter by name…"));
        let positions = std::iter::once(Opt::any("All Positions"))
            .chain(POSITIONS.iter().map(|(v, l)| Opt::new(*v, *l)))
            .collect();
        let eras = std::iter::once(Opt::any("All Eras"))
            .chain(
                catalog::decades()
                    .into_iter()
                    .rev()
                    .map(|d| Opt::new(d.to_string(), format!("{d}s"))),
            )
            .collect();
        let sorts = SORTS.iter().map(|(v, l)| Opt::new(*v, *l)).collect();
        let position_picker = picker::picker(positions, "", false, window, cx);
        let era_picker = picker::picker(eras, "", false, window, cx);
        let sort_picker = picker::picker(sorts, "name", false, window, cx);
        let subscriptions = vec![
            cx.subscribe_in(&name, window, |this, state, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let query = state.read(cx).value().trim().to_string();
                    this.query_changed(query, cx);
                }
            }),
            cx.subscribe_in(
                &position_picker,
                window,
                |this, _, event: &PickerEvent, window, cx| {
                    this.request.pos = picker::value_of(event);
                    this.refilter(window, cx);
                },
            ),
            cx.subscribe_in(
                &era_picker,
                window,
                |this, _, event: &PickerEvent, window, cx| {
                    this.request.era = picker::value_of(event).and_then(|v| v.parse().ok());
                    this.refilter(window, cx);
                },
            ),
            cx.subscribe_in(
                &sort_picker,
                window,
                |this, _, event: &PickerEvent, window, cx| {
                    // The sort list has no "All" entry; `value_of` maps "" to None.
                    this.request.sort = picker::value_of(event).unwrap_or_else(|| "name".into());
                    this.refilter(window, cx);
                },
            ),
        ];
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            request: PlayerQuery::letter(letter.to_ascii_uppercase(), 1, PAGE_SIZE),
            name,
            position_picker,
            era_picker,
            sort_picker,
            players: Loadable::Idle,
            _task: None,
            _subscriptions: subscriptions,
        };
        page.reload(window, cx);
        page
    }

    fn refilter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.request.page = 1;
        self.reload(window, cx);
    }

    /// Debounces the name filter like the web (300 ms), then re-queries.
    fn query_changed(&mut self, query: String, cx: &mut Context<Self>) {
        let q = (!query.is_empty()).then_some(query);
        if q == self.request.q {
            return;
        }
        self.request.q = q;
        self.request.page = 1;
        self.players = Loadable::Loading;
        self._task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(300))
                .await;
            let Ok(task) = this.update(cx, |this, cx| this.fetch(cx)) else {
                return;
            };
            task.await;
        }));
        cx.notify();
    }

    fn fetch(&mut self, cx: &mut Context<Self>) -> Task<()> {
        let request = self.request.clone();
        loader::fetch(
            cx,
            move |api| api.players(&request),
            |this: &mut Self, result, _| this.players = Loadable::from_result(result),
        )
    }

    fn has_filters(&self) -> bool {
        self.request.q.is_some()
            || self.request.pos.is_some()
            || self.request.era.is_some()
            || self.request.sort != "name"
    }

    fn clear_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_name(window, cx);
        picker::select_value(&self.position_picker, "", window, cx);
        picker::select_value(&self.era_picker, "", window, cx);
        picker::select_value(&self.sort_picker, "name", window, cx);
        self.request = PlayerQuery::letter(self.request.letter, 1, PAGE_SIZE);
        self.reload(window, cx);
    }

    /// Empties the name box without triggering its debounced query.
    fn clear_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.request.q = None;
        self.name
            .update(cx, |state, cx| state.set_value("", window, cx));
    }

    /// Picking a letter keeps the other filters but drops the name query.
    fn set_letter(&mut self, letter: char, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_name(window, cx);
        self.request.letter = letter;
        self.request.page = 1;
        self.reload(window, cx);
    }

    fn set_page(&mut self, page: u32, window: &mut Window, cx: &mut Context<Self>) {
        self.request.page = page;
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.reload(window, cx);
    }

    fn render_filters(&self, cx: &mut Context<Self>) -> AnyElement {
        widgets::filter_bar(cx)
            .child(widgets::field(
                "Name",
                div()
                    .w(px(220.))
                    .child(Input::new(&self.name).small().cleanable(true)),
                cx,
            ))
            .child(widgets::field(
                "Position",
                picker::dropdown(&self.position_picker, 180.),
                cx,
            ))
            .child(widgets::field(
                "Era",
                picker::dropdown(&self.era_picker, 130.),
                cx,
            ))
            .child(widgets::field(
                "Sort by",
                picker::dropdown(&self.sort_picker, 170.),
                cx,
            ))
            .when(self.has_filters(), |this| {
                this.child(
                    Button::new("players-clear")
                        .small()
                        .ghost()
                        .label("Clear Filters")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.clear_filters(window, cx)),
                        ),
                )
            })
            .into_any_element()
    }

    fn render_letters(&self, cx: &mut Context<Self>) -> AnyElement {
        // While a name query searches every letter, none is highlighted.
        let current = self.request.q.is_none().then_some(self.request.letter);
        h_flex()
            .w_full()
            .gap_1()
            .flex_wrap()
            .children(('A'..='Z').map(|letter| {
                Button::new(SharedString::from(format!("letter-{letter}")))
                    .small()
                    .compact()
                    .w(px(32.))
                    .when(Some(letter) == current, |this| this.primary())
                    .when(Some(letter) != current, |this| this.ghost())
                    .selected(Some(letter) == current)
                    .label(letter.to_string())
                    .accessibility_label(format!("Players whose last name starts with {letter}"))
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.set_letter(letter, window, cx)),
                    )
            }))
            .into_any_element()
    }

    fn render_card(&self, player: &PlayerSummary, cx: &App) -> AnyElement {
        let name = format::full_name(player.first_name.as_deref(), player.last_name.as_deref());
        let initials: String = [player.first_name.as_deref(), player.last_name.as_deref()]
            .iter()
            .filter_map(|s| s.and_then(|s| s.chars().next()))
            .collect();
        let years = match (player.debut_year, player.final_year) {
            (Some(d), Some(f)) if f >= crate::catalog::LAST_SEASON => format!("{d} - Present"),
            (Some(d), None) => format!("{d} - Present"),
            (d, f) => format::year_span(d, f),
        };
        let stat = |label: &str, value: Option<i64>| {
            v_flex()
                .items_center()
                .child(div().font_bold().child(format::int(value)))
                .child(widgets::note(label.to_string(), cx))
        };
        Button::new(SharedString::from(format!(
            "player-card-{}",
            player.player_id
        )))
        .outline()
        .w(px(232.))
        .h_auto()
        .accessibility_label(name.clone())
        .on_click(self.nav.to(Route::Player {
            id: player.player_id.clone(),
        }))
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
                                .size_9()
                                .flex_shrink_0()
                                .rounded_full()
                                .bg(widgets::brand_navy())
                                .text_color(white())
                                .text_sm()
                                .font_bold()
                                .child(initials),
                        )
                        .child(
                            v_flex()
                                .min_w_0()
                                .items_start()
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .items_center()
                                        .child(div().font_semibold().truncate().child(name))
                                        .when(player.is_partial_record, |this| {
                                            this.child(widgets::partial_dagger(cx))
                                        })
                                        .when(player.is_hall_of_famer, |this| {
                                            this.child(widgets::hof_tag())
                                        }),
                                )
                                .child(widgets::note(years, cx)),
                        ),
                )
                .child(
                    h_flex()
                        .w_full()
                        .justify_around()
                        .pt_1()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .child(stat("H", player.total_hits))
                        .child(stat("HR", player.total_home_runs))
                        .child(stat("G", player.total_games)),
                ),
        )
        .into_any_element()
    }
}

impl Page for PlayersPage {
    fn title(&self, _: &App) -> SharedString {
        match &self.request.q {
            Some(_) => "Players".into(),
            None => format!("Players — {}", self.request.letter).into(),
        }
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.players = Loadable::Loading;
        self._task = Some(self.fetch(cx));
        cx.notify();
    }
}

impl Render for PlayersPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let letter = self.request.letter;
        let searching = self.request.q.is_some();
        let subtitle = match &self.players {
            Loadable::Ready(p) => {
                let count = format::int(Some(p.total_count as i64));
                Some(
                    if searching {
                        format!("{count} players")
                    } else {
                        format!("{count} players with last names starting with {letter}")
                    }
                    .into(),
                )
            }
            _ => None,
        };
        let header = widgets::page_header(None, "Players", subtitle, vec![], cx);
        let filters = self.render_filters(cx);
        let letters = self.render_letters(cx);
        let empty_message = if searching {
            "Try adjusting your search criteria.".to_string()
        } else {
            format!("No players found with last names starting with '{letter}'.")
        };
        let body: AnyElement = match &self.players {
            Loadable::Ready(p) if p.data.is_empty() => {
                widgets::empty_state("No Players Found", &empty_message, cx)
            }
            Loadable::Ready(p) => v_flex()
                .gap_4()
                .child(
                    h_flex()
                        .w_full()
                        .gap_3()
                        .flex_wrap()
                        .children(p.data.iter().map(|player| self.render_card(player, cx))),
                )
                .child(widgets::pager(
                    "players-pager",
                    p.page,
                    p.total_pages,
                    p.total_count,
                    {
                        let this = cx.entity().downgrade();
                        move |page, window, cx| {
                            this.update(cx, |this, cx| this.set_page(page, window, cx))
                                .ok();
                        }
                    },
                    cx,
                ))
                .child(widgets::note(
                    "† Partial record: only part of this player's name survived.",
                    cx,
                ))
                .into_any_element(),
            Loadable::Failed(message) => widgets::error_state(
                "players-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            _ => widgets::loading_state("Loading players…", cx),
        };
        widgets::page_body("players", &self.scroll).child(
            widgets::page_column()
                .child(header)
                .child(filters)
                .child(letters)
                .child(body),
        )
    }
}

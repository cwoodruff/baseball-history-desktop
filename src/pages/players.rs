//! Players: the A–Z browser, 48 player cards per page.

use gpui_kit::component::{
    ActiveTheme as _, Selectable as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::{Paged, PlayerSummary};
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::widgets;

const PAGE_SIZE: u32 = 48;

pub struct PlayersPage {
    nav: Navigator,
    scroll: ScrollHandle,
    letter: char,
    page: u32,
    players: Loadable<Paged<PlayerSummary>>,
    _task: Option<Task<()>>,
}

impl PlayersPage {
    pub fn new(letter: char, nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            letter: letter.to_ascii_uppercase(),
            page: 1,
            players: Loadable::Idle,
            _task: None,
        };
        page.reload(window, cx);
        page
    }

    fn set_letter(&mut self, letter: char, window: &mut Window, cx: &mut Context<Self>) {
        self.letter = letter;
        self.page = 1;
        self.reload(window, cx);
    }

    fn set_page(&mut self, page: u32, window: &mut Window, cx: &mut Context<Self>) {
        self.page = page;
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.reload(window, cx);
    }

    fn render_letters(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .gap_1()
            .flex_wrap()
            .children(('A'..='Z').map(|letter| {
                Button::new(SharedString::from(format!("letter-{letter}")))
                    .small()
                    .compact()
                    .w(px(32.))
                    .when(letter == self.letter, |this| this.primary())
                    .when(letter != self.letter, |this| this.ghost())
                    .selected(letter == self.letter)
                    .label(letter.to_string())
                    .accessibility_label(format!("Players whose last name starts with {letter}"))
                    .on_click(cx.listener(move |this, _, window, cx| this.set_letter(letter, window, cx)))
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
        Button::new(SharedString::from(format!("player-card-{}", player.player_id)))
            .outline()
            .w(px(232.))
            .h_auto()
            .accessibility_label(name.clone())
            .on_click(self.nav.to(Route::Player { id: player.player_id.clone() }))
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
                                            .when(player.is_hall_of_famer, |this| this.child(widgets::hof_tag())),
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
        format!("Players — {}", self.letter).into()
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let (letter, page) = (self.letter, self.page);
        self.players = Loadable::Loading;
        self._task = Some(loader::fetch(
            cx,
            move |api| api.players(Some(letter), page, PAGE_SIZE),
            |this: &mut Self, result, _| this.players = Loadable::from_result(result),
        ));
        cx.notify();
    }
}

impl Render for PlayersPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let subtitle = match &self.players {
            Loadable::Ready(p) => Some(
                format!(
                    "{} players with last names starting with {}",
                    format::int(Some(p.total_count as i64)),
                    self.letter
                )
                .into(),
            ),
            _ => None,
        };
        let header = widgets::page_header(None, "Players", subtitle, vec![], cx);
        let letters = self.render_letters(cx);
        let body: AnyElement = match &self.players {
            Loadable::Ready(p) if p.data.is_empty() => widgets::empty_state(
                "No Players Found",
                "No players have a last name starting with this letter.",
                cx,
            ),
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
                            this.update(cx, |this, cx| this.set_page(page, window, cx)).ok();
                        }
                    },
                    cx,
                ))
                .child(widgets::note(
                    "† Partial record: only part of this player's name survived. Use Search to find a player by name.",
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
                .child(letters)
                .child(body),
        )
    }
}

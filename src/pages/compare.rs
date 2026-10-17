//! Compare two players side by side.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::{PlayerAward, PlayerDetail, SearchResults};
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::Route;
use crate::ui::widgets;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Better {
    Left,
    Right,
    Neither,
}

struct Slot {
    input: Entity<InputState>,
    results: Loadable<SearchResults>,
    player_id: Option<String>,
    player: Loadable<PlayerDetail>,
    awards: Loadable<Vec<PlayerAward>>,
    _tasks: Vec<Task<()>>,
}

pub struct ComparePage {
    nav: Navigator,
    scroll: ScrollHandle,
    slots: [Slot; 2],
    _subscriptions: Vec<Subscription>,
}

/// One comparison row: values on each side and which one is better.
struct Row {
    label: &'static str,
    left: String,
    right: String,
    better: Better,
}

fn compare_num(
    label: &'static str,
    l: Option<f64>,
    r: Option<f64>,
    fmt: impl Fn(Option<f64>) -> String,
    lower_is_better: bool,
) -> Row {
    let better = match (l, r) {
        (Some(a), Some(b)) if (a - b).abs() > f64::EPSILON => {
            if (a > b) != lower_is_better {
                Better::Left
            } else {
                Better::Right
            }
        }
        _ => Better::Neither,
    };
    Row {
        label,
        left: fmt(l),
        right: fmt(r),
        better,
    }
}

fn count(v: Option<i64>) -> Option<f64> {
    v.map(|v| v as f64)
}

fn int_fmt(v: Option<f64>) -> String {
    format::int(v.map(|v| v as i64))
}

impl ComparePage {
    pub fn new(
        first: Option<String>,
        second: Option<String>,
        nav: Navigator,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let make_input =
            |placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
                cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
            };
        let inputs = [
            make_input("Search for the first player…", window, cx),
            make_input("Search for the second player…", window, cx),
        ];
        let subscriptions = inputs
            .iter()
            .enumerate()
            .map(|(ix, input)| {
                cx.subscribe_in(
                    input,
                    window,
                    move |this, state, event: &InputEvent, _, cx| {
                        if matches!(event, InputEvent::Change) {
                            let query = state.read(cx).value().trim().to_string();
                            this.search(ix, query, cx);
                        }
                    },
                )
            })
            .collect();
        let slot = |input: Entity<InputState>| Slot {
            input,
            results: Loadable::Idle,
            player_id: None,
            player: Loadable::Idle,
            awards: Loadable::Idle,
            _tasks: Vec::new(),
        };
        let [a, b] = inputs;
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            slots: [slot(a), slot(b)],
            _subscriptions: subscriptions,
        };
        if let Some(id) = first {
            page.select(0, id, cx);
        }
        if let Some(id) = second {
            page.select(1, id, cx);
        }
        page
    }

    fn search(&mut self, ix: usize, query: String, cx: &mut Context<Self>) {
        let slot = &mut self.slots[ix];
        if query.chars().count() < 2 {
            slot.results = Loadable::Idle;
            slot._tasks.clear();
            cx.notify();
            return;
        }
        slot.results = Loadable::Loading;
        slot._tasks = vec![cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(300))
                .await;
            let Ok(task) = this.update(cx, |_, cx| {
                loader::fetch(
                    cx,
                    move |api| api.search(&query, 8),
                    move |this: &mut Self, result, _| {
                        this.slots[ix].results = Loadable::from_result(result)
                    },
                )
            }) else {
                return;
            };
            task.await;
        })];
        cx.notify();
    }

    fn select(&mut self, ix: usize, id: String, cx: &mut Context<Self>) {
        let slot = &mut self.slots[ix];
        slot.player_id = Some(id.clone());
        slot.player = Loadable::Loading;
        slot.awards = Loadable::Loading;
        let awards_id = id.clone();
        slot._tasks = vec![
            loader::fetch(
                cx,
                move |api| api.player(&id),
                move |this: &mut Self, result, _| {
                    this.slots[ix].player = Loadable::from_result(result)
                },
            ),
            loader::fetch(
                cx,
                move |api| api.player_awards(&awards_id),
                move |this: &mut Self, result, _| {
                    this.slots[ix].awards = Loadable::from_result(result)
                },
            ),
        ];
        cx.notify();
    }

    fn clear_slot(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let slot = &mut self.slots[ix];
        slot.player_id = None;
        slot.player = Loadable::Idle;
        slot.awards = Loadable::Idle;
        slot.results = Loadable::Idle;
        slot._tasks.clear();
        let input = slot.input.clone();
        input.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    fn render_slot(&self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let slot = &self.slots[ix];
        let accent = if ix == 0 {
            widgets::brand_navy()
        } else {
            widgets::brand_red()
        };
        let heading = if ix == 0 { "Player 1" } else { "Player 2" };
        let header = h_flex()
            .px_4()
            .py_2()
            .bg(accent)
            .text_color(white())
            .font_semibold()
            .text_sm()
            .child(heading);
        let body: AnyElement = match (&slot.player_id, &slot.player) {
            (Some(_), Loadable::Ready(p)) => {
                let name = format::full_name(p.first_name.as_deref(), p.last_name.as_deref());
                v_flex()
                    .p_4()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .flex_wrap()
                            .child(div().text_xl().font_bold().child(name))
                            .when(p.is_hall_of_famer, |this| {
                                this.child(widgets::hof_year_tag(p.hof_induction_year))
                            })
                            .when(p.is_partial_record, |this| {
                                this.child(widgets::partial_record_chip(
                                    SharedString::from(format!("cmp-partial-{ix}")),
                                    &self.nav,
                                ))
                            }),
                    )
                    .child(widgets::note(
                        format!(
                            "{} · {}",
                            format::year_span(p.debut_year, p.final_year),
                            p.teams
                                .iter()
                                .filter_map(|t| t.team_name.clone())
                                .take(3)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        cx,
                    ))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new(SharedString::from(format!("cmp-open-{ix}")))
                                    .small()
                                    .outline()
                                    .label("Open player")
                                    .on_click(self.nav.to(Route::Player {
                                        id: p.player_id.clone(),
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("cmp-change-{ix}")))
                                    .small()
                                    .ghost()
                                    .label("Change")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.clear_slot(ix, window, cx)
                                    })),
                            ),
                    )
                    .into_any_element()
            }
            (Some(_), Loadable::Failed(message)) => widgets::error_state(
                SharedString::from(format!("cmp-error-{ix}")),
                message,
                cx.listener(move |this, _, window, cx| this.clear_slot(ix, window, cx)),
            ),
            (Some(_), _) => widgets::loading_state("Loading player…", cx),
            (None, _) => {
                let results: AnyElement = match &slot.results {
                    Loadable::Ready(r) if r.players.is_empty() => {
                        widgets::note("No players found. Try just a last name.", cx)
                    }
                    Loadable::Ready(r) => v_flex()
                        .gap_0p5()
                        .children(r.players.iter().map(|p| {
                            let id = p.player_id.clone();
                            Button::new(SharedString::from(format!(
                                "cmp-pick-{ix}-{}",
                                p.player_id
                            )))
                            .ghost()
                            .small()
                            .w_full()
                            .justify_start()
                            .label(format!(
                                "{}  ·  {}",
                                p.full_name.clone().unwrap_or_else(|| p.player_id.clone()),
                                format::year_span(p.debut_year, p.final_year)
                            ))
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.select(ix, id.clone(), cx)),
                            )
                        }))
                        .into_any_element(),
                    Loadable::Loading => widgets::loading_state("Searching…", cx),
                    Loadable::Failed(message) => widgets::note(message.clone(), cx),
                    Loadable::Idle => {
                        widgets::note("Type at least two characters of a player's name.", cx)
                    }
                };
                v_flex()
                    .p_4()
                    .gap_3()
                    .child(
                        Input::new(&slot.input)
                            .cleanable(true)
                            .prefix(gpui_kit::component::Icon::new(IconName::Search).small()),
                    )
                    .child(results)
                    .into_any_element()
            }
        };
        v_flex()
            .flex_1()
            .min_w(px(320.))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius_lg)
            .overflow_hidden()
            .child(header)
            .child(body)
            .into_any_element()
    }

    fn mirror(&self, title: &str, rows: Vec<Row>, cx: &App) -> AnyElement {
        let better_bg = cx.theme().success.opacity(0.15);
        let better_fg = cx.theme().success;
        let mono = cx.theme().mono_font_family.clone();
        let cell = |value: String, better: bool, align_right: bool| {
            div()
                .flex_1()
                .px_3()
                .py_1p5()
                .font_family(mono.clone())
                .when(align_right, |this| this.text_right())
                .when(better, |this| {
                    this.bg(better_bg).text_color(better_fg).font_bold()
                })
                .child(value)
        };
        widgets::section(title.to_string(), cx)
            .child(
                v_flex()
                    .text_sm()
                    .children(rows.into_iter().enumerate().map(|(i, row)| {
                        h_flex()
                            .w_full()
                            .when(i > 0, |this| {
                                this.border_t_1().border_color(cx.theme().border)
                            })
                            .child(cell(row.left, row.better == Better::Left, true))
                            .child(
                                div()
                                    .w(px(140.))
                                    .flex_shrink_0()
                                    .py_1p5()
                                    .text_center()
                                    .font_semibold()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(row.label),
                            )
                            .child(cell(row.right, row.better == Better::Right, false))
                    })),
            )
            .into_any_element()
    }

    fn render_comparison(&self, cx: &App) -> Option<AnyElement> {
        let (a, b) = (self.slots[0].player.ready()?, self.slots[1].player.ready()?);
        let empty = Vec::new();
        let (aw, bw) = (
            self.slots[0].awards.ready().unwrap_or(&empty),
            self.slots[1].awards.ready().unwrap_or(&empty),
        );
        let award_count = |awards: &[PlayerAward], name: &str| {
            awards.iter().filter(|x| x.award_id == name).count() as f64
        };
        let mut honors = vec![compare_num(
            "Total Awards",
            Some(aw.len() as f64),
            Some(bw.len() as f64),
            int_fmt,
            false,
        )];
        for (label, award) in [
            ("MVP", "Most Valuable Player"),
            ("Cy Young", "Cy Young Award"),
            ("Rookie of Year", "Rookie of the Year"),
            ("Gold Glove", "Gold Glove"),
            ("Silver Slugger", "Silver Slugger"),
        ] {
            honors.push(compare_num(
                label,
                Some(award_count(aw, award)),
                Some(award_count(bw, award)),
                int_fmt,
                false,
            ));
        }
        honors.push(Row {
            label: "Hall of Fame",
            left: if a.is_hall_of_famer {
                format!("Yes ({})", format::year(a.hof_induction_year))
            } else {
                "No".into()
            },
            right: if b.is_hall_of_famer {
                format!("Yes ({})", format::year(b.hof_induction_year))
            } else {
                "No".into()
            },
            better: match (a.is_hall_of_famer, b.is_hall_of_famer) {
                (true, false) => Better::Left,
                (false, true) => Better::Right,
                _ => Better::Neither,
            },
        });

        let mut sections = vec![self.mirror("Awards & Honors", honors, cx)];
        let (ab, bb) = (a.career_batting.as_ref(), b.career_batting.as_ref());
        if ab.is_some_and(|x| x.at_bats.unwrap_or(0) > 0)
            || bb.is_some_and(|x| x.at_bats.unwrap_or(0) > 0)
        {
            macro_rules! bat {
                ($label:expr, $field:ident) => {
                    compare_num(
                        $label,
                        count(ab.and_then(|x| x.$field)),
                        count(bb.and_then(|x| x.$field)),
                        int_fmt,
                        false,
                    )
                };
            }
            macro_rules! rate {
                ($label:expr, $field:ident) => {
                    compare_num(
                        $label,
                        ab.and_then(|x| x.$field),
                        bb.and_then(|x| x.$field),
                        format::rate,
                        false,
                    )
                };
            }
            sections.push(self.mirror(
                "Career Batting",
                vec![
                    bat!("G", games),
                    bat!("AB", at_bats),
                    bat!("H", hits),
                    bat!("2B", doubles),
                    bat!("3B", triples),
                    bat!("HR", home_runs),
                    bat!("RBI", rbi),
                    bat!("R", runs),
                    bat!("SB", stolen_bases),
                    bat!("BB", walks),
                    rate!("AVG", batting_average),
                    rate!("OBP", obp),
                    rate!("SLG", slg),
                    rate!("OPS", ops),
                ],
                cx,
            ));
        }
        let (ap, bp) = (a.career_pitching.as_ref(), b.career_pitching.as_ref());
        if ap.is_some_and(|x| x.innings_pitched.unwrap_or(0.) > 0.)
            || bp.is_some_and(|x| x.innings_pitched.unwrap_or(0.) > 0.)
        {
            macro_rules! pit {
                ($label:expr, $field:ident, $lower:expr) => {
                    compare_num(
                        $label,
                        count(ap.and_then(|x| x.$field)),
                        count(bp.and_then(|x| x.$field)),
                        int_fmt,
                        $lower,
                    )
                };
            }
            sections.push(self.mirror(
                "Career Pitching",
                vec![
                    pit!("W", wins, false),
                    pit!("L", losses, true),
                    compare_num(
                        "ERA",
                        ap.and_then(|x| x.era),
                        bp.and_then(|x| x.era),
                        format::decimal2,
                        true,
                    ),
                    pit!("G", games, false),
                    pit!("GS", games_started, false),
                    pit!("SV", saves, false),
                    pit!("CG", complete_games, false),
                    pit!("SHO", shutouts, false),
                    compare_num(
                        "IP",
                        ap.and_then(|x| x.innings_pitched),
                        bp.and_then(|x| x.innings_pitched),
                        format::innings,
                        false,
                    ),
                    pit!("SO", strikeouts, false),
                    compare_num(
                        "WHIP",
                        ap.and_then(|x| x.whip),
                        bp.and_then(|x| x.whip),
                        format::decimal2,
                        true,
                    ),
                ],
                cx,
            ));
        }
        Some(
            v_flex()
                .gap_4()
                .children(sections)
                .child(widgets::note("Green marks the better career value. Raw totals, not adjusted for era or park.", cx))
                .into_any_element(),
        )
    }
}

impl Page for ComparePage {
    fn title(&self, _: &App) -> SharedString {
        let names: Vec<String> = self
            .slots
            .iter()
            .filter_map(|s| s.player.ready())
            .map(|p| format::full_name(p.first_name.as_deref(), p.last_name.as_deref()))
            .collect();
        match names.as_slice() {
            [a, b] => format!("{a} vs. {b}").into(),
            _ => "Compare Players".into(),
        }
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        for ix in 0..2 {
            if let Some(id) = self.slots[ix].player_id.clone() {
                self.select(ix, id, cx);
            }
        }
    }
}

impl Render for ComparePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let left = self.render_slot(0, cx);
        let right = self.render_slot(1, cx);
        let comparison = self.render_comparison(cx).unwrap_or_else(|| {
            div()
                .child(widgets::note(
                    "Choose two players to compare their careers.",
                    cx,
                ))
                .into_any_element()
        });
        widgets::page_body("compare", &self.scroll).child(
            widgets::page_column()
                .child(widgets::page_header(
                    Some("Tools"),
                    "Compare Players",
                    Some("Put two careers side by side.".into()),
                    vec![],
                    cx,
                ))
                .child(
                    h_flex()
                        .w_full()
                        .gap_4()
                        .items_start()
                        .flex_wrap()
                        .child(left)
                        .child(right),
                )
                .child(comparison),
        )
    }
}

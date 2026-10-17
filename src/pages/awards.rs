//! Awards & Voting: award winners with filters, and the voting race for one award.

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    progress::Progress,
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::api::models::{AwardVote, AwardVoting, AwardWinner, Paged};
use crate::catalog;
use crate::format;
use crate::loader::{self, Loadable};
use crate::nav::Navigator;
use crate::pages::Page;
use crate::ui::grid::{Col, grid_with_row_style};
use crate::ui::picker::{self, Opt, Picker, PickerEvent};
use crate::ui::widgets;

const PAGE_SIZE: u32 = 50;
const YEARS_SHOWN: i32 = 60;

pub struct AwardsPage {
    nav: Navigator,
    scroll: ScrollHandle,
    award: Option<String>,
    year: Option<i32>,
    league: Option<String>,
    page: u32,
    award_picker: Picker,
    year_picker: Picker,
    league_picker: Picker,
    winners: Loadable<Paged<AwardWinner>>,
    voting: Loadable<Option<AwardVoting>>,
    _winners_task: Option<Task<()>>,
    _voting_task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl AwardsPage {
    pub fn new(nav: Navigator, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let award_picker = picker::picker(
            std::iter::once(Opt::any("All Awards"))
                .chain(catalog::AWARDS.iter().map(|a| Opt::new(*a, *a)))
                .collect(),
            "",
            true,
            window,
            cx,
        );
        let year_picker = picker::picker(
            picker::years(
                "All Years",
                catalog::LAST_SEASON - YEARS_SHOWN + 1,
                catalog::LAST_SEASON,
            ),
            "",
            true,
            window,
            cx,
        );
        let league_picker = picker::picker(
            std::iter::once(Opt::any("All Leagues"))
                .chain(
                    catalog::AWARD_LEAGUES
                        .iter()
                        .map(|l| Opt::new(l.value, l.label)),
                )
                .collect(),
            "",
            false,
            window,
            cx,
        );
        let subscriptions = vec![
            cx.subscribe_in(
                &award_picker,
                window,
                |this, _, event: &PickerEvent, window, cx| {
                    this.award = picker::value_of(event);
                    this.refetch(window, cx);
                },
            ),
            cx.subscribe_in(
                &year_picker,
                window,
                |this, _, event: &PickerEvent, window, cx| {
                    this.year = picker::value_of(event).and_then(|v| v.parse().ok());
                    this.refetch(window, cx);
                },
            ),
            cx.subscribe_in(
                &league_picker,
                window,
                |this, _, event: &PickerEvent, window, cx| {
                    this.league = picker::value_of(event);
                    this.refetch(window, cx);
                },
            ),
        ];
        let mut page = Self {
            nav,
            scroll: ScrollHandle::new(),
            award: None,
            year: None,
            league: None,
            page: 1,
            award_picker,
            year_picker,
            league_picker,
            winners: Loadable::Idle,
            voting: Loadable::Idle,
            _winners_task: None,
            _voting_task: None,
            _subscriptions: subscriptions,
        };
        page.reload(window, cx);
        page
    }

    fn refetch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.page = 1;
        self.reload(window, cx);
    }

    /// Shows the voting race for one award, year and league.
    fn view_race(
        &mut self,
        award: String,
        year: i32,
        league: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        picker::select_value(&self.award_picker, &award, window, cx);
        picker::select_value(&self.year_picker, &year.to_string(), window, cx);
        picker::select_value(&self.league_picker, &league, window, cx);
        self.award = Some(award);
        self.year = Some(year);
        self.league = Some(league);
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.refetch(window, cx);
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for p in [&self.award_picker, &self.year_picker, &self.league_picker] {
            picker::select_value(p, "", window, cx);
        }
        self.award = None;
        self.year = None;
        self.league = None;
        self.refetch(window, cx);
    }

    fn render_voting(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (award, year, league) = (self.award.clone()?, self.year?, self.league.clone()?);
        let title = format!("{year} {league} {award} Voting");
        let body: AnyElement = match &self.voting {
            Loadable::Ready(Some(voting)) if !voting.votes.is_empty() => {
                let mut votes = voting.votes.clone();
                votes.sort_by(|a, b| {
                    b.points_won
                        .partial_cmp(&a.points_won)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let winner = votes.first().map(|v| v.player_id.clone());
                let rank_of = |v: &AwardVote| {
                    votes
                        .iter()
                        .position(|o| o.player_id == v.player_id)
                        .unwrap_or(0)
                        + 1
                };
                let nav = self.nav.clone();
                let winner_for_cell = winner.clone();
                let cols = vec![
                    Col::num("#", 48., |v: &AwardVote| rank_of(v).to_string()),
                    Col::element("Player", 260., move |v: &AwardVote, _| {
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(widgets::player_link(
                                SharedString::from(format!("vote-{}", v.player_id)),
                                &v.player_id,
                                v.player_name.clone().unwrap_or_else(|| v.player_id.clone()),
                                false,
                                &nav,
                            ))
                            .when(
                                winner_for_cell.as_deref() == Some(v.player_id.as_str()),
                                |this| this.child(Tag::success().xsmall().child("Winner")),
                            )
                    }),
                    Col::num("Points", 72., |v: &AwardVote| {
                        format::decimal_trim(v.points_won)
                    }),
                    Col::num("Max", 64., |v: &AwardVote| {
                        format::decimal_trim(v.points_max)
                    }),
                    Col::num("1st Place", 80., |v: &AwardVote| {
                        format::text(v.votes_first.as_deref())
                    }),
                    Col::num("Share", 72., |v: &AwardVote| format::percent(v.vote_share)),
                    Col::element("Vote Share", 200., |v: &AwardVote, _| {
                        div().w(px(180.)).child(
                            Progress::new(SharedString::from(format!("share-{}", v.player_id)))
                                .value(v.vote_share.unwrap_or(0.) as f32)
                                .accessibility_label(format!(
                                    "{} vote share",
                                    format::percent(v.vote_share)
                                )),
                        )
                    }),
                ];
                let green = cx.theme().success.opacity(0.12);
                grid_with_row_style(
                    "voting-table",
                    cols,
                    &votes,
                    |v| (winner.as_deref() == Some(v.player_id.as_str())).then_some(green),
                    cx,
                )
            }
            Loadable::Ready(_) => div()
                .p_4()
                .child(widgets::note(
                    "No voting results are on record for this award, year and league.",
                    cx,
                ))
                .into_any_element(),
            Loadable::Failed(message) => widgets::error_state(
                "voting-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            _ => widgets::loading_state("Loading award data…", cx),
        };
        Some(widgets::section(title, cx).child(body).into_any_element())
    }
}

impl Page for AwardsPage {
    fn title(&self, _: &App) -> SharedString {
        "Awards & Voting".into()
    }

    fn reload(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let (award, year, league, page) = (
            self.award.clone(),
            self.year,
            self.league.clone(),
            self.page,
        );
        self.winners = Loadable::Loading;
        {
            let (award, league) = (award.clone(), league.clone());
            self._winners_task = Some(loader::fetch(
                cx,
                move |api| {
                    api.award_winners(award.as_deref(), year, league.as_deref(), page, PAGE_SIZE)
                },
                |this: &mut Self, result, _| this.winners = Loadable::from_result(result),
            ));
        }
        self._voting_task = None;
        self.voting = Loadable::Idle;
        if let (Some(award), Some(year), Some(league)) = (award, year, league) {
            self.voting = Loadable::Loading;
            self._voting_task = Some(loader::fetch(
                cx,
                move |api| match api.award_voting(&award, year, &league) {
                    Ok(voting) => Ok(Some(voting)),
                    Err(e) if crate::api::is_not_found(&e) => Ok(None),
                    Err(e) => Err(e),
                },
                |this: &mut Self, result, _| this.voting = Loadable::from_result(result),
            ));
        }
        cx.notify();
    }
}

impl Render for AwardsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let filters = widgets::filter_bar(cx)
            .child(widgets::field(
                "Award",
                picker::dropdown(&self.award_picker, 280.),
                cx,
            ))
            .child(widgets::field(
                "Year",
                picker::dropdown(&self.year_picker, 120.),
                cx,
            ))
            .child(widgets::field(
                "League",
                picker::dropdown(&self.league_picker, 140.),
                cx,
            ))
            .child(
                Button::new("awards-clear")
                    .small()
                    .ghost()
                    .label("Clear Filters")
                    .on_click(cx.listener(|this, _, window, cx| this.clear(window, cx))),
            );
        let voting = self.render_voting(cx);
        let nav = self.nav.clone();
        let body: AnyElement = match &self.winners {
            Loadable::Ready(p) if p.data.is_empty() => widgets::empty_state(
                "No Awards Found",
                "No award winners match the selected filters.",
                cx,
            ),
            Loadable::Ready(p) => {
                let this = cx.entity().downgrade();
                let race_this = this.clone();
                let cols = vec![
                    Col::element("Player", 240., move |w: &AwardWinner, ix| {
                        widgets::player_link(
                            SharedString::from(format!("winner-{}-{}-{ix}", w.player_id, w.year)),
                            &w.player_id,
                            w.player_name.clone().unwrap_or_else(|| w.player_id.clone()),
                            false,
                            &nav,
                        )
                    }),
                    Col::element("Award", 260., |w: &AwardWinner, _| {
                        Tag::primary().outline().xsmall().child(w.award_id.clone())
                    }),
                    Col::text("Year", 64., |w: &AwardWinner| w.year.to_string()),
                    Col::text("League", 72., |w: &AwardWinner| {
                        format::text(w.lg_id.as_deref())
                    }),
                    Col::text("Notes", 120., |w: &AwardWinner| {
                        format::text(w.notes.as_deref())
                    }),
                    Col::element("", 120., move |w: &AwardWinner, ix| {
                        let has_race = catalog::VOTED_AWARDS.contains(&w.award_id.as_str())
                            && w.lg_id.is_some();
                        let (award, year, league) = (
                            w.award_id.clone(),
                            w.year,
                            w.lg_id.clone().unwrap_or_default(),
                        );
                        let this = race_this.clone();
                        div().when(has_race, |d| {
                            d.child(
                                Button::new(SharedString::from(format!(
                                    "race-{}-{}-{ix}",
                                    w.player_id, w.year
                                )))
                                .xsmall()
                                .outline()
                                .label("View Race")
                                .on_click(
                                    move |_, window, cx| {
                                        let (award, league) = (award.clone(), league.clone());
                                        this.update(cx, |this, cx| {
                                            this.view_race(award, year, league, window, cx)
                                        })
                                        .ok();
                                    },
                                ),
                            )
                        })
                    }),
                ];
                v_flex()
                    .gap_3()
                    .child(
                        widgets::section_with(
                            "Award Winners",
                            widgets::note(
                                format!("{} awards", format::int(Some(p.total_count as i64))),
                                cx,
                            ),
                            cx,
                        )
                        .child(grid_with_row_style(
                            "winners-table",
                            cols,
                            &p.data,
                            |_| None,
                            cx,
                        )),
                    )
                    .child(widgets::pager(
                        "awards-pager",
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
                "awards-error",
                message,
                cx.listener(|this, _, window, cx| this.reload(window, cx)),
            ),
            _ => widgets::loading_state("Loading award data…", cx),
        };
        widgets::page_body("awards", &self.scroll).child(
            widgets::page_column()
                .child(widgets::page_header(
                    Some("Stats"),
                    "Awards & Voting",
                    Some("Player awards and the voting races behind them.".into()),
                    vec![],
                    cx,
                ))
                .child(filters)
                .children(voting)
                .child(body),
        )
    }
}

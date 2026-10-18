//! About, Glossary and Surviving Records: static text ported from the web app.

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    text::TextView,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::{ReferencePage, Route};
use crate::ui::widgets;

const ABOUT: &str = include_str!("../../content/about.md");
const GLOSSARY: &str = include_str!("../../content/glossary.md");
const SURVIVING_RECORDS: &str = include_str!("../../content/surviving_records.md");

/// Surviving Records links to the Negro Leagues hub before this section, as on the web.
const BROWSE_BEFORE: &str = "## How this app presents it";

pub struct ReferencePageView {
    page: ReferencePage,
    nav: Navigator,
    scroll: ScrollHandle,
}

impl ReferencePageView {
    pub fn new(page: ReferencePage, nav: Navigator, _: &mut Window, _: &mut Context<Self>) -> Self {
        Self {
            page,
            nav,
            scroll: ScrollHandle::new(),
        }
    }

    fn content(&self) -> (&'static str, &'static str, &'static str, &'static str) {
        match self.page {
            ReferencePage::About => (
                "Project",
                "About",
                "Explore the app, dataset, and stack behind the Baseball History experience.",
                ABOUT,
            ),
            ReferencePage::Glossary => (
                "Reference",
                "Stats Glossary",
                "What the statistics mean, how qualification works, and where the numbers stop short.",
                GLOSSARY,
            ),
            ReferencePage::SurvivingRecords => (
                "History",
                "The Surviving Record",
                "Why some careers in this database are fragments — and why the fragments are worth showing.",
                SURVIVING_RECORDS,
            ),
        }
    }
}

fn markdown(id: String, text: &'static str) -> TextView {
    TextView::markdown(SharedString::from(id), text).selectable(true)
}

impl ReferencePageView {
    fn browse_the_record(&self, cx: &App) -> AnyElement {
        v_flex()
            .gap_2()
            .child(div().text_xl().font_bold().child("Browse the record"))
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child("The seven recognized leagues — their seasons, standings, pennant winners, and statistical leaders — can be explored directly. Every table carries the same caveat this page explains: what survives is a floor, not a measure."),
            )
            .child(
                h_flex().child(
                    Button::new("browse-negro-leagues")
                        .small()
                        .primary()
                        .label("Explore the Negro Leagues hub")
                        .on_click(self.nav.to(Route::NegroLeagues)),
                ),
            )
            .into_any_element()
    }
}

impl Page for ReferencePageView {
    fn title(&self, _: &App) -> SharedString {
        self.content().1.into()
    }

    fn reload(&mut self, _: &mut Window, _: &mut Context<Self>) {}
}

impl Render for ReferencePageView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (eyebrow, title, subtitle, body) = self.content();
        widgets::page_body("reference", &self.scroll).child(
            widgets::page_column()
                .max_w(px(820.))
                .child(widgets::page_header(
                    Some(eyebrow),
                    title,
                    Some(subtitle.into()),
                    vec![],
                    cx,
                ))
                .map(|column| match body.split_once(BROWSE_BEFORE) {
                    Some((before, _)) if self.page == ReferencePage::SurvivingRecords => {
                        let after = &body[before.len()..];
                        column
                            .child(markdown(format!("md-{title}"), before))
                            .child(self.browse_the_record(cx))
                            .child(markdown(format!("md-{title}-2"), after))
                    }
                    _ => column.child(markdown(format!("md-{title}"), body)),
                }),
        )
    }
}

//! About, Glossary and Surviving Records: static text ported from the web app.

use gpui_kit::component::text::TextView;
use gpui_kit::*;

use crate::nav::Navigator;
use crate::pages::Page;
use crate::route::ReferencePage;
use crate::ui::widgets;

const ABOUT: &str = include_str!("../../content/about.md");
const GLOSSARY: &str = include_str!("../../content/glossary.md");
const SURVIVING_RECORDS: &str = include_str!("../../content/surviving_records.md");

pub struct ReferencePageView {
    page: ReferencePage,
    scroll: ScrollHandle,
}

impl ReferencePageView {
    pub fn new(page: ReferencePage, _nav: Navigator, _: &mut Window, _: &mut Context<Self>) -> Self {
        Self { page, scroll: ScrollHandle::new() }
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
                .child(widgets::page_header(Some(eyebrow), title, Some(subtitle.into()), vec![], cx))
                .child(TextView::markdown(SharedString::from(format!("md-{title}")), body).selectable(true)),
        )
    }
}

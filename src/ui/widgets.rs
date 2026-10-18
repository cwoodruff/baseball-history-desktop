//! Small presentational building blocks shared by every page.

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, StyledExt as _,
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    spinner::Spinner,
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::nav::Navigator;
use crate::route::{ReferencePage, Route};

/// Hall of Fame gold, taken from the web app's badge gradient.
pub fn hof_gold() -> Hsla {
    rgb(0xd4af37).into()
}

/// The web app's brand navy and red.
pub fn brand_navy() -> Hsla {
    rgb(0x002d72).into()
}

pub fn brand_red() -> Hsla {
    rgb(0xd50032).into()
}

/// The page title row: optional eyebrow, a title with trailing badges, and a subtitle.
pub fn page_header(
    eyebrow: Option<&str>,
    title: impl Into<SharedString>,
    subtitle: Option<SharedString>,
    badges: Vec<AnyElement>,
    cx: &App,
) -> AnyElement {
    v_flex()
        .gap_1()
        .when_some(eyebrow, |this, eyebrow| {
            this.child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().muted_foreground)
                    .child(eyebrow.to_uppercase()),
            )
        })
        .child(
            h_flex()
                .gap_2()
                .flex_wrap()
                .items_center()
                .child(div().text_2xl().font_bold().child(title.into()))
                .children(badges),
        )
        .when_some(subtitle, |this, subtitle| {
            this.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(subtitle),
            )
        })
        .into_any_element()
}

/// A bordered card with a heading, used for every titled block of content.
pub fn section(title: impl Into<SharedString>, cx: &App) -> Div {
    v_flex()
        .w_full()
        .min_w_0()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius_lg)
        .bg(cx.theme().background)
        .overflow_hidden()
        .child(
            h_flex()
                .px_4()
                .py_2()
                .gap_2()
                .items_center()
                .bg(cx.theme().secondary)
                .border_b_1()
                .border_color(cx.theme().border)
                .child(div().font_semibold().text_sm().child(title.into())),
        )
}

/// A section whose heading carries trailing content (a count, a toggle, ...).
pub fn section_with(title: impl Into<SharedString>, trailing: impl IntoElement, cx: &App) -> Div {
    v_flex()
        .w_full()
        .min_w_0()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius_lg)
        .bg(cx.theme().background)
        .overflow_hidden()
        .child(
            h_flex()
                .px_4()
                .py_2()
                .gap_2()
                .items_center()
                .justify_between()
                .bg(cx.theme().secondary)
                .border_b_1()
                .border_color(cx.theme().border)
                .child(div().font_semibold().text_sm().child(title.into()))
                .child(trailing),
        )
}

/// A labeled headline number.
pub fn stat_box(label: &str, value: impl Into<SharedString>, cx: &App) -> AnyElement {
    v_flex()
        .flex_1()
        .min_w_0()
        .px_3()
        .py_2()
        .gap_0p5()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius)
        .bg(cx.theme().secondary)
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(label.to_string()),
        )
        .child(div().text_lg().font_bold().truncate().child(value.into()))
        .into_any_element()
}

/// A wrapping row of stat boxes.
pub fn stat_row(items: Vec<(&str, String)>, cx: &App) -> AnyElement {
    h_flex()
        .w_full()
        .gap_3()
        .children(
            items
                .into_iter()
                .map(|(label, value)| stat_box(label, value, cx)),
        )
        .into_any_element()
}

/// A label/value list, used for biographical facts.
pub fn facts(rows: Vec<(&str, String)>, cx: &App) -> AnyElement {
    v_flex()
        .px_4()
        .py_3()
        .gap_2()
        .children(rows.into_iter().map(|(label, value)| {
            h_flex()
                .gap_3()
                .items_start()
                .child(
                    div()
                        .w(px(96.))
                        .flex_shrink_0()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(label.to_string()),
                )
                .child(div().flex_1().min_w_0().text_sm().child(value))
        }))
        .into_any_element()
}

/// The gold "HOF" marker shown next to Hall of Famers.
pub fn hof_tag() -> Tag {
    Tag::custom(hof_gold(), rgb(0x1a1a1a).into(), rgb(0xb8962e).into())
        .xsmall()
        .child("HOF")
}

/// A "HOF 1936" marker for the player header.
pub fn hof_year_tag(year: Option<i32>) -> Tag {
    let label = match year {
        Some(y) => format!("HOF {y}"),
        None => "HOF".into(),
    };
    Tag::custom(hof_gold(), rgb(0x1a1a1a).into(), rgb(0xb8962e).into())
        .small()
        .child(label)
}

pub const PARTIAL_RECORD_HELP: &str = "Historically incomplete record. Only a partial name survived in the original sources for this player — most such records come from segregation-era Black baseball, where box scores and rosters were unevenly preserved; a few date to the 19th century. Documented statistics may understate this player's actual career.";

/// The "Partial record" chip: explains itself on hover and opens Surviving Records.
pub fn partial_record_chip(id: impl Into<ElementId>, nav: &Navigator) -> AnyElement {
    let nav = nav.clone();
    Button::new(id)
        .xsmall()
        .warning()
        .outline()
        .label("Partial record")
        .tooltip(format!("{PARTIAL_RECORD_HELP} Click to learn why."))
        .on_click(move |_, window, cx| {
            nav.go(
                Route::Reference(ReferencePage::SurvivingRecords),
                window,
                cx,
            )
        })
        .into_any_element()
}

/// The compact dagger used in dense lists for partial records.
pub fn partial_dagger(cx: &App) -> AnyElement {
    div()
        .text_xs()
        .font_bold()
        .text_color(cx.theme().warning)
        .child("†")
        .into_any_element()
}

/// A player name that opens the player page, with the HOF marker when earned.
pub fn player_link(
    id: impl Into<ElementId>,
    player_id: &str,
    name: impl Into<SharedString>,
    hof: bool,
    nav: &Navigator,
) -> AnyElement {
    let route = Route::Player {
        id: player_id.to_string(),
    };
    h_flex()
        .gap_1()
        .items_center()
        .min_w_0()
        .child(nav.button(id, name, route))
        .when(hof, |this| this.child(hof_tag()))
        .into_any_element()
}

/// The centered message shown when a query returns nothing.
pub fn empty_state(title: &str, message: &str, cx: &App) -> AnyElement {
    v_flex()
        .w_full()
        .py_12()
        .gap_2()
        .items_center()
        .justify_center()
        .text_center()
        .child(
            Icon::new(IconName::Inbox)
                .size_10()
                .text_color(cx.theme().muted_foreground),
        )
        .child(div().font_semibold().child(title.to_string()))
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(message.to_string()),
        )
        .into_any_element()
}

/// A spinner with a message, for content that is genuinely loading.
pub fn loading_state(message: &str, cx: &App) -> AnyElement {
    h_flex()
        .w_full()
        .py_12()
        .gap_2()
        .items_center()
        .justify_center()
        .text_color(cx.theme().muted_foreground)
        .child(Spinner::new().small())
        .child(div().text_sm().child(message.to_string()))
        .into_any_element()
}

/// A failed request, with a Retry command.
pub fn error_state(
    id: impl Into<ElementId>,
    message: &str,
    on_retry: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let id: ElementId = id.into();
    v_flex()
        .w_full()
        .py_4()
        .gap_3()
        .child(Alert::error(id.clone(), message.to_string()).title("Couldn't load this data"))
        .child(
            h_flex().child(
                Button::new(ElementId::Name(format!("{id}-retry").into()))
                    .small()
                    .icon(IconName::RefreshCw)
                    .label("Retry")
                    .on_click(on_retry),
            ),
        )
        .into_any_element()
}

/// A small muted note line.
pub fn note(text: impl Into<SharedString>, cx: &App) -> AnyElement {
    div()
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(text.into())
        .into_any_element()
}

/// A labeled filter control for a filter bar.
pub fn field(label: &str, control: impl IntoElement, cx: &App) -> AnyElement {
    v_flex()
        .gap_1()
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child(label.to_string()),
        )
        .child(control)
        .into_any_element()
}

/// The horizontal, wrapping bar that holds a page's filters.
pub fn filter_bar(cx: &App) -> Div {
    h_flex()
        .w_full()
        .gap_3()
        .flex_wrap()
        .items_end()
        .p_3()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius_lg)
        .bg(cx.theme().secondary)
}

/// The standard scrolling page body with consistent padding.
pub fn page_body(id: impl Into<ElementId>, scroll: &ScrollHandle) -> Stateful<Div> {
    use gpui_kit::component::scroll::ScrollableElement as _;
    div()
        .id(id)
        .size_full()
        .overflow_y_scroll()
        .track_scroll(scroll)
        .vertical_scrollbar(scroll)
}

/// The padded column inside [`page_body`].
pub fn page_column() -> Div {
    v_flex().w_full().max_w(px(1280.)).mx_auto().p_5().gap_5()
}

/// Page navigation with the web app's "Page X of Y (N total)" summary.
pub fn pager(
    id: impl Into<ElementId>,
    page: u32,
    total_pages: u32,
    total_count: u32,
    on_change: impl Fn(u32, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    use gpui_kit::component::pagination::Pagination;
    h_flex()
        .w_full()
        .gap_3()
        .items_center()
        .justify_between()
        .flex_wrap()
        .child(note(
            format!(
                "Page {} of {} ({} total)",
                page,
                total_pages.max(1),
                crate::format::int(Some(total_count as i64))
            ),
            cx,
        ))
        .when(total_pages > 1, |this| {
            this.child(
                Pagination::new(id)
                    .small()
                    .current_page(page as usize)
                    .total_pages(total_pages as usize)
                    .visible_pages(7)
                    .on_click(move |page, window, cx| on_change(*page as u32, window, cx)),
            )
        })
        .into_any_element()
}

/// `« 1932` / `1934 »` buttons that step to the nearest season with data.
/// Each button is left out when there is no season in that direction.
pub fn year_steps(
    id: &str,
    previous: Option<i32>,
    next: Option<i32>,
    route: impl Fn(i32) -> Route,
    nav: &Navigator,
) -> AnyElement {
    h_flex()
        .gap_2()
        .when_some(previous, |this, year| {
            this.child(
                Button::new(SharedString::from(format!("{id}-prev")))
                    .small()
                    .outline()
                    .label(format!("« {year}"))
                    .tooltip("Previous season")
                    .on_click(nav.to(route(year))),
            )
        })
        .when_some(next, |this, year| {
            this.child(
                Button::new(SharedString::from(format!("{id}-next")))
                    .small()
                    .outline()
                    .label(format!("{year} »"))
                    .tooltip("Next season")
                    .on_click(nav.to(route(year))),
            )
        })
        .into_any_element()
}

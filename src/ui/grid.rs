//! A declarative wrapper over GPUI Kit's stateless `Table` for record lists.
//!
//! Every list in the app is a page of at most 100 rows (the API's page size
//! limit) or a short detail table, so a non-virtualized table is the right
//! fit. Cells can hold Buttons, which keeps in-app navigation keyboard
//! reachable and announces the right role.

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Selectable as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow},
    tooltip::Tooltip,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

type CellFn<'a, T> = Box<dyn Fn(&T, usize) -> AnyElement + 'a>;
type ClickFn = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

pub struct Col<'a, T> {
    name: SharedString,
    width: Pixels,
    numeric: bool,
    cell: CellFn<'a, T>,
    on_header_click: Option<ClickFn>,
    active: bool,
    tooltip: Option<SharedString>,
}

impl<'a, T> Col<'a, T> {
    /// A text column.
    pub fn text(name: &str, width: f32, value: impl Fn(&T) -> String + 'a) -> Self {
        Self {
            name: SharedString::from(name.to_string()),
            width: px(width),
            numeric: false,
            cell: Box::new(move |row, _| value(row).into_any_element()),
            on_header_click: None,
            active: false,
            tooltip: None,
        }
    }

    /// A right-aligned, tabular number column.
    pub fn num(name: &str, width: f32, value: impl Fn(&T) -> String + 'a) -> Self {
        Self {
            numeric: true,
            ..Self::text(name, width, value)
        }
    }

    /// A column with arbitrary content; the closure receives the row index.
    pub fn element<E: IntoElement>(
        name: &str,
        width: f32,
        build: impl Fn(&T, usize) -> E + 'a,
    ) -> Self {
        Self {
            name: SharedString::from(name.to_string()),
            width: px(width),
            numeric: false,
            cell: Box::new(move |row, ix| build(row, ix).into_any_element()),
            on_header_click: None,
            active: false,
            tooltip: None,
        }
    }

    pub fn right(mut self) -> Self {
        self.numeric = true;
        self
    }

    /// Makes the header a command, used by leaderboards to rank by this column.
    pub fn on_header_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_header_click = Some(Box::new(handler));
        self
    }

    /// Marks the column the rows are ranked by.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// Renders `rows` as a table. Wider than its container, it scrolls horizontally.
pub fn grid<T>(
    id: impl Into<ElementId>,
    columns: Vec<Col<'_, T>>,
    rows: &[T],
    cx: &App,
) -> AnyElement {
    grid_with_row_style(id, columns, rows, |_| None, cx).into_any_element()
}

/// Like [`grid`], with an optional background per row (e.g. World Series rows).
pub fn grid_with_row_style<T>(
    id: impl Into<ElementId>,
    columns: Vec<Col<'_, T>>,
    rows: &[T],
    row_bg: impl Fn(&T) -> Option<Hsla>,
    cx: &App,
) -> AnyElement {
    use gpui_kit::component::scroll::ScrollableElement as _;

    let id: ElementId = id.into();
    let total: Pixels = columns.iter().map(|c| c.width).fold(px(0.), |a, b| a + b);
    let mono = cx.theme().mono_font_family.clone();
    let muted = cx.theme().muted_foreground;
    let primary = cx.theme().primary;

    let mut header_cells = Vec::with_capacity(columns.len());
    let mut handlers = Vec::with_capacity(columns.len());
    for column in columns.iter() {
        handlers.push(column.on_header_click.is_some());
    }
    let mut columns = columns;
    for (ix, column) in columns.iter_mut().enumerate() {
        let name = column.name.clone();
        // Table cells default to a 100px minimum; pin both header and body
        // cells to the column width so they line up.
        let mut head = TableHead::new()
            .w(column.width)
            .min_w(column.width)
            .flex_none();
        if column.numeric {
            head = head.text_right();
        }
        let content: AnyElement = if let Some(handler) = column.on_header_click.take() {
            Button::new(ElementId::Name(format!("{id}-head-{ix}").into()))
                .ghost()
                .xsmall()
                .compact()
                .selected(column.active)
                .label(name)
                .when(column.active, |this| this.icon(IconName::ChevronDown))
                .when_some(column.tooltip.clone(), |this, tip| this.tooltip(tip))
                .on_click(handler)
                .into_any_element()
        } else if let Some(tip) = column.tooltip.clone() {
            div()
                .id(ElementId::Name(format!("{id}-head-{ix}").into()))
                .when(column.active, |this| this.font_bold().text_color(primary))
                .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
                .child(name)
                .into_any_element()
        } else {
            div()
                .when(column.active, |this| this.font_bold().text_color(primary))
                .child(name)
                .into_any_element()
        };
        head = head.child(content);
        header_cells.push(head);
    }

    let body_rows = rows.iter().enumerate().map(|(row_ix, row)| {
        TableRow::new()
            .when_some(row_bg(row), |this, bg| this.bg(bg))
            .children(columns.iter().map(|column| {
                let mut cell = TableCell::new()
                    .w(column.width)
                    .min_w(column.width)
                    .flex_none()
                    .overflow_hidden();
                if column.numeric {
                    cell = cell.text_right().font_family(mono.clone());
                }
                if column.active {
                    cell = cell.font_bold();
                }
                cell.child((column.cell)(row, row_ix))
            }))
    });

    div()
        .id(id.clone())
        .w_full()
        .overflow_x_scrollbar()
        .child(
            Table::new()
                .small()
                .min_w(total)
                .child(TableHeader::new().child(TableRow::new().children(header_cells)))
                .child(TableBody::new().children(body_rows)),
        )
        .when(rows.is_empty(), |this| {
            this.child(
                h_flex()
                    .p_4()
                    .gap_2()
                    .text_sm()
                    .text_color(muted)
                    .child(Icon::new(IconName::Inbox))
                    .child("No rows."),
            )
        })
        .into_any_element()
}

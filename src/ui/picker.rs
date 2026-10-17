//! Dropdown filters built on GPUI Kit's `Select`.

use gpui_kit::component::{
    IndexPath, Sizable as _,
    searchable_list::SearchableListItem,
    select::{Select, SelectEvent, SelectState},
};
use gpui_kit::*;

/// One dropdown entry. An empty value means "no filter" (All …).
#[derive(Debug, Clone, PartialEq)]
pub struct Opt {
    value: SharedString,
    label: SharedString,
}

impl Opt {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }

    /// The "All …" entry that clears a filter.
    pub fn any(label: impl Into<SharedString>) -> Self {
        Self::new("", label)
    }
}

impl SearchableListItem for Opt {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &SharedString {
        &self.value
    }
}

pub type Picker = Entity<SelectState<Vec<Opt>>>;
pub type PickerEvent = SelectEvent<Vec<Opt>>;

/// Creates a picker with `selected` chosen (or the first entry when absent).
pub fn picker<V: 'static>(
    options: Vec<Opt>,
    selected: &str,
    searchable: bool,
    window: &mut Window,
    cx: &mut Context<V>,
) -> Picker {
    let index = options
        .iter()
        .position(|o| o.value.as_ref() == selected)
        .unwrap_or(0);
    cx.new(|cx| {
        SelectState::new(options, Some(IndexPath::new(index)), window, cx).searchable(searchable)
    })
}

/// The committed value, `None` for the "All …" entry.
pub fn value_of(event: &PickerEvent) -> Option<String> {
    match event {
        SelectEvent::Confirm(Some(value)) if !value.is_empty() => Some(value.to_string()),
        _ => None,
    }
}

/// Selects `value` without emitting a change.
pub fn select_value(picker: &Picker, value: &str, window: &mut Window, cx: &mut App) {
    let value = SharedString::from(value.to_string());
    picker.update(cx, |state, cx| state.set_selected_value(&value, window, cx));
}

/// The dropdown element with standard sizing.
pub fn dropdown(picker: &Picker, width: f32) -> Select<Vec<Opt>> {
    Select::new(picker)
        .small()
        .w(px(width))
        .menu_max_h(px(360.))
}

/// Year options, newest first, after an "All" entry.
pub fn years(all_label: &str, from: i32, to: i32) -> Vec<Opt> {
    std::iter::once(Opt::any(all_label.to_string()))
        .chain(
            (from..=to)
                .rev()
                .map(|y| Opt::new(y.to_string(), y.to_string())),
        )
        .collect()
}

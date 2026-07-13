use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Document, HtmlOptionElement, HtmlSelectElement};

pub mod grid;

#[derive(Clone, Copy)]
pub(crate) enum ActiveElement<T> {
    None,
    Hover(T),
    Pressed(T),
}

pub(crate) trait SelectOptionElement {
    type Preset;

    fn text(&self) -> &str;
    fn is_visible(&self, preset: &Self::Preset) -> bool;
}

pub(crate) trait SelectOption: SelectOptionElement + Sized {
    fn create_element(
        document: &Document,
        name: &str,
        preset: &Self::Preset,
        options: &[Self],
    ) -> Result<HtmlSelectElement, JsValue> {
        let select = document
            .create_element("select")?
            .dyn_into::<HtmlSelectElement>()?;
        select.set_name(name);

        for (idx, o) in options.iter().filter(|o| o.is_visible(&preset)).enumerate() {
            let option = document
                .create_element("option")?
                .dyn_into::<HtmlOptionElement>()?;
            option.set_text(o.text());
            option.set_value(&idx.to_string());
            select.append_child(&option)?;
        }

        Ok(select)
    }

    fn selected_value(raw: &str, preset: &Self::Preset, options: &[Self]) -> Option<Self>
    where
        Self: Clone,
    {
        let idx = raw.parse::<usize>().ok()?;
        options
            .iter()
            .filter(|o| o.is_visible(preset))
            .nth(idx)
            .map(|o| o.clone())
    }

    fn find_index(
        mut find_value: impl FnMut(&Self) -> bool,
        preset: &Self::Preset,
        options: &[Self],
    ) -> Option<usize> {
        options
            .iter()
            .filter(|o| o.is_visible(preset))
            .enumerate()
            .find_map(|(idx, o)| (find_value(o)).then_some(idx))
    }
}

impl<T> SelectOption for T where T: SelectOptionElement {}

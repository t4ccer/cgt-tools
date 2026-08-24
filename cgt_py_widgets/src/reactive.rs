use cgt::{drawing::Interactions, numeric::v2f::V2f};
use futures_signals::signal::{Mutable, Signal, SignalExt};
use wasm_bindgen::{JsCast as _, JsValue, prelude::ScopedClosure};
use web_sys::{
    Document, HtmlCanvasElement, HtmlElement, HtmlInputElement, HtmlOptionElement,
    HtmlSelectElement, MouseEvent,
};

pub fn checkbox(input: &HtmlInputElement, output: &Mutable<bool>) -> Result<(), JsValue> {
    let on_html_change = ScopedClosure::<dyn FnMut() -> ()>::new({
        let checkbox = input.clone();
        let output = output.clone();
        move || output.set(checkbox.checked())
    });
    input.add_event_listener_with_callback("change", on_html_change.as_ref().unchecked_ref())?;
    on_html_change.forget();

    wasm_bindgen_futures::spawn_local(output.signal().dedupe().for_each({
        let checkbox = input.clone();
        move |checked| {
            let _ = checkbox.set_checked(checked);
            async {}
        }
    }));

    Ok(())
}

pub fn style_set_property<'a, S>(
    element: HtmlElement,
    property: &'static str,
    value: S,
) -> Result<(), JsValue>
where
    S: Signal<Item = &'a str> + 'static,
{
    wasm_bindgen_futures::spawn_local(value.for_each(move |value| {
        let _ = element.style().set_property(property, value);
        async {}
    }));
    Ok(())
}

fn mouse_event_to_canvas(event: &MouseEvent) -> V2f {
    V2f {
        x: event.offset_x() as f32,
        y: event.offset_y() as f32,
    }
}

pub fn canvas_interactions(
    canvas: &HtmlCanvasElement,
    output: &Mutable<Interactions>,
) -> Result<(), JsValue> {
    let down_handler = ScopedClosure::<dyn FnMut(MouseEvent)>::new({
        let interactions = output.clone();
        move |event: MouseEvent| {
            if event.button() != 0 {
                return;
            }

            interactions
                .lock_mut()
                .pointer_pressed(mouse_event_to_canvas(&event));
        }
    });
    canvas
        .add_event_listener_with_callback("mousedown", down_handler.as_ref().unchecked_ref())
        .unwrap();
    down_handler.forget();

    let up_handler = ScopedClosure::<dyn FnMut(MouseEvent)>::new({
        let interactions = output.clone();
        move |event| {
            interactions
                .lock_mut()
                .pointer_released(mouse_event_to_canvas(&event));
        }
    });
    canvas
        .add_event_listener_with_callback("mouseup", up_handler.as_ref().unchecked_ref())
        .unwrap();
    up_handler.forget();

    let move_handler = ScopedClosure::<dyn FnMut(MouseEvent)>::new({
        let interactions = output.clone();
        move |event| {
            interactions
                .lock_mut()
                .pointer_moved(mouse_event_to_canvas(&event));
        }
    });
    canvas
        .add_event_listener_with_callback("mousemove", move_handler.as_ref().unchecked_ref())
        .unwrap();
    move_handler.forget();

    let leave_handler = ScopedClosure::<dyn FnMut()>::new({
        let interactions = output.clone();
        move || {
            interactions.lock_mut().pointer_left();
        }
    });
    canvas
        .add_event_listener_with_callback("mouseleave", leave_handler.as_ref().unchecked_ref())
        .unwrap();
    leave_handler.forget();

    Ok(())
}

pub trait SelectOptionElement {
    type Preset;

    fn text(&self) -> &str;
    fn is_visible(&self, preset: &Self::Preset) -> bool;
}

pub trait SelectOption: SelectOptionElement + Sized {
    fn create_element_reactive(
        document: &Document,
        name: &str,
        preset: Self::Preset,
        options: &'static [Self],
        output: Mutable<Self>,
    ) -> Result<HtmlSelectElement, JsValue>
    where
        Self: Clone + PartialEq,
        Self::Preset: Clone,
    {
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

        let on_html_change = ScopedClosure::<dyn FnMut()>::new({
            let select = select.clone();
            let preset = preset.clone();
            let output = output.clone();
            move || {
                if let Some(mode) = SelectOption::selected_value(&select.value(), &preset, options)
                {
                    output.set(mode);
                };
            }
        });
        select
            .add_event_listener_with_callback("change", on_html_change.as_ref().unchecked_ref())?;
        on_html_change.forget();

        wasm_bindgen_futures::spawn_local(
            output
                .signal_ref(move |option| Self::find_index(|o| o == option, &preset, options))
                .for_each({
                    let select = select.clone();
                    move |option| {
                        if let Some(idx) = option {
                            let idx = idx.to_string();
                            if select.value() != idx {
                                select.set_value(&idx);
                            }
                        }
                        async {}
                    }
                }),
        );

        Ok(select)
    }

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

    fn selected_value_idx(idx: usize, preset: &Self::Preset, options: &[Self]) -> Option<Self>
    where
        Self: Clone,
    {
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

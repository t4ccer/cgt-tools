use leptos::{html::Div, *};
use leptos_use::{
    use_mouse_with_options, UseMouseCoordType, UseMouseEventExtractor, UseMouseOptions,
};
use web_sys::{MouseEvent, Touch};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy)]
pub struct MouseState {
    pub left: MouseButtonState,
    pub middle: MouseButtonState,
}

impl MouseState {
    pub fn new(cx: Scope, workplace: NodeRef<Div>) -> Self {
        MouseState {
            left: MouseButtonState::new(cx, workplace, MouseButton::Primary),
            middle: MouseButtonState::new(cx, workplace, MouseButton::Auxiliary),
        }
    }
}

#[derive(Clone, Copy)]
pub struct MouseButtonState {
    pub delta_x: ReadSignal<f64>,
    pub delta_y: ReadSignal<f64>,
    pub pressed: ReadSignal<bool>,
}

impl MouseButtonState {
    fn new(cx: Scope, workplace: NodeRef<Div>, button: MouseButton) -> Self {
        let (extractor, pressed) = DeltaExtractor::new(cx, button);
        let delta = use_mouse_with_options(
            cx,
            UseMouseOptions::default()
                .coord_type(UseMouseCoordType::Custom(extractor))
                .target(workplace),
        );

        Self {
            delta_x: delta.x,
            delta_y: delta.y,
            pressed,
        }
    }
}

#[derive(Clone)]
struct DeltaExtractor {
    button: MouseButton,
    prev: ReadSignal<Option<Position>>,
    set_prev: WriteSignal<Option<Position>>,
    is_pressed: ReadSignal<bool>,
    set_is_pressed: WriteSignal<bool>,
}

// https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent/buttons
#[bitmask_enum::bitmask(u16)]
enum MouseButton {
    /// Usually the left button
    Primary,

    /// Usually the right button
    Secondary,

    /// Usually the mouse wheel button or middle button
    Auxiliary,

    /// Typically the "Browser Back" button
    Back,

    /// Typically the "Browser Forward" button
    Forward,
}

impl DeltaExtractor {
    fn new(cx: Scope, button: MouseButton) -> (Self, ReadSignal<bool>) {
        let (prev, set_prev) = create_signal(cx, None);
        let (is_pressed, set_is_pressed) = create_signal(cx, false);
        (
            DeltaExtractor {
                button,
                prev,
                set_prev,
                is_pressed,
                set_is_pressed,
            },
            is_pressed,
        )
    }
}

impl UseMouseEventExtractor for DeltaExtractor {
    fn extract_mouse_coords(&self, event: &MouseEvent) -> Option<(f64, f64)> {
        let pressed_buttons = MouseButton::from(event.buttons());

        if !pressed_buttons.contains(self.button) {
            self.set_is_pressed.set(false);
            self.set_prev.set(None);
            return None;
        }
        if !self.is_pressed.get_untracked() {
            self.set_is_pressed.set(true);
        }

        let cur = Position {
            x: event.offset_x() as f64,
            y: event.offset_y() as f64,
        };
        let mut res = None;
        if let Some(prev_pos) = self.prev.get_untracked() {
            let dx = cur.x - prev_pos.x;
            let dy = cur.y - prev_pos.y;
            res = Some((dx, dy));
        }

        self.set_prev.set(Some(cur));

        res
    }

    // this is not necessary as it's the same as the default implementation of the trait.
    fn extract_touch_coords(&self, _touch: &Touch) -> Option<(f64, f64)> {
        // ignore touch events
        None
    }
}

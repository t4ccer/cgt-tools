use leptos::*;

pub fn main() {
    mount_to_body(|cx| Home(cx))
}

#[component]
pub fn Home(cx: Scope) -> impl IntoView {
    html::div(cx)
        .classes("flex w-screen h-screen bg-black text-white")
        .child("cgt-rs")
}

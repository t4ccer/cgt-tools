import init, {
  render_grid_widget_impl,
  render_graph_widget_impl,
} from "./pkg/cgt_py_widgets.js";
import wasmInlineSource from "./pkg/cgt_py_widgets_bg.wasm";

export async function render_grid(model, el, preset) {
  await init(wasmInlineSource);
  render_grid_widget_impl(model, el, preset);
}

export async function render_graph(model, el, preset) {
  await init(wasmInlineSource);
  render_graph_widget_impl(model, el, preset);
}

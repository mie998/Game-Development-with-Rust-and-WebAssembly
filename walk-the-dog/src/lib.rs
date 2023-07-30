use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::console;

// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn draw_triangle(context: &web_sys::CanvasRenderingContext2d, points: [(f64, f64); 3]) {
    let [top, left, right] = points;

    context.move_to(top.0, top.1);
    context.begin_path();
    context.line_to(top.0, top.1);
    context.line_to(left.0, left.1);
    context.line_to(right.0, right.1);
    context.close_path();
    context.stroke();
}

// https://en.wikipedia.org/wiki/Heron%27s_formula
fn calc_triangle_area(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    let x = ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
    let y = ((b.0 - c.0).powi(2) + (b.1 - c.1).powi(2)).sqrt();
    let z = ((c.0 - a.0).powi(2) + (c.1 - a.1).powi(2)).sqrt();

    let s = (x + y + z) / 2.0;
    (s * (s - x) * (s - y) * (s - z)).sqrt()
}

fn draw_sierpinski_gasket(context: &web_sys::CanvasRenderingContext2d, points: [(f64, f64); 3]) {
    let top = points[0];
    let left = points[1];
    let right = points[2];
    if calc_triangle_area(top, left, right) < 10.0 {
        return;
    }

    draw_triangle(context, points);

    let mid_left = ((top.0 + left.0) / 2.0, (top.1 + left.1) / 2.0);
    let mid_right = ((top.0 + right.0) / 2.0, (top.1 + right.1) / 2.0);
    let mid_bottom = ((left.0 + right.0) / 2.0, (left.1 + right.1) / 2.0);

    // left-side
    draw_sierpinski_gasket(context, [mid_left, left, mid_bottom]);

    // right-side
    draw_sierpinski_gasket(context, [mid_right, mid_bottom, right]);

    // top-side
    draw_sierpinski_gasket(context, [top, mid_left, mid_right]);
}

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    console_error_panic_hook::set_once();

    let top = (300.0, 0.0);
    let left = (0.0, 600.0);
    let right = (600.0, 600.0);
    console::log_1(&JsValue::from_str(&format!("{:?}", calc_triangle_area(top, left, right))));
    draw_sierpinski_gasket(&context, [top, left, right]);

    // Your code goes here!
    console::log_1(&JsValue::from_str("Hello world!"));

    Ok(())
}

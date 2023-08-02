use gloo_utils::format::JsValueSerdeExt;
use rand::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::console;

const RHB_PATH: &str = "walk_the_dog_assets-0.0.7/resized/rhb/";
const SPRITE_PATH: &str = "walk_the_dog_assets-0.0.7/sprite_sheets/";

// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Debug, Deserialize)]
struct Sheet {
    frames: HashMap<String, Cell>,
}

#[derive(Debug, Deserialize)]
struct Rect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

#[derive(Debug, Deserialize)]
struct Cell {
    frame: Rect,
}

fn draw_triangle(
    context: &web_sys::CanvasRenderingContext2d,
    points: [(f64, f64); 3],
    color: (u8, u8, u8),
) {
    let [top, left, right] = points;

    let color_str = format!("rgb({}, {}, {})", color.0, color.1, color.2);
    context.set_fill_style(&wasm_bindgen::JsValue::from_str(&color_str));

    context.move_to(top.0, top.1);
    context.begin_path();
    context.line_to(top.0, top.1);
    context.line_to(left.0, left.1);
    context.line_to(right.0, right.1);
    context.close_path();
    context.fill();
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
    if calc_triangle_area(top, left, right) < 100.0 {
        return;
    }

    let mut rng = thread_rng();
    let color = (
        rng.gen_range(0..255),
        rng.gen_range(0..255),
        rng.gen_range(0..255),
    );

    draw_triangle(context, points, color);

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

async fn fetch_json(json_path: &str) -> Result<JsValue, JsValue> {
    let window = web_sys::window().unwrap();
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(json_path)).await?;
    let resp: web_sys::Response = resp_value.dyn_into()?;

    wasm_bindgen_futures::JsFuture::from(resp.json()?).await
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

    wasm_bindgen_futures::spawn_local(async move {
        let (success_tx, success_rx) = futures::channel::oneshot::channel::<Result<(), JsValue>>();
        let success_tx = Rc::new(Mutex::new(Some(success_tx)));
        let error_tx = Rc::clone(&success_tx);

        let image = web_sys::HtmlImageElement::new().unwrap();

        let callback = Closure::once(move || {
            if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
                success_tx.send(Ok(()));
            }
        });
        let err_callback = Closure::once(move |err| {
            if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
                error_tx.send(Err(err));
            }
        });

        image.set_onload(Some(callback.as_ref().unchecked_ref()));
        image.set_onerror(Some(err_callback.as_ref().unchecked_ref()));

        image.set_src((String::from(RHB_PATH) + "Idle (1).png").as_str());

        success_rx.await;

        context
            .draw_image_with_html_image_element(&image, 0.0, 0.0)
            .unwrap();

        let top = (300.0, 0.0);
        let left = (0.0, 600.0);
        let right = (600.0, 600.0);
        draw_sierpinski_gasket(&context, [top, left, right]);

        
        // read sprite sheet
        context.draw_image_with_html_image_element(&image, 0.0, 0.0);

        let json = fetch_json((String::from(SPRITE_PATH) + "rhb.json").as_str())
            .await
            .expect("Failed to fetch JSON");

        let sheet: Sheet = json.into_serde().expect("Failed to parse JSON");

        let (success_tx, success_rx) = futures::channel::oneshot::channel::<Result<(), JsValue>>();
        let success_tx = Rc::new(Mutex::new(Some(success_tx)));
        let error_tx = Rc::clone(&success_tx);

        let image = web_sys::HtmlImageElement::new().unwrap();

        let callback = Closure::once(move || {
            if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
                success_tx.send(Ok(()));
            }
        });
        let err_callback = Closure::once(move |err| {
            if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
                error_tx.send(Err(err));
            }
        });

        image.set_onload(Some(callback.as_ref().unchecked_ref()));
        image.set_onerror(Some(err_callback.as_ref().unchecked_ref()));

        image.set_src((String::from(SPRITE_PATH) + "rhb.png").as_str());
        success_rx.await;

        let sprite = sheet
            .frames
            .get("Run (1).png")
            .expect("Cell not found");
        context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &image,
            sprite.frame.x.into(),
            sprite.frame.y.into(),
            sprite.frame.w.into(),
            sprite.frame.h.into(),
            300.0,
            300.0,
            sprite.frame.w.into(),
            sprite.frame.h.into(),
        );
    });

    // Your code goes here!
    console::log_1(&JsValue::from_str("Hello world!"));

    Ok(())
}

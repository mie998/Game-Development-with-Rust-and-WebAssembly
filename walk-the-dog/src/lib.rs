use gloo_utils::format::JsValueSerdeExt;
use serde::Deserialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::console;

#[macro_use]
mod browser;
mod engine;

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

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let context = browser::context().expect("Failed to get context");

    browser::spawn_local(async move {
        let json = browser::fetch_json((String::from(SPRITE_PATH) + "rhb.json").as_str())
            .await
            .expect("Failed to fetch JSON");
        let sheet: Sheet = json.into_serde().expect("Failed to parse JSON");
        
        let image = engine::load_image((String::from(RHB_PATH) + "Run (1).png").as_str())
            .await
            .expect("Failed to load image rhb.png");

        let mut frame = -1;
        let interval_callback = Closure::wrap(Box::new(move || {
            frame += 1;
            context.clear_rect(0.0, 0.0, 600.0, 600.0);

            let frame_name = format!("Run ({}).png", frame % 8 + 1);
            let sprite = sheet.frames.get(&frame_name).expect("Cell not found");
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
        }) as Box<dyn FnMut()>);

        browser::window()
            .unwrap()
            .set_interval_with_callback_and_timeout_and_arguments_0(
                interval_callback.as_ref().unchecked_ref(),
                50,
            );
        interval_callback.forget();
    });

    // Your code goes here!
    console::log_1(&JsValue::from_str("Hello world!"));

    Ok(())
}

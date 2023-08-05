use crate::browser;
use crate::browser::LoopClosure;

use serde::Deserialize;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::channel::oneshot::channel;
use std::{cell::RefCell, rc::Rc, sync::Mutex};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlImageElement};

pub async fn load_image(source: &str) -> Result<HtmlImageElement> {
    let image = browser::new_image().expect("Failed to create image");

    let (complete_tx, complete_rx) = channel::<Result<()>>();
    let success_tx = Rc::new(Mutex::new(Some(complete_tx)));
    let error_tx = Rc::clone(&success_tx);

    let success_callback = browser::closure_once(move || {
        if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
            if let Err(err) = success_tx.send(Ok(())) {
                println!("Could not send successful image loaded message! {:#?}", err);
            }
        }
    });
    let err_callback: Closure<dyn FnMut(JsValue)> = browser::closure_once(move |err| {
        if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
            if let Err(err) = error_tx.send(Err(anyhow!("Error Loading Image: {:#?}", err))) {
                println!("Could not send error message on loading image! {:#?}", err);
            }
        }
    });

    image.set_onload(Some(success_callback.as_ref().unchecked_ref()));
    image.set_onerror(Some(err_callback.as_ref().unchecked_ref()));
    image.set_src(source);

    complete_rx.await??;

    Ok(image)
}

#[async_trait(?Send)]
pub trait Game {
    async fn initialize(&self) -> Result<Box<dyn Game>>;
    fn update(&mut self);
    fn draw(&self, renderer: &Renderer);
}

const FRAME_SIZE: f32 = 1.0 / 60.0 * 1000.0;
pub struct GameLoop {
    last_frame: f64,
    accumulated_delta: f32,
}
type SharedLoopClosure = Rc<RefCell<Option<LoopClosure>>>;

impl GameLoop {
    pub async fn start(mut game: impl Game + 'static) -> Result<()> {
        let mut game = game.initialize().await?;
        let mut game_loop = GameLoop {
            last_frame: browser::now()?,
            accumulated_delta: 0.0,
        };
        let f: SharedLoopClosure = Rc::new(RefCell::new(None));
        let g = f.clone();

        let renderer = Renderer {
            context: browser::context()?
        };

        *g.borrow_mut() = Some(browser::create_raf_closure(move |perf: f64| {
            game_loop.accumulated_delta += (perf - game_loop.last_frame) as f32;
            while game_loop.accumulated_delta >= FRAME_SIZE {
                game.update();
                game_loop.accumulated_delta -= FRAME_SIZE;
            }
            game_loop.last_frame = perf;
            game.draw(&renderer);
            browser::request_animation_frame(f.borrow().as_ref().unwrap())
                .expect("Failed to request animation frame");
        }));

        browser::request_animation_frame(
            g.borrow()
                .as_ref()
                .ok_or_else(|| anyhow!("No closure found"))?,
        )?;

        Ok(())
    }
}

pub struct Renderer {
    context: CanvasRenderingContext2d,
}

#[derive(Debug, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Renderer {
    pub fn clear(&self, rect: &Rect) {
        self.context
            .clear_rect(rect.x, rect.y, rect.width, rect.height);
    }

    pub fn draw_image(&self, image: &HtmlImageElement, frame: &Rect, dest: &Rect) {
        self.context
            .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &image,
                frame.x,
                frame.y,
                frame.width,
                frame.height,
                dest.x,
                dest.y,
                dest.width,
                dest.height,
            )
            .expect("Failed to draw image");
    }
}

use crate::browser::fetch_json;
use crate::engine::{load_image, Game, Rect, Renderer};
use anyhow::Result;
use async_trait::async_trait;
use gloo_utils::format::JsValueSerdeExt;
use serde::Deserialize;
use std::collections::HashMap;
use web_sys::HtmlImageElement;

#[derive(Debug, Deserialize)]
struct SheetRect {
    x: i16,
    y: i16,
    w: i16,
    h: i16,
}

#[derive(Debug, Deserialize)]
struct Cell {
    frame: SheetRect,
}

#[derive(Debug, Deserialize)]
pub struct Sheet {
    frames: HashMap<String, Cell>,
}

pub struct WalkTheDog {
    image: Option<HtmlImageElement>,
    sheet: Option<Sheet>,
    frame: u8,
}

impl WalkTheDog {
    pub fn new() -> Self {
        Self {
            image: None,
            sheet: None,
            frame: 0,
        }
    }
}

// const RHB_PATH: &str = "walk_the_dog_assets-0.0.7/resized/rhb/";
const SPRITE_PATH: &str = "walk_the_dog_assets-0.0.7/sprite_sheets/";

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&self) -> Result<Box<dyn Game>> {
        let json = fetch_json((String::from(SPRITE_PATH) + "rhb.json").as_str()).await?;
        let sheet: Sheet = json.into_serde()?;
        let image = load_image((String::from(SPRITE_PATH) + "rhb.png").as_str()).await?;

        Ok(Box::new(WalkTheDog {
            image: Some(image),
            sheet: Some(sheet),
            frame: self.frame,
        }))
    }

    fn update(&mut self) {
        if self.frame < 23 {
            self.frame += 1;
        } else {
            self.frame = 0;
        }
    }

    fn draw(&self, renderer: &Renderer) {
        let current_sprite = (self.frame / 3) + 1;
        let frame_name = format!("Run ({}).png", current_sprite);
        let sprite = self
            .sheet
            .as_ref()
            .and_then(|sheet| sheet.frames.get(&frame_name))
            .expect("Cell not found");

        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 600.0,
        });
        self.image.as_ref().map(|image| {
            renderer.draw_image(
                &image,
                &Rect {
                    x: sprite.frame.x.into(),
                    y: sprite.frame.y.into(),
                    width: sprite.frame.w.into(),
                    height: sprite.frame.h.into(),
                },
                &Rect {
                    x: 300.0,
                    y: 300.0,
                    width: sprite.frame.w.into(),
                    height: sprite.frame.h.into(),
                },
            );
        });
    }
}

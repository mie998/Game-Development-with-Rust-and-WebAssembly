use crate::browser::fetch_json;
use crate::engine::{load_image, Game, Rect, Renderer, KeyState};
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

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

pub struct WalkTheDog {
    image: Option<HtmlImageElement>,
    sheet: Option<Sheet>,
    frame: u8,
    position: Point,
}

impl WalkTheDog {
    pub fn new() -> Self {
        Self {
            image: None,
            sheet: None,
            frame: 0,
            position: Point { x: 0, y: 0 },
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
            position: self.position,
        }))
    }

    fn update(&mut self, keystate: &mut KeyState) {
        if self.frame < 23 {
            self.frame += 1;
        } else {
            self.frame = 0;
        }

        // 入力による速度更新
        let mut velocity = Point { x: 0, y: 0 };
        if keystate.is_pressed("ArrowRight") {
            velocity.x += 10;
        }
        if keystate.is_pressed("ArrowLeft") {
            velocity.x -= 10;
        }
        if keystate.is_pressed("ArrowUp") {
            velocity.y -= 10;
        }
        if keystate.is_pressed("ArrowDown") {
            velocity.y += 10;
        }
        
        self.position.x += velocity.x;
        self.position.y += velocity.y;
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
                    x: self.position.x.into(),
                    y: self.position.y.into(),
                    width: sprite.frame.w.into(),
                    height: sprite.frame.h.into(),
                },
            );
        });
    }
}

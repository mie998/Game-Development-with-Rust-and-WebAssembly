use crate::{
    browser,
    engine::{self, Game, Image, KeyState, Point, Rect, Renderer, Sheet},
    state::red_hat_boy_states::*,
    state::{Event, RedHatBoyStateMachine},
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use gloo_utils::format::JsValueSerdeExt;
use web_sys::HtmlImageElement;

pub const HEIGHT: i16 = 600;

pub struct RedHatBoy {
    state_machine: RedHatBoyStateMachine,
    sprite_sheet: Sheet,
    image: HtmlImageElement,
}

impl RedHatBoy {
    fn new(sprite_sheet: Sheet, image: HtmlImageElement) -> Self {
        RedHatBoy {
            state_machine: RedHatBoyStateMachine::Idle(RedHatBoyState::new()),
            sprite_sheet,
            image,
        }
    }

    fn run_right(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Run);
    }

    fn slide(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Slide);
    }

    fn jump(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Jump);
    }

    fn update(&mut self) {
        self.state_machine = self.state_machine.update();
    }

    fn draw(&self, renderer: &Renderer) {
        let sprite = self.current_sprite().expect("No sprite found");

        renderer.draw_image(
            &self.image,
            &Rect {
                x: sprite.frame.x.into(),
                y: sprite.frame.y.into(),
                width: sprite.frame.w.into(),
                height: sprite.frame.h.into(),
            },
            &self.bounding_box(),
        );

        // debug draw
        renderer.draw_stroke_rect(&self.collision_box());
    }

    fn frame_name(&self) -> String {
        format!(
            "{} ({}).png",
            self.state_machine.frame_name(),
            (self.state_machine.context().frame / 3) + 1
        )
    }

    fn current_sprite(&self) -> Option<&engine::Cell> {
        self.sprite_sheet.frames.get(&self.frame_name())
    }

    fn bounding_box(&self) -> Rect {
        let sprite = self.current_sprite().expect("No sprite found");

        Rect {
            x: (self.state_machine.context().position.x + sprite.sprite_source_size.x).into(),
            y: (self.state_machine.context().position.y + sprite.sprite_source_size.y).into(),
            width: sprite.frame.w.into(),
            height: sprite.frame.h.into(),
        }
    }

    // due to this is only used for collision detection, we can use the smaller sprite's bounding box
    fn collision_box(&self) -> Rect {
        let sprite = self.current_sprite().expect("No sprite found");

        Rect {
            x: (self.state_machine.context().position.x + sprite.sprite_source_size.x + 15).into(),
            y: (self.state_machine.context().position.y + sprite.sprite_source_size.y + 15).into(),
            width: (sprite.frame.w - 30).into(),
            height: (sprite.frame.h - 15).into(),
        }
    }

    fn knock_out(&mut self) {
        self.state_machine = self.state_machine.transition(Event::KnockOut);
    }

    fn land_on(&mut self, position: f32) {
        self.state_machine = self.state_machine.transition(Event::Land(position));
    }

    fn pos_y(&self) -> i16 {
        self.state_machine.context().position.y
    }

    fn velocity_y(&self) -> i16 {
        self.state_machine.context().velocity.y
    }

    fn walking_speed(&self) -> i16 {
        self.state_machine.context().velocity.x
    }
}

pub enum WalkTheDog {
    Loading,
    Loaded(Walk),
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog::Loading {}
    }
}

pub struct Walk {
    boy: RedHatBoy,
    backgrounds: [Image; 2],
    stone: Image,
    platform: Platform,
}

impl Walk {
    fn velocity(&self) -> i16 {
        -self.boy.walking_speed()
    }
}

struct Platform {
    sheet: Sheet,
    image: HtmlImageElement,
    position: Point,
}

const LOW_PLATFORM: i16 = 420;
const HIGH_PLATFORM: i16 = 375;
impl Platform {
    fn new(sheet: Sheet, image: HtmlImageElement, position: Point) -> Self {
        Platform {
            sheet,
            image,
            position,
        }
    }

    fn draw(&self, renderer: &Renderer) {
        let platform = self.sheet.frames.get("13.png").expect("No 13.png found");

        renderer.draw_image(
            &self.image,
            &Rect {
                x: platform.frame.x.into(),
                y: platform.frame.y.into(),
                width: (platform.frame.w * 3).into(),
                height: platform.frame.h.into(),
            },
            &self.bounding_box(),
        );

        // debug
        for collision_box in self.collision_boxes() {
            renderer.draw_stroke_rect(&collision_box);
        }
    }

    fn bounding_box(&self) -> Rect {
        let platform = self.sheet.frames.get("13.png").expect("No 13.png found");

        Rect {
            x: self.position.x.into(),
            y: self.position.y.into(),
            width: (platform.frame.w * 3).into(),
            height: platform.frame.h.into(),
        }
    }

    fn collision_boxes(&self) -> Vec<Rect> {
        const X_OFFSET: f32 = 60.0;
        const END_HEIGHT: f32 = 54.0;
        let bb = self.bounding_box();

        vec![
            // left
            Rect {
                x: bb.x,
                y: bb.y,
                width: X_OFFSET,
                height: END_HEIGHT,
            },
            // center
            Rect {
                x: bb.x + X_OFFSET,
                y: bb.y,
                width: bb.width - (X_OFFSET * 2.0),
                height: bb.height,
            },
            // right
            Rect {
                x: bb.x + bb.width - X_OFFSET,
                y: bb.y,
                width: X_OFFSET,
                height: END_HEIGHT,
            },
        ]
    }
}

const SPRITE_PATH: &str = "walk_the_dog_assets-0.0.7/sprite_sheets/";
const BG_PATH: &str = "walk_the_dog_assets-0.0.7/resized/freetileset/png/BG/";
const OBJECT_PATH: &str = "walk_the_dog_assets-0.0.7/resized/freetileset/png/Object/";

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&self) -> Result<Box<dyn Game>> {
        match self {
            WalkTheDog::Loading => {
                let sheet: Sheet =
                    browser::fetch_json((String::from(SPRITE_PATH) + "rhb_trimmed.json").as_str())
                        .await?
                        .into_serde()?;
                let background =
                    engine::load_image((String::from(BG_PATH) + "BG.png").as_str()).await?;
                let stone =
                    engine::load_image((String::from(OBJECT_PATH) + "Stone.png").as_str()).await?;
                let platform_sheet =
                    browser::fetch_json((String::from(SPRITE_PATH) + "tiles.json").as_str())
                        .await?;

                let platform = Platform::new(
                    platform_sheet.into_serde::<Sheet>()?,
                    engine::load_image((String::from(SPRITE_PATH) + "tiles.png").as_str()).await?,
                    Point {
                        x: 400,
                        y: HIGH_PLATFORM,
                    },
                );

                let rhb = RedHatBoy::new(
                    sheet,
                    engine::load_image((String::from(SPRITE_PATH) + "rhb_trimmed.png").as_str())
                        .await?,
                );

                let background_width = background.width();

                Ok(Box::new(WalkTheDog::Loaded(Walk {
                    boy: rhb,
                    backgrounds: [
                        Image::new(background.clone(), Point { x: 0, y: 0 }),
                        Image::new(
                            background,
                            Point {
                                x: background_width as i16,
                                y: 0,
                            },
                        ),
                    ],
                    stone: Image::new(stone, Point { x: 180, y: 546 }),
                    platform,
                })))
            }
            WalkTheDog::Loaded(_) => Err(anyhow!("Error: Game is already initialized")),
        }
    }

    fn update(&mut self, keystate: &KeyState) {
        if let WalkTheDog::Loaded(walk) = self {
            if keystate.is_pressed("ArrowRight") {
                walk.boy.run_right();
            }

            if keystate.is_pressed("ArrowUp") {
                walk.boy.jump();
            }

            if keystate.is_pressed("ArrowDown") {
                walk.boy.slide();
            }

            let velocity = walk.velocity();
            
            walk.boy.update();
            walk.platform.position.x += velocity;
            walk.stone.move_horizontally(velocity);
            
            let [bg1, bg2] = &mut walk.backgrounds;
            bg1.move_horizontally(velocity);
            bg2.move_horizontally(velocity);

            if bg1.right() < 0 {
                bg1.set_x(bg2.right());
            }
            if bg2.right() < 0 {
                bg2.set_x(bg1.right());
            }

            // collision detection with platform
            for cb in &walk.platform.collision_boxes() {
                if walk.boy.bounding_box().intersects(cb) {
                    if walk.boy.velocity_y() > 0 && walk.boy.pos_y() < cb.y as i16 {
                        walk.boy.land_on(cb.y);
                    } else {
                        walk.boy.knock_out();
                    }
                }
            }

            if walk.boy.bounding_box().intersects(&walk.stone.bounding_box) {
                // walk.boy.knock_out();
            }
        }
    }

    fn draw(&self, renderer: &Renderer) {
        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 600.0,
        });

        if let WalkTheDog::Loaded(walk) = self {
            walk.backgrounds.iter().for_each(|bg| bg.draw(renderer));
            walk.boy.draw(renderer);
            walk.stone.draw(renderer);
            walk.platform.draw(renderer);
        }
    }
}

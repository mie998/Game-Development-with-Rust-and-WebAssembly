use crate::{
    browser,
    engine::{self, Cell, Game, Image, KeyState, Point, Rect, Renderer, Sheet, SpriteSheet},
    segments::*,
    sound::{Audio, Sound},
    state::red_hat_boy_states::*,
    state::{Event, RedHatBoyStateMachine},
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use gloo_utils::format::JsValueSerdeExt;
use rand::prelude::*;
use std::rc::Rc;
use web_sys::HtmlImageElement;

pub const HEIGHT: i16 = 600;
pub const TIMELINE_MINIMUM: i16 = 1000;
pub const OBSTACLE_BUFFER: i16 = 20;

#[derive(Clone)]
pub struct RedHatBoy {
    state_machine: RedHatBoyStateMachine,
    sprite_sheet: Sheet,
    image: HtmlImageElement,
}

impl RedHatBoy {
    fn new(sprite_sheet: Sheet, image: HtmlImageElement, audio: Audio, sound: Sound) -> Self {
        RedHatBoy {
            state_machine: RedHatBoyStateMachine::Idle(RedHatBoyState::new(audio, sound)),
            sprite_sheet,
            image,
        }
    }

    fn run_right(&mut self) {
        self.state_machine = self.state_machine.clone().transition(Event::Run);
    }

    fn slide(&mut self) {
        self.state_machine = self.state_machine.clone().transition(Event::Slide);
    }

    fn jump(&mut self) {
        self.state_machine = self.state_machine.clone().transition(Event::Jump);
    }

    fn update(&mut self) {
        self.state_machine = self.state_machine.clone().update();
    }

    fn draw(&self, renderer: &Renderer) {
        let sprite = self.current_sprite().expect("No sprite found");

        renderer.draw_image(
            &self.image,
            &Rect::new(
                Point::new(sprite.frame.x.into(), sprite.frame.y.into()),
                sprite.frame.w.into(),
                sprite.frame.h.into(),
            ),
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

    fn current_sprite(&self) -> Option<&Cell> {
        self.sprite_sheet.frames.get(&self.frame_name())
    }

    fn bounding_box(&self) -> Rect {
        let sprite = self.current_sprite().expect("No sprite found");

        Rect::new(
            Point::new(
                (self.state_machine.context().position.x + sprite.sprite_source_size.x).into(),
                (self.state_machine.context().position.y + sprite.sprite_source_size.y).into(),
            ),
            sprite.frame.w.into(),
            sprite.frame.h.into(),
        )
    }

    // due to this is only used for collision detection, we can use the smaller sprite's bounding box
    fn collision_box(&self) -> Rect {
        let sprite = self.current_sprite().expect("No sprite found");

        Rect::new(
            Point::new(
                (self.state_machine.context().position.x + sprite.sprite_source_size.x + 15).into(),
                (self.state_machine.context().position.y + sprite.sprite_source_size.y + 15).into(),
            ),
            (sprite.frame.w - 30).into(),
            (sprite.frame.h - 15).into(),
        )
    }

    fn knock_out(&mut self) {
        self.state_machine = self.state_machine.clone().transition(Event::KnockOut);
    }

    fn land_on(&mut self, position: i16) {
        self.state_machine = self.state_machine.clone().transition(Event::Land(position));
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
    obstacles: Vec<Box<dyn Obstacle>>,
    obstacle_sheet: Rc<SpriteSheet>,
    stone: HtmlImageElement,
    timeline: i16,
}

impl Walk {
    fn velocity(&self) -> i16 {
        -self.boy.walking_speed()
    }

    fn generate_next_segment(&mut self) {
        let mut rng = thread_rng();
        let next_segment = rng.gen_range(0..2);

        let mut next_obstacles = match next_segment {
            0 => stone_and_platform(
                self.stone.clone(),
                self.obstacle_sheet.clone(),
                self.timeline + OBSTACLE_BUFFER,
            ),
            1 => platform_and_stone(
                self.stone.clone(),
                self.obstacle_sheet.clone(),
                self.timeline + OBSTACLE_BUFFER,
            ),
            _ => vec![],
        };

        self.timeline = rightmost(&next_obstacles);
        self.obstacles.append(&mut next_obstacles);
    }
}

pub struct Platform {
    sheet: Rc<SpriteSheet>,
    bounding_boxes: Vec<Rect>,
    sprites: Vec<Cell>,
    position: Point,
}

impl Platform {
    pub fn new(
        sheet: Rc<SpriteSheet>,
        position: Point,
        sprite_names: &[&str],
        bounding_boxes: &[Rect],
    ) -> Self {
        let sprites = sprite_names
            .iter()
            .filter_map(|name| sheet.cell(name).clone())
            .collect();
        let bounding_boxes = bounding_boxes
            .iter()
            .map(|bb| {
                Rect::new_from_x_y(
                    bb.x() + position.x,
                    bb.y() + position.y,
                    bb.width,
                    bb.height,
                )
            })
            .collect();

        Platform {
            sheet,
            position,
            sprites,
            bounding_boxes,
        }
    }

    fn collision_boxes(&self) -> &Vec<Rect> {
        &self.bounding_boxes
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
                let stone_image =
                    engine::load_image((String::from(OBJECT_PATH) + "Stone.png").as_str()).await?;
                let tiles =
                    browser::fetch_json((String::from(SPRITE_PATH) + "tiles.json").as_str())
                        .await?;
                let sprite_sheet = Rc::new(SpriteSheet::new(
                    engine::load_image((String::from(SPRITE_PATH) + "tiles.png").as_str()).await?,
                    tiles.into_serde::<Sheet>()?,
                ));

                let audio = Audio::new()?;
                let sound = audio
                    .load_sound("walk_the_dog_assets-0.0.7/sounds/SFX_Jump_23.mp3")
                    .await?;

                let rhb = RedHatBoy::new(
                    sheet,
                    engine::load_image((String::from(SPRITE_PATH) + "rhb_trimmed.png").as_str())
                        .await?,
                    audio,
                    sound,
                );

                let starting_obstacles =
                    stone_and_platform(stone_image.clone(), sprite_sheet.clone(), 0);
                let timeline = rightmost(&starting_obstacles);

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
                    obstacles: starting_obstacles,
                    obstacle_sheet: sprite_sheet,
                    stone: stone_image,
                    timeline,
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

            // remove all obstacles that are out of screen
            walk.obstacles.retain(|obstacle| obstacle.right() > 0);

            // !NOTE: This is a workaround for borrow checker.
            // Expect future Rust update to fix this issue.
            // ref: https://stackoverflow.com/questions/64921625/closure-requires-unique-access-to-self-but-it-is-already-borrowed
            let mut obstacles = std::mem::take(&mut walk.obstacles);
            obstacles.iter_mut().for_each(|obstacle| {
                obstacle.move_horizontally(velocity);
                obstacle.check_intersection(&mut walk.boy);
            });
            walk.obstacles = obstacles;

            let [bg1, bg2] = &mut walk.backgrounds;
            bg1.move_horizontally(velocity);
            bg2.move_horizontally(velocity);

            if bg1.right() < 0 {
                bg1.set_x(bg2.right());
            }
            if bg2.right() < 0 {
                bg2.set_x(bg1.right());
            }

            if walk.timeline < TIMELINE_MINIMUM {
                walk.generate_next_segment();
            } else {
                walk.timeline += velocity;
            }
        }
    }

    fn draw(&self, renderer: &Renderer) {
        renderer.clear(&Rect::new(Point::new(0, 0), 600, 600));

        if let WalkTheDog::Loaded(walk) = self {
            walk.backgrounds.iter().for_each(|bg| bg.draw(renderer));
            walk.boy.draw(renderer);
            walk.obstacles.iter().for_each(|obstacle| {
                obstacle.draw(renderer);
            });
        }
    }
}

pub trait Obstacle {
    fn draw(&self, renderer: &Renderer);
    fn check_intersection(&self, boy: &mut RedHatBoy);
    fn move_horizontally(&mut self, x: i16);
    fn right(&self) -> i16;
}

impl Obstacle for Platform {
    fn draw(&self, renderer: &Renderer) {
        let mut x = 0;
        self.sprites.iter().for_each(|sprite| {
            self.sheet.draw(
                renderer,
                &Rect::new_from_x_y(
                    sprite.frame.x,
                    sprite.frame.y,
                    sprite.frame.w,
                    sprite.frame.h,
                ),
                &Rect::new_from_x_y(
                    self.position.x + x,
                    self.position.y,
                    sprite.frame.w,
                    sprite.frame.h,
                ),
            );
            x += sprite.frame.w;
        });

        // debug
        for collision_box in self.collision_boxes() {
            renderer.draw_stroke_rect(&collision_box);
        }
    }

    fn move_horizontally(&mut self, x: i16) {
        self.position.x += x;
        self.bounding_boxes
            .iter_mut()
            .for_each(|cb| cb.set_x(cb.x() + x))
    }

    fn check_intersection(&self, boy: &mut RedHatBoy) {
        if let Some(box_to_land_on) = self
            .collision_boxes()
            .iter()
            .find(|&cb| boy.bounding_box().intersects(cb))
        {
            if boy.velocity_y() > 0 && boy.pos_y() < self.position.y {
                boy.land_on(box_to_land_on.y());
            } else {
                boy.knock_out();
            }
        }
    }

    // Max right value of all collision boxes
    fn right(&self) -> i16 {
        self.collision_boxes()
            .iter()
            .map(|cb| cb.right())
            .max()
            .unwrap()
    }
}

pub struct Barrier {
    image: Image,
}

impl Barrier {
    pub fn new(image: Image) -> Self {
        Self { image }
    }
}

impl Obstacle for Barrier {
    fn draw(&self, renderer: &Renderer) {
        self.image.draw(renderer);
    }

    fn move_horizontally(&mut self, x: i16) {
        self.image.move_horizontally(x);
    }

    fn check_intersection(&self, boy: &mut RedHatBoy) {
        if boy.bounding_box().intersects(&self.image.bounding_box) {
            boy.knock_out();
        }
    }

    fn right(&self) -> i16 {
        self.image.right()
    }
}

fn rightmost(obstacle_list: &Vec<Box<dyn Obstacle>>) -> i16 {
    obstacle_list
        .iter()
        .map(|obstacle| obstacle.right())
        .max_by(|x, y| x.cmp(&y))
        .unwrap()
}

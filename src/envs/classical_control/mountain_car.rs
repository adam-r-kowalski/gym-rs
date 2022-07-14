use std::fmt::Debug;
use std::iter::zip;

use crate::core::{ActionReward, Env};
use crate::spaces::{self, Discrete, Space};
use crate::utils::math_ops;
use crate::utils::renderer::{Render, RenderMode, Renderer};
use crate::utils::seeding::rand_random;
use derivative::Derivative;
use derive_new::new;
use log::debug;
use na::{Point2, Rotation2};
use nalgebra as na;
use rand::distributions::Uniform;
use rand::Rng;
use rand_pcg::Pcg64;
use sdl2::event::Event;
use sdl2::gfx::framerate::FPSManager;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Point;
use sdl2::render::WindowCanvas;
use sdl2::sys::gfx::framerate::{FPSmanager, FPS_DEFAULT};
use sdl2::{EventPump, EventSubsystem, Sdl, TimerSubsystem};
use serde::Serialize;

///Description:
///
///  The agent (a car) is started at the bottom of a valley. For any given
///  state, the agent may choose to accelerate to the left, right or cease
///  any acceleration.
///
///Source:
///
///  The environment appeared first in Andrew Moore's PhD Thesis (1990).
///  the source code in python: https://www.github.com/openai/gym
///
///Observation:
///
///  Num     Observation         Min     Max
///  0       Car Position        -1.2    0.6
///  1       Car Velocity        -0.07   0.07
///
///Actions:
///
///  In case of discrete action:
///  Num    Action
///  0      Accelerate to the left
///  1      Don't accelerate
///  2      Accelerate to the right
///
///  In case of continuous action, apply a force in range [-1.0, 1.0],
///  1.0 being maximum acceleration to the right,
///  -1.0 being maximum acceleration to the left
///
///  Note: This does not affect the amount of velocity affected by the gravitational pull acting on the cart.
///
///Reward:
///
///  Reward of 0 is awarded if the agent reached the flag (position = 0.5)
///  on top of the mountain.
///  Reward of -1 is awarded if the position of the agent is less than 0.5.
///
///Starting State:
///
///  The position of the car is assigned a uniform random value in
///  [-0.6, -0.4].
///  The starting velocity of the car is always assigned to 0.
///
///Episode Termination:
///
///  The car position is more than 0.5
///  Episode length is greater than 200
#[derive(Serialize, Derivative)]
#[derivative(Debug)]
pub struct MountainCarEnv<'a> {
    /// TODO
    pub min_position: f64,
    /// TODO
    pub max_position: f64,
    /// TODO
    pub max_speed: f64,
    /// TODO
    pub goal_position: f64,
    /// TODO
    pub goal_velocity: f64,

    /// TODO
    pub force: f64,
    /// TODO
    pub gravity: f64,

    /// TODO
    pub low: Observation,
    /// TODO
    pub high: Observation,

    /// TODO
    pub render_mode: RenderMode,
    /// TODO
    pub renderer: Renderer<'a>,

    // NOTE: Consider using SDL2 to reduce differences between gym_rs and the python implementation.
    /// TODO
    pub screen_width: u32,
    /// TODO
    pub screen_height: u32,
    /// TODO
    #[serde(skip_serializing)]
    #[derivative(Debug = "ignore")]
    pub screen: Option<Screen>,
    /// TODO
    pub isopen: bool,

    /// TODO
    pub action_space: spaces::Discrete,
    /// TODO
    pub observation_space: spaces::Box<Observation>,

    /// TODO
    pub state: Observation,

    /// RANDOM NUMBER GENERATOR
    #[serde(skip_serializing)]
    rand_random: Pcg64,
    metadata: MountainCarMetadata,
}

const MOUNTAIN_CAR_RENDER_MODES: &'static [RenderMode] = &[
    RenderMode::Human,
    RenderMode::RgbArray,
    RenderMode::SingleRgbArray,
    RenderMode::None,
];

#[derive(Debug, Clone, Serialize)]
pub struct MountainCarMetadata {
    render_modes: &'static [RenderMode],
    render_fps: u32,
}

impl Default for MountainCarMetadata {
    fn default() -> Self {
        Self {
            render_modes: MOUNTAIN_CAR_RENDER_MODES,
            render_fps: 30,
        }
    }
}

/// Utility structure intended to reduce confusion around meaning of properties.
#[derive(Debug, new, Copy, Clone, Serialize)]
pub struct Observation {
    position: f64,
    velocity: f64,
}

impl From<Observation> for Vec<f64> {
    fn from(o: Observation) -> Self {
        vec![o.position, o.velocity]
    }
}

impl Default for Observation {
    fn default() -> Self {
        Observation {
            position: 0.,
            velocity: 0.,
        }
    }
}

impl Observation {
    /// TODO
    pub fn update(&mut self, position: f64, velocity: f64) {
        self.position = position;
        self.velocity = velocity;
    }
}

pub struct Screen {
    pub canvas: WindowCanvas,
    pub fps_manager: FPSManager,
    pub event_pump: EventPump,
    pub event_subsystem: EventSubsystem,
}

impl<'a> MountainCarEnv<'a> {
    fn height(xs: &Vec<f64>) -> Vec<f64> {
        xs.clone()
            .iter()
            .map(|value| (3. * value).sin() * 0.45 + 0.55)
            .collect()
    }

    pub fn new(render_mode: RenderMode, goal_velocity: Option<f64>) -> Self {
        let (rand_random, _) = rand_random(None);

        let min_position = -1.2;
        let max_position = 0.6;
        let max_speed = 0.07;
        let goal_position = 0.5;
        let goal_velocity = goal_velocity.unwrap_or(0.);

        let force = 0.001;
        let gravity = 0.0025;

        let low = Observation::new(min_position, -max_speed);
        let high = Observation::new(max_position, max_speed);

        let renderer = Renderer::new(render_mode, None, None);

        // NOTE: Since rust requires statically typed properties, state must explicitly initiated or lazy
        // loaded via function (the later would deviate more from the current interface, so we
        // shouldn't use it).
        let state = Observation::default();

        let screen_width = 600;
        let screen_height = 400;
        let screen = None;
        let isopen = false;

        let action_space = spaces::Discrete(3);
        let observation_space = spaces::Box::new(low, high);

        let metadata = MountainCarMetadata::default();

        Self {
            min_position,
            max_position,
            max_speed,
            goal_position,
            goal_velocity,

            force,
            gravity,

            low,
            high,

            render_mode,
            renderer,

            action_space,
            observation_space,

            state,
            rand_random,

            screen,
            screen_width,
            screen_height,
            isopen,

            metadata,
        }
    }
}

impl<'a> Env for MountainCarEnv<'a> {
    type Action = usize;
    type Observation = Observation;
    type Info = String;
    type Metadata = MountainCarMetadata;
    type ActionSpace = Discrete;
    type ObservationSpace = spaces::Box<Self::Observation>;

    fn step(&mut self, action: Self::Action) -> ActionReward<Self::Observation, Self::Info> {
        assert!(
            self.action_space.contains(action),
            "{} (usize) invalid",
            action
        );

        let mut position = self.state.position;
        let mut velocity = self.state.velocity;

        velocity +=
            (action as isize - 1) as f64 * self.force + (3.0 * position).cos() * (-self.gravity);
        velocity = math_ops::clip(velocity, -self.max_speed, self.max_speed);

        position += velocity;
        position = math_ops::clip(position, self.min_position, self.max_position);

        if position == self.min_position && velocity < 0.0 {
            velocity = 0.0;
        }

        let done: bool = position >= self.goal_position && velocity >= self.goal_velocity;
        let reward: f64 = -1.0;

        self.state.update(position, velocity);
        self.render(self.render_mode);

        ActionReward {
            observation: self.state,
            reward,
            done,
            info: None,
        }
    }

    fn reset(&mut self) -> Self::Observation {
        let initial_position = Uniform::new::<f64, f64>(-0.6, -0.4);
        self.state = Observation::new(self.rand_random.sample(initial_position), 0.0);
        self.state
    }

    fn render(&mut self, mode: RenderMode) -> Render {
        assert!(self.metadata.render_modes.contains(&mode));

        let screen_width = self.screen_width;
        let screen_height = self.screen_height;
        let max_position = self.max_position;
        let min_position = self.min_position;
        let goal_position = self.goal_position;
        let state = self.state;
        let fps = self.metadata().render_fps;

        self.screen.get_or_insert_with(|| {
            let context = sdl2::init().unwrap();
            let video_subsystem = context.video().unwrap();
            let mut window_builder =
                video_subsystem.window("Mountain Car", screen_width, screen_height);

            window_builder.position_centered();

            if mode != RenderMode::Human {
                window_builder.hidden();
            }

            let window = window_builder.build().unwrap();
            let canvas = window.into_canvas().accelerated().build().unwrap();
            let event_pump = context.event_pump().expect("Could not recieve event pump.");
            let mut fps_manager = FPSManager::new();
            let event_subsystem = context
                .event()
                .expect("Event subsystem was not initialized.");
            fps_manager
                .set_framerate(fps)
                .expect("Framerate was unable to be set.");

            let screen = Screen {
                canvas,
                event_pump,
                event_subsystem,
                fps_manager,
            };

            screen
        });

        let world_width = max_position - min_position;
        let scale = screen_width as f64 / world_width;
        let carwidth = 40;
        let carheight = 20;

        let screen = self.screen.as_mut().unwrap();
        let canvas = &mut screen.canvas;
        let creator = canvas.texture_creator();
        let fps_manager = &mut screen.fps_manager;
        let events = &mut screen.event_pump;

        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    panic!("MOUNTAIN CAR ANIMATION WAS FORCED TO EXIT!")
                }
                _ => (),
            }
        }

        let mut texture = creator
            .create_texture_target(
                PixelFormatEnum::RGB24,
                self.screen_width,
                self.screen_height,
            )
            .unwrap();

        canvas
            .with_texture_canvas(&mut texture, |texture_canvas| {
                texture_canvas.set_draw_color(Color::WHITE);
                texture_canvas.clear();

                let pos = state.position;

                let xs: Vec<f64> = (0..100)
                    .into_iter()
                    .map(|index| (((max_position - min_position) / 100.) * index as f64))
                    .map(|value| value + min_position)
                    .collect();

                let ys: Vec<f64> = Self::height(&xs);
                let xys: Vec<Point> = zip(
                    xs.iter().map(|value| (value - min_position) * scale),
                    ys.iter().map(|value| value * scale),
                )
                .map(|(x, y)| Point::new(x.floor() as i32, y.floor() as i32))
                .collect();

                texture_canvas.set_draw_color(Color::BLACK);
                texture_canvas.draw_lines(xys.as_slice()).unwrap();

                debug!("SCALE: {}", scale);
                debug!("X: {:?}", xs[0..5].to_vec());
                debug!("Y: {:?}", ys[0..5].to_vec());
                debug!("{:?}", xys[0..5].to_vec());

                let clearance = 10f64;

                let (l, r, t, b) = (-carwidth / 2, carwidth / 2, carheight, 0);
                let coords = [(l, b), (l, t), (r, t), (r, b)].map(|(x, y)| {
                    let point = Point2::new(x as f64, y as f64);
                    let desired_angle = (3. * pos).cos();
                    let rotation_matrix = Rotation2::new(desired_angle);
                    let rotated_point = rotation_matrix.transform_point(&point);

                    let (x, y) = (rotated_point.x, rotated_point.y);

                    let new_x = x + (pos - min_position) * scale;
                    let new_y = y + clearance + Self::height(&vec![pos]).pop().unwrap() * scale;

                    (new_x, new_y)
                });

                let coords_x = coords.map(|coord| coord.0.floor() as i16);
                let coords_y = coords.map(|coord| coord.1.floor() as i16);

                texture_canvas
                    .aa_polygon(&coords_x, &coords_y, Color::BLACK)
                    .unwrap();

                texture_canvas
                    .filled_polygon(&coords_x, &coords_y, Color::BLACK)
                    .unwrap();

                for (x, y) in [(carwidth as f64 / 4., 0.), ((-carwidth as f64 / 4.), 0.)] {
                    let point = Point2::new(x as f64, y as f64);
                    let desired_angle = (3. * pos).cos();
                    let rotation_matrix = Rotation2::new(desired_angle);
                    let rotated_point = rotation_matrix.transform_point(&point);

                    let (x, y) = (rotated_point.x, rotated_point.y);

                    let (wheel_x, wheel_y) = (
                        (x + (pos - min_position) * scale).floor() as i16,
                        (y + clearance + Self::height(&vec![pos]).pop().unwrap() * scale).floor()
                            as i16,
                    );

                    let rad = (carheight as f64 / 2.5).floor() as i16;

                    texture_canvas
                        .aa_circle(wheel_x, wheel_y, rad, Color::RGB(128, 128, 128))
                        .unwrap();

                    texture_canvas
                        .filled_circle(wheel_x, wheel_y, rad, Color::RGB(128, 128, 128))
                        .unwrap();
                }

                let flagx = ((goal_position - min_position) * scale).floor() as i16;
                let flagy1 =
                    (Self::height(&vec![goal_position]).pop().unwrap() * scale).floor() as i16;
                let flagy2 = flagy1 + 50;
                texture_canvas
                    .vline(flagx, flagy1, flagy2, Color::BLACK)
                    .unwrap();

                texture_canvas
                    .aa_polygon(
                        &vec![flagx, flagx, flagx + 25],
                        &vec![flagy2, flagy2 - 10, flagy2 - 5],
                        Color::RGB(204, 204, 0),
                    )
                    .unwrap();
                texture_canvas
                    .filled_polygon(
                        &vec![flagx, flagx, flagx + 25],
                        &vec![flagy2, flagy2 - 10, flagy2 - 5],
                        Color::RGB(204, 204, 0),
                    )
                    .unwrap();
            })
            .unwrap();

        canvas
            .copy_ex(&texture, None, None, 0., None, false, true)
            .unwrap();

        canvas.present();
        fps_manager.delay();

        Render::Human
    }

    fn seed(&mut self, seed: Option<u64>) -> u64 {
        let (new_rng, new_rng_seed) = rand_random(seed);
        self.rand_random = new_rng;
        new_rng_seed
    }

    fn rand_random(&self) -> &Pcg64 {
        &self.rand_random
    }

    fn metadata(&self) -> &Self::Metadata {
        &self.metadata
    }

    fn action_space(&self) -> &Self::ActionSpace {
        &self.action_space
    }

    fn observation_space(&self) -> &Self::ObservationSpace {
        &self.observation_space
    }

    fn close(&mut self) {
        self.screen.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    #[test]
    fn mountain_car() {
        let mut mc = MountainCarEnv::new(RenderMode::Human, None);
        let _state = mc.reset();

        let mut rng = thread_rng();
        let mut end: bool = false;
        let mut episode_length = 0;
        while !end {
            if episode_length > 200 {
                break;
            }
            let action = rng.gen_range(0, 3);
            let ActionReward { done, .. } = mc.step(action);
            episode_length += 1;
            end = done;
            println!("episode_length: {}", episode_length);
        }

        mc.close();

        for _ in 0..200 {
            let action = rng.gen_range(0, 3);
            mc.step(action);
            episode_length += 1;
            println!("episode_length: {}", episode_length);
        }
    }
}

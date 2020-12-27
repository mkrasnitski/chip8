use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::chip8::{CHIP8_HEIGHT, CHIP8_WIDTH};
const SCALE_FACTOR: usize = 10;
const SCREEN_WIDTH: usize = CHIP8_WIDTH * SCALE_FACTOR;
const SCREEN_HEIGHT: usize = CHIP8_HEIGHT * SCALE_FACTOR;

pub struct Display {
    event_pump: sdl2::EventPump,
    canvas: Canvas<Window>,
}

impl Display {
    pub fn new(context: sdl2::Sdl) -> Self {
        let video_subsystem = context.video().unwrap();

        let window = video_subsystem
            .window("CHIP8", SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
            .position_centered()
            .build()
            .unwrap();

        return Display {
            event_pump: context.event_pump().unwrap(),
            canvas: window.into_canvas().build().unwrap(),
        };
    }

    pub fn draw(&mut self, x: &[[u8; CHIP8_WIDTH]; CHIP8_HEIGHT]) {
        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();
        for i in 0..64 {
            for j in 0..32 {
                match x[j][i] {
                    1 => {
                        self.canvas.set_draw_color(Color::WHITE);
                        let _ = self.canvas.fill_rect(Rect::new(
                            (i * SCALE_FACTOR) as i32,
                            (j * SCALE_FACTOR) as i32,
                            SCALE_FACTOR as u32,
                            SCALE_FACTOR as u32,
                        ));
                    }
                    _ => continue,
                }
            }
        }
        self.canvas.present();
    }

    pub fn poll_events(&mut self) -> Option<(String, bool)> {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return Some((String::new(), true)),
                Event::KeyDown {
                    keycode: Some(x),
                    repeat: false,
                    ..
                } => return Some((x.name(), true)),
                Event::KeyUp {
                    keycode: Some(x), ..
                } => return Some((x.name(), false)),
                _ => {}
            }
        }
        return None;
    }
}

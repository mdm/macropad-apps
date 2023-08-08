use chip8::{Chip8, Error};
use embassy_rp::clocks::RoscRng;
use embassy_sync::pubsub::{DynSubscriber, WaitResult};
use embassy_time::{Duration, Ticker};
use embedded_graphics::{
    image::ImageRaw,
    pixelcolor::BinaryColor,
    prelude::{ImageDrawable, Point, Size},
    primitives::{Primitive, PrimitiveStyleBuilder, Rectangle},
    Drawable,
};
use rand::rngs::SmallRng;
use rand::Rng;
use sh1106::{interface::DisplayInterface, prelude::GraphicsMode};

use crate::{
    input_handler::{InputEvent, InputSource},
    INPUT_CHANNEL,
};

const VRAM_WIDTH: usize = 64;
const VRAM_HEIGHT: usize = 32;

pub struct Chip8Harness<'i> {
    input_subscriber: DynSubscriber<'i, InputEvent>,
    emulator: Chip8<SmallRng>,
    active_keys: [bool; 16],
}

impl<'i> Chip8Harness<'i> {
    pub fn new() -> Self {
        let input_subscriber = INPUT_CHANNEL.dyn_subscriber().unwrap();
        let emulator = Chip8::new(RoscRng.gen::<u64>());
        let active_keys = [false; 16];

        Chip8Harness {
            input_subscriber,
            emulator,
            active_keys,
        }
    }

    pub async fn run<DI>(
        &mut self,
        rom: &[u8],
        keymap: [usize; 12],
        display: &mut GraphicsMode<DI>,
    ) -> Result<(), Error>
    where
        DI: DisplayInterface,
        <DI as DisplayInterface>::Error: core::fmt::Debug,
    {
        self.emulator.load_rom(rom)?;
        let mut ticker = Ticker::every(Duration::from_micros(16_667));

        loop {
            if let Some(wait_result) = self.input_subscriber.try_next_message() {
                match wait_result {
                    WaitResult::Message(InputEvent::Pressed(InputSource::Key(key))) => {
                        self.active_keys[keymap[key]] = true;
                    }
                    WaitResult::Message(InputEvent::Released(InputSource::Key(key))) => {
                        self.active_keys[keymap[key]] = false;
                    }
                    _ => {}
                }
                continue;
            }

            let keypad = self
                .active_keys
                .iter()
                .enumerate()
                .map(|(i, key)| if *key { 1 << i } else { 0 })
                .sum();
            self.emulator.frame(keypad)?;

            let framebuffer = self.emulator.fb();
            ImageRaw::<BinaryColor>::new(&framebuffer, chip8::SCREEN_WIDTH as u32 * 2)
                .draw(display)
                .unwrap();

            display.flush().unwrap();
            ticker.next().await;
        }
    }
}

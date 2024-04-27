use embassy_futures::select::{select, Either};
use embassy_rp::{
    gpio::{AnyPin, Input, Pull},
    pio::Instance,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, pubsub::Publisher};
use embassy_time::{Duration, Timer};

use crate::rotary_io::RotaryIO;

#[derive(Clone)]
pub enum InputSource {
    Button,
    Key(usize),
}

#[derive(Clone)]
pub enum InputEvent {
    Pressed(InputSource),
    Released(InputSource),
    TurnedCW(i32),
    TurnedCCW(i32),
}

const NUM_KEYS: usize = 12;

pub struct InputHandler<'a, P: Instance, const S: usize, const CAP: usize, const SUBS: usize> {
    button_input: Input<'a>,
    button_active: bool,
    key_inputs: [Input<'a>; NUM_KEYS],
    key_active: [bool; NUM_KEYS],
    rotary_io: RotaryIO<'a, P, S>,
    encoder_position: i32,
    publisher: Publisher<'a, ThreadModeRawMutex, InputEvent, CAP, SUBS, 1>,
}

impl<'a, P: Instance, const S: usize, const CAP: usize, const SUBS: usize>
    InputHandler<'a, P, S, CAP, SUBS>
{
    pub fn new(
        button: AnyPin,
        keys: [AnyPin; NUM_KEYS],
        rotary_io: RotaryIO<'a, P, S>,
        publisher: Publisher<'a, ThreadModeRawMutex, InputEvent, CAP, SUBS, 1>,
    ) -> Self {
        let button_input = Input::new(button, Pull::Up);
        let button_active = false;

        let key_inputs = keys.map(|key| Input::new(key, Pull::Up));
        let key_active = [false; NUM_KEYS];

        let encoder_position = 0;

        InputHandler {
            button_input,
            button_active,
            key_inputs,
            key_active,
            rotary_io,
            encoder_position,
            publisher,
        }
    }

    pub async fn run(&mut self) {
        let interval = Duration::from_millis(100);
        loop {
            if !self.button_active && self.button_input.is_low() {
                self.button_active = true;
                self.publisher
                    .publish(InputEvent::Pressed(InputSource::Button))
                    .await;
            }
            if self.button_active && self.button_input.is_high() {
                self.button_active = false;
                self.publisher
                    .publish(InputEvent::Released(InputSource::Button))
                    .await;
            }

            for (i, key_input) in self.key_inputs.iter().enumerate() {
                if !self.key_active[i] && key_input.is_low() {
                    self.key_active[i] = true;
                    self.publisher
                        .publish(InputEvent::Pressed(InputSource::Key(i)))
                        .await;
                }
                if self.key_active[i] && key_input.is_high() {
                    self.key_active[i] = false;
                    self.publisher
                        .publish(InputEvent::Released(InputSource::Key(i)))
                        .await;
                }
            }

            match select(
                Timer::after(interval),
                self.rotary_io.wait_position_change(),
            )
            .await
            {
                Either::First(_) => {}
                Either::Second(position) => {
                    if position < self.encoder_position {
                        self.publisher
                            .publish(InputEvent::TurnedCCW(position))
                            .await;
                    } else {
                        self.publisher.publish(InputEvent::TurnedCW(position)).await;
                    }
                    self.encoder_position = position;
                }
            }
        }
    }
}

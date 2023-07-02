use embassy_rp::gpio::{AnyPin, Input, Pull};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, pubsub::Publisher};
use embassy_time::{Duration, Timer};

#[derive(Clone)]
pub enum InputSource {
    Button,
    Key(usize),
}

#[derive(Clone)]
pub enum InputEvent {
    Pressed(InputSource),
    Released(InputSource),
    TurnedCW,
    TurnedCCW,
}

const NUM_KEYS: usize = 12;

pub struct InputHandler<'a, const SUBS: usize> {
    button_input: Input<'a, AnyPin>,
    button_active: bool,
    key_inputs: [Input<'a, AnyPin>; NUM_KEYS],
    key_active: [bool; NUM_KEYS],
    publisher: Publisher<'a, ThreadModeRawMutex, InputEvent, 4, SUBS, 1>,
}

impl<'a, const SUBS: usize> InputHandler<'a, SUBS> {
    pub fn new(
        button: AnyPin,
        keys: [AnyPin; NUM_KEYS],
        publisher: Publisher<'a, ThreadModeRawMutex, InputEvent, 4, SUBS, 1>,
    ) -> Self {
        let button_input = Input::new(button, Pull::Up);
        let button_active = false;

        let key_inputs = keys.map(|key| Input::new(key, Pull::Up));
        let key_active = [false; NUM_KEYS];

        InputHandler {
            button_input,
            button_active,
            key_inputs,
            key_active,
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

            Timer::after(interval).await;
        }
    }
}

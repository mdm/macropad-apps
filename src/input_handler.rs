use embassy_rp::gpio::{AnyPin, Input, Pull};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, pubsub::Publisher};
use embassy_time::{Duration, Timer};

#[derive(Clone)]
pub enum InputSource {
    Button,
    RotaryEncoder,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key10,
    Key11,
    Key12,
}

#[derive(Clone)]
pub enum InputEvent {
    Pressed(InputSource),
    Released(InputSource),
    TurnedCW,
    TurnedCCW,
}

pub struct InputHandler<'a, const SUBS: usize> {
    button_input: Input<'a, AnyPin>,
    button_active: bool,
    key_inputs: [Input<'a, AnyPin>; 12],
    key_active: [bool; 12],
    publisher: Publisher<'a, ThreadModeRawMutex, InputEvent, 4, SUBS, 1>,
}

impl<'a, const SUBS: usize> InputHandler<'a, SUBS> {
    pub fn new(
        button: AnyPin,
        keys: [AnyPin; 12],
        publisher: Publisher<'a, ThreadModeRawMutex, InputEvent, 4, SUBS, 1>,
    ) -> Self {
        let button_input = Input::new(button, Pull::Up);
        let button_active = false;

        let key_inputs = keys.map(|key| Input::new(key, Pull::Up));
        let key_active = [false; 12];

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
                    .publish(InputEvent::Pressed(InputSource::Button)).await;
            }
            if self.button_active && self.button_input.is_high() {
                self.button_active = false;
                self.publisher
                    .publish(InputEvent::Released(InputSource::Button)).await;
            }
            Timer::after(interval).await;
        }
    }
}

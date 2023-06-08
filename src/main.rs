#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
use core::convert::Infallible;

// use adafruit_macropad::{
//     entry,
//     hal::{
//         clocks::{init_clocks_and_plls, Clock},
//         gpio::FunctionSpi,
//         pac,
//         pio::PIOExt,
//         rosc::RingOscillator,
//         watchdog::Watchdog,
//         Sio, Spi, Timer,
//     },
//     Pins, XOSC_CRYSTAL_FREQ,
// };
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Level, Output, AnyPin},
};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    pubsub::{PubSubChannel, Subscriber, WaitResult},
};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    image::{Image, ImageRawLE},
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal::{digital::v2::InputPin, digital::v2::OutputPin, spi::MODE_0};
use fugit::RateExtU32;
use menu::Menu;
use panic_semihosting as _;
use rand::Rng;
use sh1106::{prelude::*, Builder};
use smart_leds::{
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite,
};
use ws2812_pio::Ws2812;

use input_handler::{InputEvent, InputHandler, InputSource};

mod input_handler;
mod menu;

const SUBS: usize = 4;
static INPUT_CHANNEL: PubSubChannel<ThreadModeRawMutex, InputEvent, 4, SUBS, 1> =
    PubSubChannel::new();

#[embassy_executor::task]
async fn blinker_task(mut led: Output<'static, AnyPin>, interval: Duration) {
    let mut input_subscriber = INPUT_CHANNEL.subscriber().unwrap();

    for _ in 0..3 {
        led.set_high();
        Timer::after(interval).await;
        led.set_low();
        Timer::after(interval).await;
    }

    loop {
        if let WaitResult::Message(InputEvent::Pressed(InputSource::Button)) =
            input_subscriber.next_message().await
        {
            for _ in 0..3 {
                led.set_high();
                Timer::after(interval).await;
                led.set_low();
                Timer::after(interval).await;
            }
        }
    }
}

#[embassy_executor::task]
async fn input_handler_task(mut input_handler: InputHandler<'static, SUBS>) {
    input_handler.run().await;
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_rp::init(Default::default());

    let button = peripherals.PIN_0.into();
    let keys = [
        peripherals.PIN_1.into(),
        peripherals.PIN_2.into(),
        peripherals.PIN_3.into(),
        peripherals.PIN_4.into(),
        peripherals.PIN_5.into(),
        peripherals.PIN_6.into(),
        peripherals.PIN_7.into(),
        peripherals.PIN_8.into(),
        peripherals.PIN_9.into(),
        peripherals.PIN_10.into(),
        peripherals.PIN_11.into(),
        peripherals.PIN_12.into(),
    ];
    let input_publisher = INPUT_CHANNEL.publisher().unwrap();
    let input_handler = InputHandler::new(button, keys, input_publisher);
    spawner.spawn(input_handler_task(input_handler)).unwrap();

    let led: AnyPin = peripherals.PIN_13.into(); 
    let led = Output::new(led, Level::Low);
    spawner
        .spawn(blinker_task(led, Duration::from_millis(300)))
        .unwrap();

    // let mut hues_and_values = [(0, 0); 12];
}

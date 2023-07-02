#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::cell::RefCell;

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::{
    clocks::RoscRng,
    gpio::{AnyPin, Level, Output},
    pac::{pio::StateMachine, resets::regs::Peripherals},
    peripherals::PIO0,
    pio::{Instance, Pio}, spi::{Spi, Blocking, self},
};
use embassy_sync::{
    blocking_mutex::{raw::{ThreadModeRawMutex, NoopRawMutex}, Mutex},
    pubsub::{PubSubChannel, Subscriber, WaitResult},
};
use embassy_time::{Duration, Timer, Delay};
use embedded_graphics::{
    image::{Image, ImageRawLE},
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal::{digital::v2::InputPin, digital::v2::OutputPin, spi::MODE_0, timer::CountDown};
use fugit::RateExtU32;
use menu::Menu;
use panic_halt as _;
use rand::Rng;
use rp2040_hal::rosc::{Enabled, RingOscillator};
use sh1106::{prelude::*, Builder};
use smart_leds::{
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB8,
};
use ws2812_pio_embassy::Ws2812;

use input_handler::{InputEvent, InputHandler, InputSource};

mod input_handler;
mod menu;

const SUBS: usize = 4;
static INPUT_CHANNEL: PubSubChannel<ThreadModeRawMutex, InputEvent, 4, SUBS, 1> =
    PubSubChannel::new();

const NEOPIXEL_NUM_LEDS: usize = 12;

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
async fn color_fader_task(mut ws2812: Ws2812<'static, PIO0, 0, NEOPIXEL_NUM_LEDS>) {
    let mut input_subscriber = INPUT_CHANNEL.subscriber().unwrap();

    let mut hues_and_values = [(0, 0); 12];

    loop {
        if let Some(WaitResult::Message(InputEvent::Pressed(InputSource::Key(key)))) =
            input_subscriber.try_next_message()
        {
            hues_and_values[key] = (RoscRng.gen::<u8>(), 255);
        }

        let data = hues_and_values.map(|(hue, val)| {
            let hsv = Hsv { hue, sat: 255, val };

            hsv2rgb(hsv)
        });

        ws2812.write(&data).await;

        hues_and_values = hues_and_values.map(|mut hv| {
            if hv.1 > 0 {
                hv.1 -= 5;
            }

            hv
        });

        Timer::after(Duration::from_millis(10)).await;
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

    let Pio {
        mut common, sm0, ..
    } = Pio::new(peripherals.PIO0);
    let ws2812 = Ws2812::new(&mut common, sm0, peripherals.DMA_CH0, peripherals.PIN_19);
    spawner.spawn(color_fader_task(ws2812)).unwrap();

    let led: AnyPin = peripherals.PIN_13.into();
    let led = Output::new(led, Level::Low);
    spawner
        .spawn(blinker_task(led, Duration::from_millis(300)))
        .unwrap();

    let sclk = peripherals.PIN_26;
    let mosi = peripherals.PIN_27;
    let miso = peripherals.PIN_28;
    let mut display_config = spi::Config::default();
    display_config.frequency = 10_000_000;
    let spi: Spi<'_, _, Blocking> = Spi::new_blocking(peripherals.SPI1, sclk, mosi, miso, display_config);

    let oled_cs = Output::new(peripherals.PIN_22, Level::Low);
    let mut oled_reset = Output::new(peripherals.PIN_23, Level::Low);
    let oled_dc = Output::new(peripherals.PIN_24, Level::Low);

    let mut display: GraphicsMode<_> = Builder::new().connect_spi(spi, oled_dc, oled_cs).into();

    display.reset(&mut oled_reset, &mut Delay).unwrap();
    display.init().unwrap();
    display.flush().unwrap();

    let mut menu = Menu::new(
        &[
            "Hello Rust!",
            "Hello world!",
            "Hello Marc!",
            "Test Item 4",
            "Test Item 5",
        ],
        display.size().height,
        &FONT_6X10,
        BinaryColor::Off,
        BinaryColor::On,
    );
    menu.draw(&mut display).unwrap();
    display.flush().unwrap();

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

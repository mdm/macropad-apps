#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(array_chunks)]

use embassy_executor::Spawner;
use embassy_rp::{
    clocks::{clk_sys_freq, RoscRng},
    gpio::{AnyPin, Level, Output},
    peripherals::{PIO0, PIO1},
    pio::Pio,
    pwm::{self, Pwm},
    spi::{self, Blocking, Spi},
};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    pubsub::{PubSubChannel, WaitResult},
};
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::prelude::*;
use fixed::FixedU16;
use menu::MenuManager;
use panic_halt as _;
use rand::Rng;
use sh1106::{prelude::*, Builder};
use smart_leds::hsv::{hsv2rgb, Hsv};
use ws2812_pio_embassy::Ws2812;

use input_handler::{InputEvent, InputHandler, InputSource};

mod chip8;
mod input_handler;
mod menu;
mod rotary_io;

const CAP: usize = 8;
const SUBS: usize = 8;
static INPUT_CHANNEL: PubSubChannel<ThreadModeRawMutex, InputEvent, CAP, SUBS, 1> =
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
async fn color_fader_task(mut ws2812: Ws2812<'static, PIO1, 0, NEOPIXEL_NUM_LEDS>) {
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
async fn input_handler_task(mut input_handler: InputHandler<'static, PIO0, 0, CAP, SUBS>) {
    input_handler.run().await;
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_rp::init(Default::default());

    let Pio {
        mut common, sm0, ..
    } = Pio::new(peripherals.PIO0);

    let rotary_io =
        rotary_io::RotaryIO::new(&mut common, sm0, peripherals.PIN_17, peripherals.PIN_18);

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
    let input_handler = InputHandler::new(button, keys, rotary_io, input_publisher);
    spawner.spawn(input_handler_task(input_handler)).unwrap();

    let Pio {
        mut common, sm0, ..
    } = Pio::new(peripherals.PIO1);
    let ws2812 = Ws2812::new(&mut common, sm0, peripherals.DMA_CH0, peripherals.PIN_19);
    spawner.spawn(color_fader_task(ws2812)).unwrap();

    let led: AnyPin = peripherals.PIN_13.into();
    let led = Output::new(led, Level::Low);
    spawner
        .spawn(blinker_task(led, Duration::from_millis(300)))
        .unwrap();

    let mut speaker_enable = Output::new(peripherals.PIN_14, Level::High);
    let mut pwm_config: pwm::Config = Default::default();
    pwm_config.divider = FixedU16::from_num(40);
    pwm_config.top = (clk_sys_freq() as f64 / f64::from(pwm_config.divider) / 440.0) as u16;
    pwm_config.compare_a = pwm_config.top / 2;
    let mut pwm = Pwm::new_output_a(peripherals.PWM_CH0, peripherals.PIN_16, pwm_config);
    Timer::after(Duration::from_secs(2)).await;
    speaker_enable.set_low();

    let sclk = peripherals.PIN_26;
    let mosi = peripherals.PIN_27;
    let miso = peripherals.PIN_28;
    let mut display_config = spi::Config::default();
    display_config.frequency = 10_000_000;
    let spi: Spi<'_, _, Blocking> =
        Spi::new_blocking(peripherals.SPI1, sclk, mosi, miso, display_config);

    let oled_cs = Output::new(peripherals.PIN_22, Level::Low);
    let mut oled_reset = Output::new(peripherals.PIN_23, Level::Low);
    let oled_dc = Output::new(peripherals.PIN_24, Level::Low);

    let mut display: GraphicsMode<_> = Builder::new().connect_spi(spi, oled_dc, oled_cs).into();

    display.reset(&mut oled_reset, &mut Delay).unwrap();
    display.init().unwrap();
    display.flush().unwrap();

    let choice = MenuManager::new(
        &[
            "Chip-8 Emulator",
            "Hello Rust!",
            "Hello world!",
            "Hello Marc!",
            "Test Item 4",
            "Test Item 5",
        ],
        display.size().height,
    )
    .choose(&mut display)
    .await;

    if let Some(choice) = choice {
        display.clear();
        display.flush().unwrap();
        match choice {
            0 => {
                let choice = MenuManager::new(&["Pong", "Blinky"], display.size().height)
                    .choose(&mut display)
                    .await;

                if let Some(choice) = choice {
                    display.clear();
                    display.flush().unwrap();

                    let mut chip8 = chip8::Chip8Harness::new();
                    match choice {
                        0 => {
                            chip8
                                .run(
                                    include_bytes!("../chip8-rs/games/PONG"),
                                    [1, 0, 12, 4, 0, 13, 0, 0, 0, 0, 0, 0],
                                    &mut display,
                                )
                                .await
                                .unwrap();
                        }
                        1 => {
                            chip8
                                .run(
                                    include_bytes!("../chip8-rs/games/BLINKY"),
                                    [0, 3, 0, 7, 6, 8, 0, 0, 0, 0, 0, 0],
                                    &mut display,
                                )
                                .await
                                .unwrap();
                        }
                        _ => unreachable!(),
                    };
                }
            }
            _ => {}
        }
    }

    display.clear();
    display.flush().unwrap();

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

#![no_std]
#![no_main]

use core::convert::Infallible;

use adafruit_macropad::{
    entry,
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::FunctionSpi,
        pac,
        pio::PIOExt,
        rosc::RingOscillator,
        watchdog::Watchdog,
        Sio, Spi, Timer,
    },
    Pins, XOSC_CRYSTAL_FREQ,
};
use embedded_graphics::{
    image::{Image, ImageRawLE},
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal::{digital::v2::InputPin, digital::v2::OutputPin, spi::MODE_0};
use fugit::RateExtU32;
use panic_halt as _;
use rand::Rng;
use sh1106::{prelude::*, Builder};
use smart_leds::{SmartLedsWrite, hsv::{Hsv, hsv2rgb}};
use ws2812_pio::Ws2812;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.led.into_push_pull_output();

    // Configure the addressable LED
    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);

    let mut ws = Ws2812::new(
        pins.neopixel.into_mode(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
        timer.count_down(),
    );

    let _spi_sclk = pins.sclk.into_mode::<FunctionSpi>();
    let _spi_mosi = pins.mosi.into_mode::<FunctionSpi>();
    // let _spi_miso = pins.miso.into_mode::<FunctionSpi>();
    // TODO: Are we using the best settings for data size and baudrate?
    let spi = Spi::<_, _, 8>::new(pac.SPI1).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        10.MHz(),
        &MODE_0,
    );
    let mut oled_dc = pins.oled_dc.into_push_pull_output();
    let mut oled_cs = pins.oled_cs.into_push_pull_output();
    let mut oled_reset = pins.oled_reset.into_push_pull_output();

    // Init OLED display
    oled_cs.set_high().unwrap();
    oled_dc.set_high().unwrap();
    oled_reset.set_high().unwrap(); // set RESET high
    oled_reset.set_low().unwrap(); // set RESET low
    delay.delay_us(1000); // delay 1000us
    oled_reset.set_high().unwrap(); // set RESET high
    delay.delay_us(1000); // delay 1000us

    let mut display: GraphicsMode<_> = Builder::new().connect_spi(spi, oled_dc, oled_cs).into();

    display.init().unwrap();
    display.flush().unwrap();

    // let text_style = MonoTextStyleBuilder::new()
    //     .font(&FONT_6X10)
    //     .text_color(BinaryColor::On)
    //     .build();

    // Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
    //     .draw(&mut display)
    //     .unwrap();

    // Text::with_baseline("Hello Rust!", Point::new(0, 16), text_style, Baseline::Top)
    //     .draw(&mut display)
    //     .unwrap();

    let im: ImageRawLE<BinaryColor> = ImageRawLE::new(include_bytes!("../rust.raw"), 64);

    Image::new(&im, Point::new(32, 0))
        .draw(&mut display)
        .unwrap();

    display.flush().unwrap();

    for _i in 0..3 {
        led_pin.set_high().unwrap();
        delay.delay_ms(100);
        led_pin.set_low().unwrap();
        delay.delay_ms(100);
    }

    let key1 = pins.key1.into_pull_up_input();
    let key2 = pins.key2.into_pull_up_input();
    let key3 = pins.key3.into_pull_up_input();
    let key4 = pins.key4.into_pull_up_input();
    let key5 = pins.key5.into_pull_up_input();
    let key6 = pins.key6.into_pull_up_input();
    let key7 = pins.key7.into_pull_up_input();
    let key8 = pins.key8.into_pull_up_input();
    let key9 = pins.key9.into_pull_up_input();
    let key10 = pins.key10.into_pull_up_input();
    let key11 = pins.key11.into_pull_up_input();
    let key12 = pins.key12.into_pull_up_input();

    let keys: &[&dyn InputPin<Error = Infallible>] = &[
        &key1, &key2, &key3, &key4, &key5, &key6, &key7, &key8, &key9, &key10, &key11, &key12,
    ];

    let mut hues_and_values = [(0, 0); 12];

    let mut rosc = RingOscillator::new(pac.ROSC).initialize();

    loop {
        delay.delay_ms(10);

        for (i, key) in keys.iter().enumerate() {
            if key.is_low().unwrap() {
                hues_and_values[i] = (rosc.gen::<u8>(), 255);
            } else if hues_and_values[i].1 > 0 {
                hues_and_values[i].1 -= 5;
            }
        }

        ws.write(hues_and_values.iter().copied().map(|color| {
            let hsv = Hsv {
                hue: color.0,
                sat: 255,
                val: color.1,
            };

            hsv2rgb(hsv)
        })).unwrap();
    }
}

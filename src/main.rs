#![no_std]
#![no_main]

use adafruit_macropad::{
    entry,
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::FunctionSpi,
        pac,
        pio::PIOExt,
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
use embedded_hal::{digital::v2::OutputPin, spi::MODE_0};
use fugit::RateExtU32;
use panic_halt as _;
use sh1106::{prelude::*, Builder};
use smart_leds::{brightness, SmartLedsWrite, RGB8};
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

    loop {
        let red: RGB8 = (255, 0, 0).into();
        let green: RGB8 = (0, 255, 0).into();
        let blue: RGB8 = (0, 0, 255).into();
        ws.write([red, green, blue].iter().copied()).unwrap();

        led_pin.set_high().unwrap();
        delay.delay_ms(1500);
        led_pin.set_low().unwrap();
        delay.delay_ms(1500);
    }
}

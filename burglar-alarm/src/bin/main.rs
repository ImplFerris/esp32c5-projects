#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::{error, info};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::main;
use esp_hal::time::{Duration, Instant};
use esp_println as _;

use esp_hal::rmt::Rmt;
use esp_hal::time::Rate;
use esp_hal_smartled::{RmtSmartLeds, buffer_size, color_order};
use smart_leds::{RGB8, SmartLedsWrite};

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    error!("{}", panic_info);
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    // generator version: 1.4.0
    // generator parameters: -o esp32c5 -o defmt -o esp32c5-wroom-1-psram -o vscode

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let sensor_pin = Input::new(
        peripherals.GPIO10,
        InputConfig::default().with_pull(Pull::Down),
    );

    let mut buzzer_pin = Output::new(peripherals.GPIO24, Level::Low, OutputConfig::default());

    let freq = Rate::from_mhz(80);
    let rmt = Rmt::new(peripherals.RMT, freq).unwrap();
    let mut led =
        RmtSmartLeds::<{ buffer_size::<RGB8>(1) }, _, RGB8, color_order::Rgb>::new_with_memsize(
            esp_hal_smartled::WS2812_TIMING,
            rmt.channel0,
            peripherals.GPIO27,
            2,
            freq,
        )
        .unwrap();

    info!("Monitoring...");
    loop {
        if sensor_pin.is_high() {
            info!("Motion detected");
            buzzer_pin.set_high();
            led.write([RGB8::new(255, 0, 0)]).unwrap();
        } else {
            buzzer_pin.set_low();
            led.write([RGB8::new(0, 0, 0)]).unwrap();
        }

        blocking_delay(Duration::from_millis(100));
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.2.2/examples
}

fn blocking_delay(duration: Duration) {
    let delay_start = Instant::now();
    while delay_start.elapsed() < duration {}
}

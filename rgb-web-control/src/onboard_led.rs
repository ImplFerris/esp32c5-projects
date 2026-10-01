use esp_hal::peripherals;
use esp_hal::rmt::Rmt;
use esp_hal::time::Rate;
use esp_hal_smartled::{buffer_size, color_order};
use serde::Deserialize;
use smart_leds::{RGB8, SmartLedsWrite};

const RMT_FREQ: Rate = Rate::from_mhz(80);
const BUFFER_SIZE: usize = buffer_size::<RGB8>(1);
type RmtSmartLeds =
    esp_hal_smartled::RmtSmartLeds<'static, BUFFER_SIZE, esp_hal::Blocking, RGB8, color_order::Rgb>;

#[derive(Deserialize)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

pub struct OnboardLed {
    led: RmtSmartLeds,
}

impl OnboardLed {
    pub fn new(
        rmt_peripheral: peripherals::RMT<'static>,
        led_peripheral: peripherals::GPIO27<'static>,
    ) -> Self {
        let rmt = Rmt::new(rmt_peripheral, RMT_FREQ).unwrap();
        let led = RmtSmartLeds::new_with_memsize(
            esp_hal_smartled::WS2812_TIMING,
            rmt.channel0,
            led_peripheral,
            2,
            RMT_FREQ,
        )
        .unwrap();

        Self { led }
    }

    pub fn set_color(&mut self, color: Color) {
        self.led
            .write([RGB8::new(color.red, color.green, color.blue)])
            .unwrap();
    }
}

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

pub static LED_SIGNAL: Signal<CriticalSectionRawMutex, Color> = Signal::new();

#[embassy_executor::task]
pub async fn led_task(mut led: OnboardLed) -> ! {
    loop {
        let color = LED_SIGNAL.wait().await;
        led.set_color(color);
    }
}

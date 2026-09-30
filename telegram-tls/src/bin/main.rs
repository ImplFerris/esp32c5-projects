#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;

use telegram_notification::telegram::TelegramClient;
use telegram_notification::wifi;

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    error!("{}", panic_info);
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.4.0
    // generator parameters: -o esp32c5 -o esp32c5-wroom-1-psram -o unstable-hal -o alloc -o embassy -o wifi -o defmt -o vscode

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    info!("Embassy initialized!");

    let mut sensor_pin = Input::new(
        peripherals.GPIO10,
        InputConfig::default().with_pull(Pull::Down),
    );

    let wifi_stack = wifi::init_wifi(spawner, peripherals.WIFI).await;
    info!("Wi-Fi Initialized");

    let mut telegram = TelegramClient::new(wifi_stack);

    Timer::after(Duration::from_secs(60)).await; // PIR warm-up
    info!("Monitoring...");

    loop {
        sensor_pin.wait_for_high().await;
        info!("Motion detected!");

        telegram.notify("Motion Detected").await;

        sensor_pin.wait_for_low().await;
    }
}

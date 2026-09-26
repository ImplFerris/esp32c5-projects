#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use access_website::mk_static;
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
};
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;

use reqwless::{client::HttpClient, request::Method};

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
    // generator parameters: -o esp32c5 -o esp32c5-wroom-1-psram -o unstable-hal -o alloc -o embassy -o wifi -o defmt

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    info!("Embassy initialized!");

    let wifi_stack = access_website::wifi::init_wifi(spawner, peripherals.WIFI).await;
    info!("Wi-Fi Initialized");

    Timer::after(Duration::from_millis(1000)).await;

    send_http_request(wifi_stack).await;

    loop {
        Timer::after(Duration::from_millis(3000)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.2.2/examples
}

async fn send_http_request(wifi_stack: embassy_net::Stack<'static>) {
    let tcp_client = TcpClient::new(
        wifi_stack,
        mk_static!(
            TcpClientState
            <1, 1500, 1500>,
            TcpClientState::<1, 1500, 1500>::new()
        ),
    );
    let dns_client = DnsSocket::new(wifi_stack);

    let mut client = HttpClient::new(&tcp_client, &dns_client);
    let mut rx_buf = [0u8; 4096];

    let mut builder = client
        .request(Method::GET, "http://httpbin.org/get?hello=Hello+esp-hal")
        .await
        .inspect_err(|e| error!("Request Build Error: {:?}", e))
        .unwrap();

    // let mut builder = builder.headers(&[("Host", "httpbin.org"), ("Connection", "close")]);

    info!("Sending HTTP Request");
    let response = builder.send(&mut rx_buf).await.unwrap();

    match response.body().read_to_end().await {
        Ok(data) => {
            if let Ok(st) = core::str::from_utf8(data) {
                info!("Body: {}", st);
            }
        }
        Err(e) => info!("Body error: {:?}", e),
    }
}

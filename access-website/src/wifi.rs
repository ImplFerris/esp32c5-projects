use defmt::info;
use embassy_executor::Spawner;
use embassy_net::{Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::peripherals::WIFI;
use esp_hal::rng::Rng;
use esp_radio::wifi::scan::ScanConfig;
use esp_radio::wifi::sta::StationConfig;
use esp_radio::wifi::{self, AuthenticationMethodConfig, Interface, WifiController};

use crate::mk_static;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

pub async fn init_wifi(spawner: Spawner, wifi_peripheral: WIFI<'static>) -> Stack<'static> {
    let station_config = wifi::Config::Station(
        StationConfig::default()
            .with_ssid(SSID.try_into().unwrap())
            .with_authentication(AuthenticationMethodConfig::Wpa2Personal(
                PASSWORD.try_into().unwrap(),
            )),
    );

    let mut controller = esp_radio::wifi::WifiController::new(
        wifi_peripheral,
        wifi::ControllerConfig::default().with_initial_config(station_config),
    )
    .expect("Failed to initialize Wi-Fi controller");
    let wifi_interface = esp_radio::wifi::Interface::station();

    let config = embassy_net::Config::dhcpv4(Default::default());

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Init network stack
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    info!("Scan");
    let scan_config = ScanConfig::default().with_max(10);
    let result = controller.scan_async(&scan_config).await.unwrap();
    for ap in result {
        info!("{:?}", ap);
    }

    spawner.spawn(connection(controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());

    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        info!("Got IP: {}", config.address);
    }

    set_dns_servers(stack);

    stack
}

fn set_dns_servers(stack: Stack<'static>) {
    // Custom DNS Servers
    if let Some(mut cfg) = stack.config_v4() {
        cfg.dns_servers = heapless::Vec::from_slice(&[
            embassy_net::Ipv4Address::new(1, 1, 1, 1),
            embassy_net::Ipv4Address::new(8, 8, 8, 8),
        ])
        .expect("DNS server list exceeds heapless::Vec capacity");

        stack.set_config_v4(embassy_net::ConfigV4::Static(cfg));
        info!("Overrode DNS servers, keeping DHCP address/gateway");
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    info!("start connection task");

    loop {
        info!("About to connect...");

        match controller.connect_async().await {
            Ok(info) => {
                info!("Wifi connected to {:?}", info);

                // wait until we're no longer connected
                let info = controller.wait_for_disconnect_async().await.ok();
                info!("Disconnected: {:?}", info);
            }
            Err(e) => {
                info!("Failed to connect to wifi: {:?}", e);
            }
        }

        Timer::after(Duration::from_millis(5000)).await
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await
}

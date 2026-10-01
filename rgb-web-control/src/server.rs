use picoserve::{Router, extract, response::File, routing};

use crate::onboard_led;

pub const WEB_TASK_POOL_SIZE: usize = 2;
static CONFIG: picoserve::Config = picoserve::Config::const_default().keep_connection_alive();

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
pub async fn web_task(task_id: usize, stack: embassy_net::Stack<'static>) -> ! {
    let port = 80;
    let mut tcp_rx = [0; 1024];
    let mut tcp_tx = [0; 1024];
    let mut http_buffer = [0; 2048];

    let app = Router::new()
        .route(
            "/",
            routing::get_service(File::html(include_str!("../assets/index.html"))),
        )
        .route("/api/onboard-led", routing::post(handle_color));

    picoserve::Server::new(&app, &CONFIG, &mut http_buffer)
        .listen_and_serve(task_id, stack, port, &mut tcp_rx, &mut tcp_tx)
        .await
        .into_never()
}

async fn handle_color(extract::Json(color): extract::Json<onboard_led::Color>) {
    onboard_led::LED_SIGNAL.signal(color);
}

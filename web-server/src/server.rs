use picoserve::{
    Router,
    response::File,
    routing::{self},
};

pub const WEB_TASK_POOL_SIZE: usize = 2;
static CONFIG: picoserve::Config = picoserve::Config::const_default().keep_connection_alive();

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
pub async fn web_task(task_id: usize, stack: embassy_net::Stack<'static>) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    let app = Router::new()
        .route(
            "/",
            routing::get_service(File::html(include_str!("../assets/index.html"))),
        )
        .route(
            "/logo.svg",
            routing::get_service(File::with_content_type(
                "image/svg+xml",
                include_bytes!("../assets/logo.svg"),
            )),
        );

    picoserve::Server::new(&app, &CONFIG, &mut http_buffer)
        .listen_and_serve(task_id, stack, port, &mut tcp_rx_buffer, &mut tcp_tx_buffer)
        .await
        .into_never()
}

use defmt::{error, info};

use embassy_net::Stack;
use embassy_time::{Duration, Instant, with_timeout};

use embassy_net::{dns::DnsSocket, tcp::client::TcpClient};
use reqwless::client::{HttpClient, TlsConfig, TlsVerify};
use reqwless::request::Method;

use alloc::format;
use reqwless::headers::ContentType;
use reqwless::request::RequestBuilder;

use crate::mk_static;

extern crate alloc;

// TCP Config
const MAX_CONCURRENT_CONNECTIONS: usize = 1;
const TCP_TX_BUFFER_SIZE: usize = 1500;
const TCP_RX_BUFFER_SIZE: usize = 1500;

type TcpClientState = embassy_net::tcp::client::TcpClientState<
    MAX_CONCURRENT_CONNECTIONS,
    TCP_TX_BUFFER_SIZE,
    TCP_RX_BUFFER_SIZE,
>;

// Telegram Config
const TELEGRAM_TOKEN: &str = env!("TELEGRAM_TOKEN");
const CHAT_ID: &str = env!("TELEGRAM_CHAT_ID");
const NOTIFICATION_INTERVAL: Duration = Duration::from_secs(120);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
enum TelegramError {
    Http(reqwless::Error),
    Status(u16),
}

impl From<reqwless::Error> for TelegramError {
    fn from(e: reqwless::Error) -> Self {
        TelegramError::Http(e)
    }
}

pub struct TelegramClient {
    stack: Stack<'static>,
    tcp_state: &'static TcpClientState,
    last_sent: Option<Instant>,

    // Buffers
    tls_rx: [u8; 16_640], // 16640 required for the TLS read buffer
    tls_tx: [u8; 4096],
    http_rx: [u8; 4096],
}

impl TelegramClient {
    pub fn new(stack: Stack<'static>) -> Self {
        Self {
            stack,
            tcp_state: mk_static!(TcpClientState, TcpClientState::new()),
            last_sent: None,

            // Buffers
            tls_rx: [0; 16_640],
            tls_tx: [0; 4096],
            http_rx: [0; 4096],
        }
    }

    pub async fn notify(&mut self, text: &str) {
        if let Some(t) = self.last_sent {
            if t.elapsed() < NOTIFICATION_INTERVAL {
                info!("Cooldown active, skipping");
                return;
            }
        }

        match with_timeout(REQUEST_TIMEOUT, self.send(text)).await {
            Ok(result) => self.handle_result(result),
            Err(_) => {
                error!("Telegram request timed out");
            }
        }
    }

    fn handle_result(&mut self, result: Result<(), TelegramError>) {
        match result {
            Ok(()) => {
                info!("Message sent successfully");
                self.last_sent = Some(Instant::now());
            }
            Err(TelegramError::Http(e)) => {
                error!("HTTP request failed: {:?}", e);
            }
            Err(TelegramError::Status(s)) => {
                error!("Telegram returned HTTP status: {}", s);
            }
        }
    }

    async fn send(&mut self, text: &str) -> Result<(), TelegramError> {
        let rng = esp_hal::rng::Rng::new();
        let tls_seed = {
            let mut bytes = [0u8; 8];
            rng.read(&mut bytes);
            u64::from_le_bytes(bytes)
        };

        let tls_config = TlsConfig::new(
            tls_seed,
            &mut self.tls_rx,
            &mut self.tls_tx,
            TlsVerify::None, // Insecure but ok for now
        );

        let tcp_client = TcpClient::new(self.stack, self.tcp_state);
        let dns_client = DnsSocket::new(self.stack);
        let mut client = HttpClient::new_with_tls(&tcp_client, &dns_client, tls_config);

        let url = format!("https://api.telegram.org/bot{}/sendMessage", TELEGRAM_TOKEN);
        let body = format!(r#"{{"chat_id":"{}","text":"{}"}}"#, CHAT_ID, text);

        let mut request = client
            .request(Method::POST, &url)
            .await?
            .body(body.as_bytes())
            .content_type(ContentType::ApplicationJson);

        let response = request.send(&mut self.http_rx).await?;

        let status = response.status.0;
        let resp_body = response.body().read_to_end().await?;
        let resp_text = core::str::from_utf8(resp_body).unwrap_or("invalid UTF-8");

        if status != 200 {
            error!("Telegram error body: {}", resp_text);
            return Err(TelegramError::Status(status));
        }

        info!("Telegram response: {}", resp_text);

        Ok(())
    }
}

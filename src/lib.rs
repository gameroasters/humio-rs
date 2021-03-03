#![forbid(unsafe_code)]
#![deny(clippy::pedantic)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::perf)]
#![deny(clippy::nursery)]
#![deny(clippy::match_like_matches_macro)]
#![allow(clippy::module_name_repetitions)]

mod error;

use error::Result;
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};

const HUMIO_EVENT_CHUNK_SIZE: usize = 10;
const HUMIO_UPDATE_LOOP_DELAY: Duration = Duration::from_secs(1);
const HUMIO_SEND_FAILED_RETRY_DELAY: Duration = Duration::from_millis(500);

#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct TrackingEvent {
    pub timestamp: String,
    pub attributes: HashMap<String, String>,
}

#[derive(Serialize, Clone, Debug)]
struct TrackingRequest<'a> {
    pub tags: &'a HashMap<String, String>,
    pub events: &'a [&'a TrackingEvent],
}

#[derive(Default, Deserialize, Serialize, Clone, Debug)]
struct TrackingRequestUnstructured {
    pub fields: HashMap<String, String>,
    pub messages: Vec<String>,
}

///
#[derive(Clone)]
pub struct HumioLogger {
    outbox: Arc<Mutex<Vec<TrackingEvent>>>,
    tags: HashMap<String, String>,
    key: String,
    url: String,
}

const HUMIO_ENDPOINT: &str = "https://cloud.humio.com:443/api/v1/ingest/humio-structured";

impl HumioLogger {
    #[must_use]
    pub fn new(key: String, tags: HashMap<String, String>) -> Self {
        let this = Self {
            outbox: Arc::new(Mutex::new(Vec::new())),
            tags,
            key,
            url: String::from(HUMIO_ENDPOINT),
        };

        let this_clone = this.clone();
        tokio::spawn(async move {
            this_clone.update_loop().await;
        });

        this
    }

    ///
    pub async fn push(&self, ev: TrackingEvent) {
        let mut outbox = self.outbox.lock().await;
        outbox.push(ev);
    }

    async fn update_loop(self) {
        loop {
            let logger = self.clone();
            let task_res = tokio::spawn(async move {
                logger.update().await;
            })
            .await;

            if let Err(e) = task_res {
                eprintln!("humio send error: {}", e);
            }

            sleep(HUMIO_UPDATE_LOOP_DELAY).await;
        }
    }

    async fn update(&self) {
        let outbox = {
            let mut outbox = self.outbox.lock().await;
            //Takes ownership of outbox, replaces it with new one
            std::mem::take(&mut *outbox)
        };

        for ev_chunk in outbox
            .iter()
            .collect::<Vec<&TrackingEvent>>()
            .chunks(HUMIO_EVENT_CHUNK_SIZE)
        {
            while self.send(ev_chunk).await.is_err() {
                sleep(HUMIO_SEND_FAILED_RETRY_DELAY).await
            }
        }
    }

    async fn send(&self, events: &[&TrackingEvent]) -> Result<()> {
        let auth_header = format!("Bearer {}", &self.key);

        let body = TrackingRequest {
            tags: &self.tags,
            events,
        };
        let body = serde_json::to_string(&[body])?;

        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, Body>(https);
        let req = Request::builder()
            .method("POST")
            .header("Authorization", auth_header)
            .uri(&self.url)
            .body(Body::from(body))?;
        client.request(req).await?;

        Ok(())
    }
}

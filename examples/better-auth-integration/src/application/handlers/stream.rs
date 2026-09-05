use async_stream::stream;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive},
    response::Sse,
};
use orpc::orpc;
use std::{convert::Infallible, time::Duration};
use tokio_stream::{iter, Stream, StreamExt};

use crate::{domain::models::planet::StreamEvent, infrastructure::context::AppState};

#[orpc(method = "GET", path = "/stream")]
pub async fn stream_events(
    State(_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Initial empty comment flushes headers so the client connects immediately
    let initial = tokio_stream::iter([Ok(Event::default().comment(""))]);

    let events = iter(0u32..)
        .throttle(Duration::from_secs(1))
        .take(10)
        .map(|count| {
            let payload = serde_json::to_string(&StreamEvent {
                message: format!("Event #{count}"),
                count,
            })
            .unwrap();
            Ok(Event::default()
                .event("message")
                .id(count.to_string())
                .retry(Duration::from_secs(5))
                .data(payload))
        })
        .chain(tokio_stream::iter([Ok(Event::default()
            .event("close")
            .data(""))]));

    Sse::new(initial.chain(events))
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text(""))
}

#[orpc(method = "GET", path = "/stream-async")]
pub async fn stream_events_async(
    State(_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let s = stream! {
        yield Ok(Event::default().comment(""));
        for i in 0u32..15 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let payload = serde_json::to_string(&StreamEvent {
                message: format!("Async Stream Event #{i}"),
                count: i,
            })
            .unwrap();
            yield Ok(Event::default()
                .event("message")
                .id(i.to_string())
                .retry(Duration::from_secs(5))
                .data(payload));
        }
        yield Ok(Event::default().event("close").data(""));
    };

    Sse::new(s).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text(""))
}

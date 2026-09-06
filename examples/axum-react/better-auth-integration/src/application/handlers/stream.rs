use async_stream::stream;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive},
    response::Sse,
};
use rorpc::orpc;
use std::{convert::Infallible, time::Duration};
use tokio_stream::{iter, Stream, StreamExt};

use crate::{domain::models::planet::EventData, infrastructure::context::AppState};

// ---------------------------------------------------------------------------
// SSE helpers
// ---------------------------------------------------------------------------

fn sse_flush() -> Result<Event, Infallible> {
    Ok(Event::default().comment(""))
}

fn sse_close() -> Result<Event, Infallible> {
    Ok(Event::default().event("close").data(""))
}

fn sse_message<T: serde::Serialize>(id: impl ToString, payload: &T) -> Result<Event, Infallible> {
    let data = serde_json::to_string(payload).unwrap_or_default();
    Ok(Event::default()
        .event("message")
        .id(id.to_string())
        .retry(Duration::from_secs(5))
        .data(data))
}

/// Wraps an inner stream with a flush header and close trailer.
/// Caller builds the `Sse` response to allow customisation of keep-alive etc.
fn sse_stream<S>(inner: S) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static
where
    S: Stream<Item = Result<Event, Infallible>> + Send + 'static,
{
    iter([sse_flush()]).chain(inner).chain(iter([sse_close()]))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[orpc(method = "GET", path = "/stream", data = StreamEvent)]
pub async fn stream_events(
    State(_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(sse_stream(
        iter(0u32..)
            .throttle(Duration::from_secs(1))
            .take(10)
            .map(|count| {
                sse_message(
                    count,
                    &EventData {
                        message: format!("Event #{count}"),
                        count,
                    },
                )
            }),
    ))
    .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text(""))
}

#[orpc(method = "GET", path = "/stream-async", data = EventData)]
pub async fn stream_events_async(
    State(_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(sse_stream(stream! {
        for i in 0u32..15 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            yield sse_message(i, &EventData {
                message: format!("Async Stream Event #{i}"),
                count: i,
            });
        }
    }))
    .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text(""))
}

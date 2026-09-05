use crate::domain::models::planet::StreamEvent;
use crate::infrastructure::auth::middleware::BaseContext;
use async_stream::stream;
use orpc_core::OrpcError;
use std::time::Duration;
use tokio_stream::{iter, Stream, StreamExt};

pub async fn stream_events(
    _ctx: BaseContext,
    _: (),
) -> Result<impl Stream<Item = StreamEvent>, OrpcError> {
    let s = iter(0u32..)
        .throttle(Duration::from_secs(1))
        .take(10)
        .map(|count| StreamEvent {
            message: format!("Event #{count}"),
            count,
        });
    Ok(s)
}

pub async fn stream_events_async(
    _ctx: BaseContext,
    _: (),
) -> Result<impl Stream<Item = StreamEvent>, OrpcError> {
    let s = stream! {
        for i in 0u32..15 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            yield StreamEvent { message: format!("Async Stream Event #{i}"), count: i };
        }
    };
    Ok(s)
}

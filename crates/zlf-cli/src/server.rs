use std::convert::Infallible;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::State,
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    routing::post,
    Json, Router,
};
use tower_http::cors::{Any, CorsLayer};
use zlf_config::ZlfConfig;

use crate::handler::handle_request;
use crate::protocol::Request;
use crate::state::{ensure_db, AppState};

pub(crate) async fn serve_http(port: u16) -> Result<()> {
    let state = Arc::new(AppState::empty());

    let app = Router::new()
        .route("/api", post(handle_http_request))
        .route("/api/sse", post(handle_sse_request))
        .route("/health", axum::routing::get(|| async { "ok" }))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("zlf server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_http_request(
    State(state): State<Arc<AppState>>,
    Json(request): Json<Request>,
) -> impl IntoResponse {
    let response = handle_request(request, &state).await;
    Json(response)
}

async fn handle_sse_request(
    State(state): State<Arc<AppState>>,
    Json(request): Json<Request>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let events = build_sse_events(request, &state).await;
    Sse::new(tokio_stream::iter(events))
}

#[allow(clippy::too_many_lines)]
async fn build_sse_events(request: Request, state: &AppState) -> Vec<Result<Event, Infallible>> {
    match request {
        Request::Query { path, query } => {
            let config = ZlfConfig::load();
            let path = path.unwrap_or(config.db_path);
            match ensure_db(state, &path).await {
                Ok(db) => match db.query_prolog(&query) {
                    Ok(results) => {
                        let mut events = Vec::with_capacity(results.len() + 2);
                        events.push(sse_json_event(
                            "started",
                            serde_json::json!({"type":"started","query":query}),
                        ));
                        let count = results.len();
                        for item in results {
                            events.push(sse_json_event(
                                "chunk",
                                serde_json::json!({"type":"chunk","data":item}),
                            ));
                        }
                        events.push(sse_json_event(
                            "done",
                            serde_json::json!({"type":"done","count":count}),
                        ));
                        events
                    }
                    Err(e) => vec![sse_json_event(
                        "error",
                        serde_json::json!({"type":"error","code":"QUERY_FAILED","message":e.to_string()}),
                    )],
                },
                Err(e) => vec![sse_json_event(
                    "error",
                    serde_json::json!({"type":"error","code":"DB_OPEN_FAILED","message":e}),
                )],
            }
        }
        other => {
            let response = handle_request(other, state).await;
            let data = serde_json::to_value(response).unwrap_or_else(|e| {
                serde_json::json!({"type":"error","code":"SERIALIZE_FAILED","message":e.to_string()})
            });
            vec![sse_json_event("response", data)]
        }
    }
}

fn sse_json_event(event: &str, data: serde_json::Value) -> Result<Event, Infallible> {
    let data = serde_json::to_string(&data).unwrap_or_else(|e| {
        serde_json::json!({"type":"error","code":"SERIALIZE_FAILED","message":e.to_string()})
            .to_string()
    });
    Ok(Event::default().event(event).data(data))
}

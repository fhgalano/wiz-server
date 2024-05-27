use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time;

use axum::{
    debug_handler,
    routing::{get, post},
    Router,
    extract::{State, Path},
};
use surrealdb::sql::Id;
use tokio::sync::RwLock;
use tokio::task::block_in_place;
use tokio::runtime::Handle;
use url::Url;

use wiz_bulb::bulb::Bulb;
use wiz_bulb::registry::Registry;
use wiz_bulb::registry::connect_to_db;


#[tokio::main]
async fn main() {
    let registry = Arc::new(RwLock::new(
        Registry::new_from_url(Url::parse("ws://localhost:8000").unwrap()).await
    ));

    let c = registry.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_secs(5));
        loop {
            dbg!("DeezNuts");
            interval.tick().await;
        }
    });

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/bulb", post(add_bulb))
        .route("/bulb/on/:id", get(turn_on_bulb))
        .route("/bulb/off/:id", get(turn_off_bulb))
        .route("/bulb/:name", get(get_bulb_by_name))
        .with_state(Arc::clone(&registry));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


async fn turn_on_bulb(State(state): State<Arc<RwLock<Registry>>>, Path(id): Path<String>) -> String {
    let mut registry = state.write().await;

    let id = match id.parse::<i32>() {
        Ok(i) => Id::from(i),
        _ => Id::from(id)
    };

    match registry.turn_on_by_id(id.clone()) {
        Ok(status) => format!("Turn On Function Status for id: {} - {}", id, status),
        Err(e) => e.to_string()
    }
}


#[debug_handler]
async fn get_bulb_by_name(State(state): State<Arc<RwLock<Registry>>>, Path(name): Path<String>) -> String {
    let registry = state.read().await;
    serde_json::to_string(&registry.find_bulb_by_name(name).unwrap()).unwrap()
}


#[debug_handler]
async fn turn_off_bulb(State(state): State<Arc<RwLock<Registry>>>, Path(id): Path<String>) -> String {
    let mut registry = state.write().await;
    let id = match id.parse::<i32>() {
        Ok(i) => Id::from(i),
        _ => Id::from(id)
    };
    match registry.turn_off_by_id(id.clone()) {
        Ok(status) => format!("Turn Off Function Status for id: {} - {}", id, status),
        Err(e) => e.to_string()
    }
}


#[debug_handler]
async fn add_bulb(State(state): State<Arc<RwLock<Registry>>>) -> String {
    let mut registry = state.write().await;

    let bulb = Bulb::new(
        IpAddr::V4(Ipv4Addr::new(192, 168, 68, 58)),
        format!("test_bulb_{}", 2),
        2,
    );

    block_in_place(move || {
        let _ = Handle::current().block_on(
            registry.add(Box::new(bulb))
        );
    });
    "added bulb".to_string()
}

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time;

use axum::{
    debug_handler,
    routing::{get, post},
    Router,
    extract::{State, Path, Json},
};
use logging_timer::stime;
use surrealdb::sql::Id;
use tokio::sync::RwLock;
use tokio::task::block_in_place;
use tokio::runtime::Handle;
use url::Url;

use wiz_bulb::bulb::Bulb;
use wiz_bulb::bulb::{On, Off};
use wiz_bulb::registry::Registry;
use wiz_bulb::registry::connect_to_db;

use env_logger::Builder;
use log::{info};

#[tokio::main]
async fn main() {
    configure_logging();

    let registry = Arc::new(RwLock::new(
        Registry::new_from_url(Url::parse("ws://localhost:8000").unwrap()).await
    ));

    let c = registry.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_secs(45));
        loop {
            info!("NewCycle");
            interval.tick().await;
        }
    });

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/bulb", post(add_bulb))
        .route("/bulb/test", post(toggle_bulb))
        .route("/bulb/on/:id", get(turn_on_bulb))
        .route("/bulb/off/:id", get(turn_off_bulb))
        .route("/bulb/:name", get(get_bulb_by_name))
        .route("/bulb/discover", get(discover_unknown_bulbs))
        .with_state(Arc::clone(&registry));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn configure_logging() {
    let mut builder = Builder::new();

    builder
        .filter(None, log::LevelFilter::Info)
        .target(env_logger::Target::Stdout)
        .init();
}


#[stime("info")]
#[debug_handler]
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


#[stime]
#[debug_handler]
async fn get_bulb_by_name(State(state): State<Arc<RwLock<Registry>>>, Path(name): Path<String>) -> String {
    let registry = state.read().await;
    serde_json::to_string(&registry.find_bulb_by_name(name).unwrap()).unwrap()
}


#[stime("info")]
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

#[stime]
#[debug_handler]
async fn add_bulb(State(state): State<Arc<RwLock<Registry>>>, Json(bulb): Json<Bulb>) -> String {
    let mut registry = state.write().await;

    block_in_place(move || {
        Handle::current().block_on(async move {
                registry.add(Box::new(bulb)).await.unwrap();
        });
    });
    "added bulb".to_string()
}

#[stime]
#[debug_handler]
async fn toggle_bulb(Json(mut bulb): Json<Bulb>) -> String {
    match bulb.get_state() {
        Ok(true) => {bulb.off();},
        Ok(false) => {bulb.on();},
        Err(e) => println!("failed to toggle the bulb, you'll have to guess: {e:?}")
    };

    "flipped bulb".to_string()
}

#[stime]
#[debug_handler]
async fn discover_unknown_bulbs(State(state): State<Arc<RwLock<Registry>>>) -> String {
    let registry = state.read().await;

    serde_json::to_string(&registry.discover_unknown_bulbs()).unwrap()
}


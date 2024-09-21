mod db;

use warp::http::Response;
use warp::Filter;
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

use db::MongoDB;
use copydocx::{CustomError, handle_rejection};

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
type Rooms = Arc<RwLock<HashMap<String, HashMap<usize, mpsc::UnboundedSender<Message>>>>>; 


#[tokio::main]
async fn main() {
	pretty_env_logger::init();
	dotenv::dotenv().ok();

	let mongodb = MongoDB::init().await;
	if let Err(err) = mongodb.create_ttl_index().await {
        eprintln!("Failed to create TTL index: {:?}", err);
        return;
    }
	let mongodb_filter = warp::any().map(move || mongodb.clone());

	let rooms = Rooms::default();
    let rooms_filter = warp::any().map(move || rooms.clone());

	let chat = warp::path("chat")
        .and(warp::ws())
		.and(warp::path::param::<String>())
        .and(rooms_filter)
        .map(|ws: warp::ws::Ws, path: String, rooms| {
            ws.on_upgrade(move |socket| user_connected(socket, path, rooms))
        });

	let save_text = warp::path("save_text")
        .and(warp::post())
        .and(warp::body::json())
		.and(mongodb_filter.clone())
        .and_then(|body: HashMap<String, String>, mongodb: MongoDB| async move  {
            handle_save_text(body, mongodb).await
        });

	let load = warp::path("load")
	.and(warp::get())
	.and(warp::path::param::<String>()) // Expecting a path parameter
	.and(mongodb_filter.clone())
	.and_then(|path: String, mongodb: MongoDB| async move {
		handle_load(path, mongodb).await
	});

	let static_files = warp::fs::dir("static");
	let fallback = warp::any()
        .and_then(|| async {
            // Read the contents of `page.html`
            let path = PathBuf::from("static/page.html");
            let contents = fs::read_to_string(&path).unwrap_or_else(|_| "<h1>404 Not Found</h1>".to_string());
            Ok::<_, warp::Rejection>(Response::builder()
                .header("Content-Type", "text/html")
                .body(contents)
                .unwrap())
        });

	let routes = static_files.or(fallback);
	let routes = chat.or(load).or(save_text).or(routes).recover(handle_rejection);
	
	println!("Server running on port 8000");
    // Start the warp server
    warp::serve(routes)
        .run(([127, 0, 0, 1], 8000))
        .await;
}

async fn user_connected(ws: WebSocket, path: String, rooms: Rooms) {
    // Generate a unique ID for this user
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    eprintln!("new chat user (ID: {}, Path: {})", my_id, path);

    // Split the socket into a sender and receiver
    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    // Unbounded channel for buffering messages
    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            user_ws_tx.send(message).unwrap_or_else(|e| {
                eprintln!("websocket send error: {}", e);
            }).await;
        }
    });

    // Add user to the room
    {
        let mut rooms = rooms.write().await;
        let room = rooms.entry(path.clone()).or_default();
        room.insert(my_id, tx);
    }

    // Handle incoming messages
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error (ID: {}): {}", my_id, e);
                break;
            }
        };
        user_message(my_id, msg, &path, &rooms).await;
    }

    // Handle disconnection
    user_disconnected(my_id, &path, &rooms).await;
}

async fn user_message(my_id: usize, msg: Message, path: &str, rooms: &Rooms) {
    let msg = if let Ok(s) = msg.to_str() {
        s
    } else {
        return;
    };

    let new_msg = format!("{}", msg);

    let rooms = rooms.read().await;
    if let Some(room) = rooms.get(path) {
        for (&uid, tx) in room.iter() {
            if my_id != uid {
                if let Err(_disconnected) = tx.send(Message::text(new_msg.clone())) {
                    // Handle disconnected sender
                }
            }
        }
    }
}

async fn user_disconnected(my_id: usize, path: &str, rooms: &Rooms) {
    eprintln!("good bye user (ID: {}, Path: {})", my_id, path);

    let mut rooms = rooms.write().await;
    if let Some(room) = rooms.get_mut(path) {
        room.remove(&my_id);
        if room.is_empty() {
            rooms.remove(path);
        }
    }
}

async fn handle_save_text(
    body: HashMap<String, String>, 
    mongodb: MongoDB
) -> Result<warp::reply::Json, warp::Rejection> {
    let text = body.get("text").unwrap_or(&"".to_string()).clone();
    let path = body.get("path").unwrap_or(&"".to_string()).clone();

    match mongodb.save_data(&path, Some(&text), None).await {
        Ok(_) => {
            println!("Saved text for path '{}': {}", path, text);
            Ok(warp::reply::json(&format!("Saved text for path '{}'", path)))
        }
        Err(err) => {
            eprintln!("Failed to save data: {:?}", err);
            Err(warp::reject::custom(CustomError::from_mongo_error(err)))
        }
    }
}

async fn handle_load(path: String, mongodb: MongoDB) -> Result<warp::reply::Json, warp::Rejection> {
    match mongodb.retrieve_data(&path).await {
        Ok(Some(data)) => {
            println!("Loaded text for path '{}': {}", path, data.text);
            Ok(warp::reply::json(&data.text))
        }
        Ok(_) => {
            println!("No text found for path '{}'", path);
            Ok(warp::reply::json(&"".to_string())) // Return empty if not found
        }
        Err(err) => {
            eprintln!("Failed to retrieve data: {:?}", err);
            // Optionally, handle the error more gracefully here
            Err(warp::reject::custom(CustomError::from_mongo_error(err)))
        }
    }
}

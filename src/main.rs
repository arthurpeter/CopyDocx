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
use serde_json::Value;
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use serde::Serialize;
use db::MongoDB;
use copydocx::{CustomError, handle_rejection};

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
type Rooms = Arc<RwLock<HashMap<String, HashMap<usize, mpsc::UnboundedSender<Message>>>>>; 

#[derive(Serialize)]
struct LoadResponse {
    text: String,
    file: Option<Vec<u8>>, // Assuming the file is binary data
	file_name: String,
}

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

	let save_file = warp::path("save_file")
		.and(warp::post())
		.and(warp::body::json())
        .and(mongodb_filter.clone())
        .and_then(|body: Value, mongodb: MongoDB| async move {
            let file = body["file"].as_array().map(|arr| arr.iter().map(|v| v.as_u64().unwrap() as u8).collect::<Vec<u8>>());
            let path = body["path"].as_str().unwrap();
			let file_name = body["file_name"].as_str().unwrap();
            handle_save_file(file, file_name, path, mongodb).await
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
	let routes = chat.or(load).or(save_text).or(save_file).or(routes).recover(handle_rejection);
	
	//println!("Server running HTTP");
    // Start the warp server
    warp::serve(routes)
        .run(([0, 0, 0, 0], 80))
        .await;
}

async fn user_connected(ws: WebSocket, path: String, rooms: Rooms) {
    // Generate a unique ID for this user
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    //eprintln!("new chat user (ID: {}, Path: {})", my_id, path);

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
                eprintln!("websocket error: {}", e);
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
    //eprintln!("good bye user (ID: {}, Path: {})", my_id, path);

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

    match mongodb.save_data(&path, Some(&text), None, &"").await {
        Ok(_) => {
            //println!("Saved text for path '{}': {}", path, text);
            Ok(warp::reply::json(&format!("Saved text for path '{}'", path)))
        }
        Err(err) => {
            eprintln!("Failed to save data: {:?}", err);
            Err(warp::reject::custom(CustomError::from_mongo_error(err)))
        }
    }
}

async fn handle_save_file(
	file: Option<Vec<u8>>,
	file_name: &str,
	path: &str,
	mongodb: MongoDB
) -> Result<impl warp::Reply, warp::Rejection> {
	// Check if the file size is over 1MB
    if let Some(ref file_data) = file {
        const MAX_FILE_SIZE: usize = 1 * 1024 * 1024; // 1MB in bytes
        if file_data.len() > MAX_FILE_SIZE {
            eprintln!("File size exceeds 1MB for path '{}'", path);
            return Ok(warp::reply::json(&serde_json::json!({ "success": false, "error": "File size exceeds 1MB" })));
        }
    }
	
	match mongodb.save_data(path, None, file, file_name).await {
        Ok(_) => {
            //println!("Saved file '{}' for path '{}'", file_name, path);
            Ok(warp::reply::json(&serde_json::json!({ "success": true })))
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
            //println!("Loaded text for path '{}': {}", path, data.text);
            let file_data = data.file.map(|binary| binary.bytes); // Transform Binary to Vec<u8>
            let response = LoadResponse {
                text: data.text,
                file: file_data,
				file_name: data.file_name,
            };
            Ok(warp::reply::json(&response))
        }
        Ok(_) => {
            //println!("No data found for path '{}'", path);
            let response = LoadResponse {
                text: "".to_string(),
                file: None,
				file_name: "".to_string(),
            };
            Ok(warp::reply::json(&response))
        }
        Err(err) => {
            eprintln!("Failed to retrieve data: {:?}", err);
            // Optionally, handle the error more gracefully here
            Err(warp::reject::custom(CustomError::from_mongo_error(err)))
        }
    }
}

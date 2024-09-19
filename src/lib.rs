use warp::http::StatusCode;
use warp::Rejection;
use warp::Reply;

#[derive(Debug)]
pub struct CustomError {
    pub message: String,
}

impl CustomError {
    pub fn from_mongo_error(err: mongodb::error::Error) -> Self {
        CustomError {
            message: format!("MongoDB error: {}", err),
        }
    }
}

impl warp::reject::Reject for CustomError {}

pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    if let Some(custom_err) = err.find::<CustomError>() {
        let json = warp::reply::json(&format!("Error: {}", custom_err.message));
        Ok(warp::reply::with_status(json, StatusCode::INTERNAL_SERVER_ERROR))
    } else {
        // Handle other rejections here (e.g., path not found)
        let json = warp::reply::json(&"Unknown error occurred");
        Ok(warp::reply::with_status(json, StatusCode::INTERNAL_SERVER_ERROR))
    }
}
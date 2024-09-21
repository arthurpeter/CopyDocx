use mongodb::{Client, options::*, Collection, bson::doc, IndexModel};
use serde::{Deserialize, Serialize};
use std::env;
use bson::DateTime;

// Define the structure of the data to be saved in MongoDB
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveData {
    pub path: String,
    pub text: String,
	pub updated_at: DateTime,
}

#[derive(Clone)]
pub struct MongoDB {
    collection: Collection<SaveData>,
}

impl MongoDB {
    // Initialize the MongoDB client and collection
    pub async fn init() -> Self {
        let mongo_uri = env::var("MONGODB_URI").expect("MONGODB_URI must be set");
        let client_options = ClientOptions::parse(&mongo_uri).await.unwrap();
        let client = Client::with_options(client_options).unwrap();

        // Replace `your_database` and `your_collection` with actual names
        let db = client.database("copydocx");
        let collection = db.collection::<SaveData>("text_boxes");

        MongoDB { collection }
    }

    // Function to save data to MongoDB
    pub async fn save_data(&self, path: &str, text: &str) -> mongodb::error::Result<()> {
		let filter = doc!("path": path);
		let existing_document = self.collection.find_one(filter.clone()).await?;

		let current_time = DateTime::now();

		if let Some(_) = existing_document {
			let update = doc! { "$set": { 
                "text": text.to_string(),
                "updated_at": current_time,
            }};
			self.collection.update_one(filter, update).await?;
		} else  {
			let data = SaveData {
				path: path.to_string(),
				text: text.to_string(),
				updated_at: current_time,
			};
			self.collection.insert_one(data).await?;
		}

        Ok(())
    }

	pub async fn retrieve_data(&self, path: &str) -> mongodb::error::Result<Option<String>> {
        let filter = doc! { "path": path };
        if let Some(document) = self.collection.find_one(filter).await? {
            Ok(Some(document.text))
        } else {
            Ok(None)
        }
    }

	pub async fn create_ttl_index(&self) -> mongodb::error::Result<()> {
        let index_model = IndexModel::builder()
        .keys(doc! { "updated_at": 1 }) // Specify the field to index
        .options(Some(IndexOptions::builder()
            .expire_after(Some(std::time::Duration::from_secs(3600)))  // 1 hour expiration
            .build()))
        .build();

		// Create the index on the collection
		self.collection.create_index(index_model).await?;

		Ok(())
    }
}

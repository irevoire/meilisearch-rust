use crate::{
    indexes::{IndexStats, JsonIndex},
    prelude::*,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// The top-level struct of the SDK, representing a client containing [indexes](../indexes/struct.Index.html).
#[derive(Debug, Clone)]
pub struct Client {
    pub(crate) host: Rc<String>,
    pub(crate) api_key: Rc<String>,
}

impl Client {
    /// Create a client using the specified server.
    /// Don't put a '/' at the end of the host.
    /// In production mode, see [the documentation about authentication](https://docs.meilisearch.com/reference/features/authentication.html#authentication).
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::prelude::*;
    /// #
    /// // create the client
    /// let client = Client::new("http://localhost:7700", "masterKey");
    /// ```
    pub fn new(host: impl Into<String>, api_key: impl Into<String>) -> Client {
        Client {
            host: Rc::new(host.into()),
            api_key: Rc::new(api_key.into()),
        }
    }

    /*
    TODO: restore this method

    /// List all [indexes](../indexes/struct.Index.html).
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::prelude::*;
    /// # futures::executor::block_on(async move {
    /// // create the client
    /// let client = Client::new("http://localhost:7700", "masterKey");
    ///
    /// let indexes: Vec<Index> = client.list_all_indexes().await.unwrap();
    /// println!("{:?}", indexes);
    /// # });
    /// ```
    pub async fn list_all_indexes(&self) -> Result<Vec<Index>, Error> {
        let json_indexes = request::<(), Vec<JsonIndex>>(
            &format!("{}/indexes", self.host),
            &self.api_key,
            Method::Get,
            200,
        ).await?;

        let mut indexes = Vec::new();
        for json_index in json_indexes {
            indexes.push(json_index.into_index(self))
        }

        Ok(indexes)
    }
    */

    /// Get an [`Index`].
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::doc_tests::*;
    /// # doc_test(async {
    /// // create the client
    /// let client = Client::new("http://localhost:7700", "masterKey");
    ///
    /// // get the index called movies
    /// // you need to define the `Movie` document before
    /// let movies = client.get_index::<Movie>("movies").await;
    /// # });
    /// ```
    pub async fn get_index<Document: crate::document::Document>(
        &self,
        uid: &str,
    ) -> Result<Index<Document>, Error> {
        Ok(request::<(), JsonIndex>(
            &format!("{}/indexes/{}", self.host, uid),
            &self.api_key,
            Method::Get,
            200,
        )
        .await?
        .into_index(self))
    }

    /// Assume that an [index](../indexes/struct.Index.html) exist and create a corresponding object without any check.
    pub fn assume_index<Document: crate::document::Document>(
        &self,
        uid: impl Into<String>,
    ) -> Index<Document> {
        Index {
            uid: Rc::new(uid.into()),
            host: Rc::clone(&self.host),
            api_key: Rc::clone(&self.api_key),
            _phantom_document: std::marker::PhantomData,
        }
    }

    /// Creates an [Index].  
    ///   
    /// The second parameter will be used as the primary key of the new index.  
    /// If it is not specified, MeiliSearch will **try** to infer the primary key.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::doc_tests::*;
    /// # doc_test(async {
    /// // create the client
    /// let client = Client::new("http://localhost:7700", "masterKey");
    ///
    /// // create a new index called movies and access it
    /// // you need to define the `Movie` document before
    /// let movies = client.create_index::<Movie>("movies", None).await;
    /// # });
    /// ```
    pub async fn create_index<Document: crate::document::Document>(
        &self,
        uid: &str,
        primary_key: Option<&str>,
    ) -> Result<Index<Document>, Error> {
        Ok(request::<Value, JsonIndex>(
            &format!("{}/indexes", self.host),
            &self.api_key,
            Method::Post(json!({
                "uid": uid,
                "primaryKey": primary_key,
            })),
            201,
        )
        .await?
        .into_index(self))
    }

    /// Delete an index from its UID if it exists.
    /// To delete an index if it exists from the [`Index`] object, use the [Index::delete_if_exists] method.
    pub async fn delete_index_if_exists(&self, uid: &str) -> Result<bool, Error> {
        match self.delete_index(uid).await {
            Ok(_) => Ok(true),
            Err(Error::MeiliSearchError {
                message: _,
                error_code: ErrorCode::IndexNotFound,
                error_type: _,
                error_link: _,
            }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Delete an index from its UID.
    /// To delete an index from the [`Index`], use [the delete method](../indexes/struct.Index.html#method.delete).
    pub async fn delete_index(&self, uid: &str) -> Result<(), Error> {
        Ok(request::<(), ()>(
            &format!("{}/indexes/{}", self.host, uid),
            &self.api_key,
            Method::Delete,
            204,
        )
        .await?)
    }

    /// This will try to get an index and create the index if it does not exist.
    pub async fn get_or_create<Document: crate::document::Document>(
        &self,
        uid: &str,
    ) -> Result<Index<Document>, Error> {
        if let Ok(index) = self.get_index(uid).await {
            Ok(index)
        } else {
            self.create_index(uid, None).await
        }
    }

    /*
    TODO: restore function

    /// Alias for [list_all_indexes](#method.list_all_indexes).
    pub async fn get_indexes(&self) -> Result<Vec<Index>, Error> {
        self.list_all_indexes().await
    }
    */

    /// Get stats of all indexes.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::doc_tests::*;
    /// # doc_test(async {
    /// # let (client, index) = init_doc_test("get_stats_doc_test").await;
    /// let stats = client.get_stats().await.unwrap();
    /// # });
    /// ```
    pub async fn get_stats(&self) -> Result<ClientStats, Error> {
        request::<serde_json::Value, ClientStats>(
            &format!("{}/stats", self.host),
            &self.api_key,
            Method::Get,
            200,
        )
        .await
    }

    /// Get health of MeiliSearch server.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::doc_tests::*;
    /// # doc_test(async {
    /// # let (client, index) = init_doc_test("health_doc_test").await;
    /// let health = client.health().await.unwrap();
    /// # });
    /// ```
    pub async fn health(&self) -> Result<Health, Error> {
        request::<serde_json::Value, Health>(
            &format!("{}/health", self.host),
            &self.api_key,
            Method::Get,
            200,
        )
        .await
    }

    /// Get health of MeiliSearch server, return true or false.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::doc_tests::*;
    /// # doc_test(async {
    /// # let (client, index) = init_doc_test("is_healthy_doc_test").await;
    /// let healthy = client.is_healthy().await;
    /// assert_eq!(healthy, true);
    /// # });
    /// ```
    pub async fn is_healthy(&self) -> bool {
        if let Ok(health) = self.health().await {
            health.status.as_str() == "available"
        } else {
            false
        }
    }

    /// Get the private and public key.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::doc_tests::*;
    /// # doc_test(async {
    /// # let (client, index) = init_doc_test("get_keys_doc_test").await;
    /// let keys = client.get_keys().await.unwrap();
    /// println!("public key: {:?}", keys.public);
    /// println!("private key: {:?}", keys.private);
    /// # });
    /// ```
    pub async fn get_keys(&self) -> Result<Keys, Error> {
        request::<(), Keys>(
            &format!("{}/keys", self.host),
            &self.api_key,
            Method::Get,
            200,
        )
        .await
    }

    /// Get version of the MeiliSearch server.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::doc_tests::*;
    /// # doc_test(async {
    /// # let (client, index) = init_doc_test("get_version_doc_test").await;
    /// let version = client.get_version().await.unwrap();
    /// # });
    /// ```
    pub async fn get_version(&self) -> Result<Version, Error> {
        request::<(), Version>(
            &format!("{}/version", self.host),
            &self.api_key,
            Method::Get,
            200,
        )
        .await
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStats {
    pub database_size: usize,
    pub last_update: Option<String>,
    pub indexes: HashMap<String, IndexStats>,
}

/// Health of the MeiliSearch server.
///
/// Example:
///
/// ```
/// # use meilisearch_sdk::prelude::*;
/// Health {
///    status: "available".to_string(),
/// };
/// ```
#[derive(Deserialize)]
pub struct Health {
    pub status: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Keys {
    pub public: Option<String>,
    pub private: Option<String>,
}

/// Version of a MeiliSearch server.
///
/// Example:
///
/// ```
/// # use meilisearch_sdk::{prelude::*, client::Version};
/// Version {
///    commit_sha: "b46889b5f0f2f8b91438a08a358ba8f05fc09fc1".to_string(),
///    build_date: "2019-11-15T09:51:54.278247+00:00".to_string(),
///    pkg_version: "0.1.1".to_string(),
/// };
/// ```
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub commit_sha: String,
    pub build_date: String,
    pub pkg_version: String,
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use futures_await_test::async_test;

    #[async_test]
    async fn test_get_keys() {
        let client = Client::new("http://localhost:7700", "masterKey");
        client.get_keys().await.unwrap();
    }

    #[async_test]
    async fn test_delete_if_exits() {
        let client = Client::new("http://localhost:7700", "masterKey");
        let index_name = "movies_delete_if_exists";
        client
            .create_index::<UnknownDocument>(index_name, None)
            .await
            .unwrap();
        let mut index = client.get_index::<UnknownDocument>(index_name).await;
        assert!(index.is_ok());
        let deleted = client.delete_index_if_exists(index_name).await.unwrap();
        assert!(deleted);
        index = client.get_index(index_name).await;
        assert!(index.is_err());
    }

    #[async_test]
    async fn test_delete_if_exits_none() {
        let client = Client::new("http://localhost:7700", "masterKey");
        assert!(!client.delete_index_if_exists("bad").await.unwrap());
    }
}

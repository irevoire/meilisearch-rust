use crate::{
    errors::*,
    indexes::*,
    key::{Key, KeyBuilder},
    request::*,
    tasks::{async_sleep, Task},
    Rc,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, time::Duration};

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
    /// # use meilisearch_sdk::{client::*, indexes::*};
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

    /// List all [Index]es and returns values as instances of [Index].
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, indexes::*};
    /// # futures::executor::block_on(async move {
    /// // create the client
    /// let client = Client::new("http://localhost:7700", "masterKey");
    ///
    /// let indexes: Vec<Index> = client.list_all_indexes().await.unwrap();
    /// println!("{:?}", indexes);
    /// # });
    /// ```
    pub async fn list_all_indexes(&self) -> Result<Vec<Index>, Error> {
        match self.list_all_indexes_raw().await {
            Ok(json_indexes) => Ok({
                let mut indexes = Vec::new();
                for json_index in json_indexes {
                    indexes.push(json_index.into_index(self))
                }
                indexes
            }),
            Err(error) => Err(error),
        }
    }

    /// List all [Index]es and returns as Json.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, indexes::*};
    /// # futures::executor::block_on(async move {
    /// // create the client
    /// let client = Client::new("http://localhost:7700", "masterKey");
    ///
    /// let json_indexes: Vec<JsonIndex> = client.list_all_indexes_raw().await.unwrap();
    /// println!("{:?}", json_indexes);
    /// # });
    /// ```
    pub async fn list_all_indexes_raw(&self) -> Result<Vec<JsonIndex>, Error> {
        let json_indexes = request::<(), Vec<JsonIndex>>(
            &format!("{}/indexes", self.host),
            &self.api_key,
            Method::Get,
            200,
        )
        .await?;

        Ok(json_indexes)
    }

    /// Get an [Index], this index should already exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, indexes::*};
    ///
    /// # futures::executor::block_on(async move {
    /// // create the client
    /// let client = Client::new("http://localhost:7700", "masterKey");
    /// # let index = client.create_index("movies", None).await.unwrap().wait_for_completion(&client, None, None).await.unwrap().try_make_index(&client).unwrap();
    ///
    /// // get the index named "movies"
    /// let movies = client.get_index("movies").await.unwrap();
    /// assert_eq!(movies.as_ref(), "movies");
    /// # index.delete().await.unwrap().wait_for_completion(&client, None, None).await.unwrap();
    /// # });
    /// ```
    pub async fn get_index(&self, uid: impl AsRef<str>) -> Result<Index, Error> {
        match self.get_raw_index(uid).await {
            Ok(raw_idx) => Ok(raw_idx.into_index(self)),
            Err(error) => Err(error),
        }
    }

    /// Get a raw JSON [Index], this index should already exist.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, indexes::*};
    ///
    /// # futures::executor::block_on(async move {
    /// // create the client
    /// let client = Client::new("http://localhost:7700", "masterKey");
    /// # let index = client.create_index("movies", None).await.unwrap().wait_for_completion(&client, None, None).await.unwrap().try_make_index(&client).unwrap();
    ///
    /// // get the index named "movies"
    /// let movies = client.get_raw_index("movies").await.unwrap();
    /// assert_eq!(movies.uid, "movies");
    /// # index.delete().await.unwrap().wait_for_completion(&client, None, None).await.unwrap();
    /// # });
    /// ```
    /// If you use it directly from an [Index], you can use the method [Index::fetch_info], which is the equivalent method from an index.
    pub async fn get_raw_index(&self, uid: impl AsRef<str>) -> Result<JsonIndex, Error> {
        Index::fetch_info(&self.index(uid.as_ref())).await
    }

    /// Create a corresponding object of an [Index] without any check or doing an HTTP call.
    pub fn index(&self, uid: impl Into<String>) -> Index {
        Index {
            uid: Rc::new(uid.into()),
            client: self.clone(),
        }
    }

    /// Create an [Index].
    /// The second parameter will be used as the primary key of the new index.
    /// If it is not specified, Meilisearch will **try** to infer the primary key.
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, indexes::*};
    /// #
    /// # futures::executor::block_on(async move {
    /// // Create the client
    /// let client = Client::new("http://localhost:7700", "masterKey");
    ///
    /// // Create a new index called movies and access it
    /// let task = client.create_index("movies", None).await.unwrap();
    ///
    /// // Wait for the task to complete
    /// let task = task.wait_for_completion(&client, None, None).await.unwrap();
    ///
    /// // Try to get the inner index if the task succeeded
    /// let index = task.try_make_index(&client).unwrap();
    ///
    /// assert_eq!(index.as_ref(), "movies");
    /// # index.delete().await.unwrap().wait_for_completion(&client, None, None).await.unwrap();
    /// # });
    /// ```
    pub async fn create_index(
        &self,
        uid: impl AsRef<str>,
        primary_key: Option<&str>,
    ) -> Result<Task, Error> {
        Ok(request::<Value, Task>(
            &format!("{}/indexes", self.host),
            &self.api_key,
            Method::Post(json!({
                "uid": uid.as_ref(),
                "primaryKey": primary_key,
            })),
            202,
        )
        .await?)
    }

    /// Delete an index from its UID.
    /// To delete an [Index], use the [Index::delete] method.
    pub async fn delete_index(&self, uid: impl AsRef<str>) -> Result<Task, Error> {
        Ok(request::<(), Task>(
            &format!("{}/indexes/{}", self.host, uid.as_ref()),
            &self.api_key,
            Method::Delete,
            202,
        )
        .await?)
    }

    /// Alias for [Client::list_all_indexes].
    pub async fn get_indexes(&self) -> Result<Vec<Index>, Error> {
        self.list_all_indexes().await
    }

    /// Alias for [Client::list_all_indexes_raw].
    pub async fn get_indexes_raw(&self) -> Result<Vec<JsonIndex>, Error> {
        self.list_all_indexes_raw().await
    }

    /// Get stats of all indexes.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, indexes::*};
    /// #
    /// # futures::executor::block_on(async move {
    /// let client = Client::new("http://localhost:7700", "masterKey");
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

    /// Get health of Meilisearch server.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, errors::{Error, ErrorCode}};
    /// #
    /// # futures::executor::block_on(async move {
    /// let client = Client::new("http://localhost:7700", "masterKey");
    /// let health = client.health().await.unwrap();
    /// assert_eq!(health.status, "available");
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

    /// Get health of Meilisearch server, return true or false.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::client::*;
    /// #
    /// # futures::executor::block_on(async move {
    /// let client = Client::new("http://localhost:7700", "masterKey");
    /// let health = client.is_healthy().await;
    /// assert_eq!(health, true);
    /// # });
    /// ```
    pub async fn is_healthy(&self) -> bool {
        if let Ok(health) = self.health().await {
            health.status.as_str() == "available"
        } else {
            false
        }
    }

    /// Get the API [Key]s from Meilisearch.
    /// See the [meilisearch documentation](https://docs.meilisearch.com/reference/api/keys.html#get-all-keys).
    ///
    /// See also [Client::create_key] and [Client::get_key].
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, errors::Error, key::KeyBuilder};
    /// #
    /// # futures::executor::block_on(async move {
    /// let client = Client::new("http://localhost:7700", "masterKey");
    /// let keys = client.get_keys().await.unwrap();
    /// assert!(keys.len() >= 2);
    /// # });
    /// ```
    pub async fn get_keys(&self) -> Result<Vec<Key>, Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Keys {
            #[serde(rename = "results")]
            pub inner: Vec<Key>,
        }

        let keys = request::<(), Keys>(
            &format!("{}/keys", self.host),
            &self.api_key,
            Method::Get,
            200,
        )
        .await?;

        Ok(keys.inner)
    }

    /// Get one API [Key] from Meilisearch.
    /// See the [meilisearch documentation](https://docs.meilisearch.com/reference/api/keys.html#get-one-key).
    ///
    /// See also [Client::create_key] and [Client::get_keys].
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, errors::Error, key::KeyBuilder};
    /// #
    /// # futures::executor::block_on(async move {
    /// let client = Client::new("http://localhost:7700", "masterKey");
    /// # let key = client.get_keys().await.unwrap().into_iter().find(|k| k.description.starts_with("Default Search API Key")).unwrap();
    /// let key_id = // enter your API key here, for the example we'll say we entered our search API key.
    /// # key.key;
    /// let key = client.get_key(key_id).await.unwrap();
    /// assert_eq!(key.description, "Default Search API Key (Use it to search from the frontend)");
    /// # });
    /// ```
    pub async fn get_key(&self, key: impl AsRef<str>) -> Result<Key, Error> {
        request::<(), Key>(
            &format!("{}/keys/{}", self.host, key.as_ref()),
            &self.api_key,
            Method::Get,
            200,
        )
        .await
    }

    /// Delete an API [Key] from Meilisearch.
    /// See the [meilisearch documentation](https://docs.meilisearch.com/reference/api/keys.html#delete-a-key).
    ///
    /// See also [Client::create_key], [Client::update_key] and [Client::get_key].
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, errors::Error, key::KeyBuilder};
    /// #
    /// # futures::executor::block_on(async move {
    /// let client = Client::new("http://localhost:7700", "masterKey");
    /// let key = KeyBuilder::new("delete_key");
    /// let key = client.create_key(key).await.unwrap();
    /// let inner_key = key.key.clone();
    ///
    /// client.delete_key(key).await.unwrap();
    ///
    /// let keys = client.get_keys().await.unwrap();
    /// assert!(keys.iter().all(|key| key.key != inner_key));
    /// # });
    /// ```
    pub async fn delete_key(&self, key: impl AsRef<str>) -> Result<(), Error> {
        request::<(), ()>(
            &format!("{}/keys/{}", self.host, key.as_ref()),
            &self.api_key,
            Method::Delete,
            204,
        )
        .await
    }

    /// Create an API [Key] in Meilisearch.
    /// See the [meilisearch documentation](https://docs.meilisearch.com/reference/api/keys.html#create-a-key).
    ///
    /// See also [Client::update_key], [Client::delete_key] and [Client::get_key].
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, errors::Error, key::KeyBuilder, key::Action};
    /// #
    /// # futures::executor::block_on(async move {
    /// let client = Client::new("http://localhost:7700", "masterKey");
    /// let mut key = KeyBuilder::new("create_key");
    /// key.with_index("*").with_action(Action::DocumentsAdd);
    /// let key = client.create_key(key).await.unwrap();
    /// assert_eq!(key.description, "create_key");
    /// # client.delete_key(key).await.unwrap();
    /// # });
    /// ```
    pub async fn create_key(&self, key: impl AsRef<KeyBuilder>) -> Result<Key, Error> {
        request::<&KeyBuilder, Key>(
            &format!("{}/keys", self.host),
            &self.api_key,
            Method::Post(key.as_ref()),
            201,
        )
        .await
    }

    /// Update an API [Key] in Meilisearch.
    /// See the [meilisearch documentation](https://docs.meilisearch.com/reference/api/keys.html#update-a-key).
    ///
    /// See also [Client::create_key], [Client::delete_key] and [Client::get_key].
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, errors::Error, key::KeyBuilder};
    /// #
    /// # futures::executor::block_on(async move {
    /// let client = Client::new("http://localhost:7700", "masterKey");
    /// let key = KeyBuilder::new("update_key");
    /// let mut key = client.create_key(key).await.unwrap();
    /// assert!(key.indexes.is_empty());
    ///
    /// key.indexes = vec!["*".to_string()];
    /// let key = client.update_key(key).await.unwrap();
    /// assert_eq!(key.indexes, vec!["*"]);
    /// # client.delete_key(key).await.unwrap();
    /// # });
    /// ```
    pub async fn update_key(&self, key: impl AsRef<Key>) -> Result<Key, Error> {
        request::<&Key, Key>(
            &format!("{}/keys/{}", self.host, key.as_ref().key),
            &self.api_key,
            Method::Patch(key.as_ref()),
            200,
        )
        .await
    }

    /// Get version of the Meilisearch server.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*};
    /// #
    /// # futures::executor::block_on(async move {
    /// let client = Client::new("http://localhost:7700", "masterKey");
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

    /// Wait until Meilisearch processes a [Task], and get its status.
    ///
    /// `interval` = The frequency at which the server should be polled. Default = 50ms
    /// `timeout` = The maximum time to wait for processing to complete. Default = 5000ms
    ///
    /// If the waited time exceeds `timeout` then an [Error::Timeout] will be returned.
    ///
    /// See also [Index::wait_for_task, Task::wait_for_completion].
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::{client::*, document, indexes::*, tasks::Task};
    /// # use serde::{Serialize, Deserialize};
    /// #
    /// # #[derive(Debug, Serialize, Deserialize, PartialEq)]
    /// # struct Document {
    /// #    id: usize,
    /// #    value: String,
    /// #    kind: String,
    /// # }
    /// #
    /// # impl document::Document for Document {
    /// #    type UIDType = usize;
    /// #
    /// #    fn get_uid(&self) -> &Self::UIDType {
    /// #        &self.id
    /// #    }
    /// # }
    /// #
    /// # futures::executor::block_on(async move {
    /// let client = Client::new("http://localhost:7700", "masterKey");
    /// let movies = client.index("movies_client_wait_for_task");
    ///
    /// let task = movies.add_documents(&[
    ///     Document { id: 0, kind: "title".into(), value: "The Social Network".to_string() },
    ///     Document { id: 1, kind: "title".into(), value: "Harry Potter and the Sorcerer's Stone".to_string() },
    /// ], None).await.unwrap();
    ///
    /// let status = client.wait_for_task(task, None, None).await.unwrap();
    ///
    /// assert!(matches!(status, Task::Succeeded { .. }));
    /// # movies.delete().await.unwrap().wait_for_completion(&client, None, None).await.unwrap();
    /// # });
    /// ```
    pub async fn wait_for_task(
        &self,
        task_id: impl AsRef<u64>,
        interval: Option<Duration>,
        timeout: Option<Duration>,
    ) -> Result<Task, Error> {
        let interval = interval.unwrap_or_else(|| Duration::from_millis(50));
        let timeout = timeout.unwrap_or_else(|| Duration::from_millis(5000));

        let mut elapsed_time = Duration::new(0, 0);
        let mut task_result: Result<Task, Error>;

        while timeout > elapsed_time {
            task_result = self.get_task(&task_id).await;

            match task_result {
                Ok(status) => match status {
                    Task::Failed { .. } | Task::Succeeded { .. } => {
                        return self.get_task(task_id).await;
                    }
                    Task::Enqueued { .. } | Task::Processing { .. } => {
                        elapsed_time += interval;
                        async_sleep(interval).await;
                    }
                },
                Err(error) => return Err(error),
            };
        }

        Err(Error::Timeout)
    }

    /// Get a task from the server given a task id.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::*;
    /// # futures::executor::block_on(async move {
    /// # let client = client::Client::new("http://localhost:7700", "masterKey");
    /// # let index = client.create_index("movies_get_task", None).await.unwrap().wait_for_completion(&client, None, None).await.unwrap().try_make_index(&client).unwrap();
    /// let task = index.delete_all_documents().await.unwrap();
    /// let task = client.get_task(task).await.unwrap();
    /// # index.delete().await.unwrap().wait_for_completion(&client, None, None).await.unwrap();
    /// # });
    /// ```
    pub async fn get_task(&self, task_id: impl AsRef<u64>) -> Result<Task, Error> {
        request::<(), Task>(
            &format!("{}/tasks/{}", self.host, task_id.as_ref()),
            &self.api_key,
            Method::Get,
            200,
        )
        .await
    }

    /// Get all tasks from the server.
    ///
    /// # Example
    ///
    /// ```
    /// # use meilisearch_sdk::*;
    /// # futures::executor::block_on(async move {
    /// # let client = client::Client::new("http://localhost:7700", "masterKey");
    /// let tasks = client.get_tasks().await.unwrap();
    /// # });
    /// ```
    pub async fn get_tasks(&self) -> Result<Vec<Task>, Error> {
        #[derive(Deserialize)]
        struct Tasks {
            pub results: Vec<Task>,
        }

        let tasks = request::<(), Tasks>(
            &format!("{}/tasks", self.host),
            &self.api_key,
            Method::Get,
            200,
        )
        .await?;

        Ok(tasks.results)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStats {
    pub database_size: usize,
    pub last_update: Option<String>,
    pub indexes: HashMap<String, IndexStats>,
}

/// Health of the Meilisearch server.
///
/// Example:
///
/// ```
/// # use meilisearch_sdk::{client::*, indexes::*, errors::Error};
/// Health {
///    status: "available".to_string(),
/// };
/// ```
#[derive(Deserialize)]
pub struct Health {
    pub status: String,
}

/// Version of a Meilisearch server.
///
/// Example:
///
/// ```
/// # use meilisearch_sdk::{client::*, indexes::*, errors::Error};
/// Version {
///    commit_sha: "b46889b5f0f2f8b91438a08a358ba8f05fc09fc1".to_string(),
///    commit_date: "2019-11-15T09:51:54.278247+00:00".to_string(),
///    pkg_version: "0.1.1".to_string(),
/// };
/// ```
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub commit_sha: String,
    pub commit_date: String,
    pub pkg_version: String,
}

#[cfg(test)]
mod tests {
    use crate::{
        client::*,
        key::{Action, KeyBuilder},
    };
    use meilisearch_test_macro::meilisearch_test;

    #[meilisearch_test]
    async fn test_get_keys(client: Client) {
        let keys = client.get_keys().await.unwrap();
        assert!(keys.len() >= 2);
        assert!(keys.iter().any(
            |k| k.description != "Default Search API Key (Use it to search from the frontend)"
        ));
        assert!(keys.iter().any(
            |k| k.description != "Default Admin API Key (Use it for all other operations. Caution! Do not use it on a public frontend)"
        ));
    }

    #[meilisearch_test]
    async fn test_delete_key(client: Client, description: String) {
        let key = KeyBuilder::new(description);
        let key = client.create_key(key).await.unwrap();

        client.delete_key(&key).await.unwrap();
        let keys = client.get_keys().await.unwrap();
        assert!(keys.iter().all(|k| k.key != key.key));
    }

    #[meilisearch_test]
    async fn test_error_delete_key(mut client: Client, description: String) {
        // ==> accessing a key that does not exist
        let error = client.delete_key("invalid_key").await.unwrap_err();
        assert!(matches!(
            error,
            Error::Meilisearch(MeilisearchError {
                error_code: ErrorCode::ApiKeyNotFound,
                error_type: ErrorType::InvalidRequest,
                ..
            })
        ));

        // ==> executing the action without enough right
        let key = KeyBuilder::new(description);
        let key = client.create_key(key).await.unwrap();

        let master_key = client.api_key.clone();
        // this key has no right
        client.api_key = Rc::new(key.key.clone());
        // with a wrong key
        let error = client.delete_key("invalid_key").await.unwrap_err();
        assert!(matches!(
            error,
            Error::Meilisearch(MeilisearchError {
                error_code: ErrorCode::InvalidApiKey,
                error_type: ErrorType::Auth,
                ..
            })
        ));
        // with a good key
        let error = client.delete_key(&key.key).await.unwrap_err();
        assert!(matches!(
            error,
            Error::Meilisearch(MeilisearchError {
                error_code: ErrorCode::InvalidApiKey,
                error_type: ErrorType::Auth,
                ..
            })
        ));

        // cleanup
        client.api_key = master_key;
        client.delete_key(key).await.unwrap();
    }

    #[meilisearch_test]
    async fn test_create_key(client: Client, description: String) {
        let mut key = KeyBuilder::new(description.clone());
        key.with_action(Action::DocumentsAdd)
            .with_expires_at("3022-02-09T01:32:46Z")
            .with_index("*");
        let key = client.create_key(key).await.unwrap();

        assert_eq!(key.actions, vec![Action::DocumentsAdd]);
        assert_eq!(key.description, description);
        assert_eq!(key.expires_at, Some("3022-02-09T01:32:46Z".to_string()));
        assert_eq!(key.indexes, vec!["*".to_string()]);

        let keys = client.get_keys().await.unwrap();

        let remote_key = keys.iter().find(|k| k.key == key.key).unwrap();

        assert_eq!(remote_key.actions, vec![Action::DocumentsAdd]);
        assert_eq!(remote_key.description, description);
        assert_eq!(
            remote_key.expires_at,
            Some("3022-02-09T01:32:46Z".to_string())
        );
        assert_eq!(remote_key.indexes, vec!["*".to_string()]);

        client.delete_key(key).await.unwrap();
    }

    #[meilisearch_test]
    async fn test_error_create_key(mut client: Client, description: String) {
        // ==> Invalid index name
        /* TODO: uncomment once meilisearch fix this bug: https://github.com/meilisearch/meilisearch/issues/2158
        let mut key = KeyBuilder::new(&description);
        key.with_index("invalid index # / \\name with spaces");
        let error = client.create_key(key).await.unwrap_err();

        assert!(matches!(
            error,
            Error::MeilisearchError {
                error_code: ErrorCode::InvalidApiKeyIndexes,
                error_type: ErrorType::InvalidRequest,
                ..
            }
        ));
        */

        // ==> Invalid expires_at
        let mut key = KeyBuilder::new(&description);
        key.with_expires_at("That’s totally not a date");
        let error = client.create_key(key).await.unwrap_err();

        assert!(matches!(
            error,
            Error::Meilisearch(MeilisearchError {
                error_code: ErrorCode::InvalidApiKeyExpiresAt,
                error_type: ErrorType::InvalidRequest,
                ..
            })
        ));

        // ==> executing the action without enough right
        let no_right_key = KeyBuilder::new(&description);
        let no_right_key = client.create_key(no_right_key).await.unwrap();

        // backup the master key for cleanup at the end of the test
        let master_client = client.clone();
        client.api_key = Rc::new(no_right_key.key.clone());

        let key = KeyBuilder::new(&description);
        let error = client.create_key(key).await.unwrap_err();

        assert!(matches!(
            error,
            Error::Meilisearch(MeilisearchError {
                error_code: ErrorCode::InvalidApiKey,
                error_type: ErrorType::Auth,
                ..
            })
        ));

        // cleanup
        master_client.delete_key(&*client.api_key).await.unwrap();
    }

    #[meilisearch_test]
    async fn test_update_key(client: Client, description: String) {
        let key = KeyBuilder::new(description.clone());
        let mut key = client.create_key(key).await.unwrap();

        key.actions = vec![Action::DocumentsAdd];
        key.expires_at = Some("3022-02-09T01:32:46Z".to_string());
        key.indexes = vec!["*".to_string()];

        let key = client.update_key(key).await.unwrap();

        assert_eq!(key.actions, vec![Action::DocumentsAdd]);
        assert_eq!(key.description, description);
        assert_eq!(key.expires_at, Some("3022-02-09T01:32:46Z".to_string()));
        assert_eq!(key.indexes, vec!["*".to_string()]);

        let keys = client.get_keys().await.unwrap();

        let remote_key = keys.iter().find(|k| k.key == key.key).unwrap();

        assert_eq!(remote_key.actions, vec![Action::DocumentsAdd]);
        assert_eq!(remote_key.description, description);
        assert_eq!(
            remote_key.expires_at,
            Some("3022-02-09T01:32:46Z".to_string())
        );
        assert_eq!(remote_key.indexes, vec!["*".to_string()]);

        client.delete_key(key).await.unwrap();
    }

    #[meilisearch_test]
    async fn test_error_update_key(mut client: Client, description: String) {
        let key = KeyBuilder::new(description.clone());
        let mut key = client.create_key(key).await.unwrap();

        // ==> Invalid index name
        /* TODO: uncomment once meilisearch fix this bug: https://github.com/meilisearch/meilisearch/issues/2158
        key.indexes = vec!["invalid index # / \\name with spaces".to_string()];
        let error = client.update_key(key).await.unwrap_err();

        assert!(matches!(
            error,
            Error::MeilisearchError {
                error_code: ErrorCode::InvalidApiKeyIndexes,
                error_type: ErrorType::InvalidRequest,
                ..
            }
        ));
        */

        // ==> Invalid expires_at
        key.expires_at = Some("That’s totally not a date".to_string());
        let error = client.update_key(&key).await.unwrap_err();

        assert!(matches!(
            error,
            Error::Meilisearch(MeilisearchError {
                error_code: ErrorCode::InvalidApiKeyExpiresAt,
                error_type: ErrorType::InvalidRequest,
                ..
            })
        ));
        key.expires_at = None;

        // ==> executing the action without enough right
        let no_right_key = KeyBuilder::new(&description);
        let no_right_key = client.create_key(no_right_key).await.unwrap();

        // backup the master key for cleanup at the end of the test
        let master_client = client.clone();
        client.api_key = Rc::new(no_right_key.key.clone());

        let error = client.update_key(key).await.unwrap_err();

        assert!(matches!(
            error,
            Error::Meilisearch(MeilisearchError {
                error_code: ErrorCode::InvalidApiKey,
                error_type: ErrorType::Auth,
                ..
            })
        ));

        // cleanup
        master_client.delete_key(&*client.api_key).await.unwrap();
    }

    #[meilisearch_test]
    async fn test_get_index(client: Client, index_uid: String) -> Result<(), Error> {
        let task = client.create_index(&index_uid, None).await?;
        let index = client
            .wait_for_task(task, None, None)
            .await?
            .try_make_index(&client)
            .unwrap();

        assert_eq!(index.uid.to_string(), index_uid);
        index
            .delete()
            .await?
            .wait_for_completion(&client, None, None)
            .await?;
        Ok(())
    }

    #[meilisearch_test]
    async fn test_list_all_indexes(client: Client, index: Index) {
        let all_indexes = client.list_all_indexes().await.unwrap();
        assert!(all_indexes.len() > 0);
        assert!(all_indexes.iter().any(|idx| idx.uid == index.uid));
    }

    #[meilisearch_test]
    async fn test_list_all_indexes_raw(client: Client, index: Index) {
        let all_indexes_raw = client.list_all_indexes_raw().await.unwrap();
        assert!(all_indexes_raw.len() > 0);
        assert!(all_indexes_raw.iter().any(|idx| idx.uid == *index.uid));
    }

    #[meilisearch_test]
    async fn test_fetch_info(index: Index) {
        let index = index.fetch_info().await;
        assert!(index.is_ok());
    }

    #[meilisearch_test]
    async fn test_get_primary_key_is_none(index: Index) {
        let primary_key = index.get_primary_key().await;

        assert!(primary_key.is_ok());
        assert!(primary_key.unwrap().is_none());
    }

    #[meilisearch_test]
    async fn test_get_primary_key(client: Client, index_uid: String) -> Result<(), Error> {
        let index = client
            .create_index(index_uid, Some("primary_key"))
            .await?
            .wait_for_completion(&client, None, None)
            .await?
            .try_make_index(&client)
            .unwrap();

        let primary_key = index.get_primary_key().await;
        assert!(primary_key.is_ok());
        assert_eq!(primary_key?.unwrap(), "primary_key");

        index
            .delete()
            .await?
            .wait_for_completion(&client, None, None)
            .await?;

        Ok(())
    }
}

use crate::common::*;

use crate::utils_module::io_utils::*;

use crate::config::config_settings::*;

static ELASTICSEARCH_CONN_SEMAPHORE_POOL: once_lazy<Vec<Arc<EsRepositoryPub>>> = once_lazy::new(
    || {
        let es_host: Vec<String> = ES_DB_URL.split(',').map(|s| s.trim().to_string()).collect();
        let es_pool_size: usize = ES_POOL_SIZE.parse::<usize>()
            .expect("[Error][ELASTICSEARCH_CONN_SEMAPHORE_POOL] There was a problem converting ES_POOL_SIZE to usize.");
        
        (0..es_pool_size)
        .map(|_| {
            Arc::new(
                EsRepositoryPub::new(es_host.clone(), &ES_ID, &ES_PW)
                    .expect("[Error][ELASTICSEARCH_CONN_SEMA_POOL] Failed to create Elasticsearch client"),
            )
        })
        .collect()
    }
);


static SEMAPHORE: once_lazy<Arc<Semaphore>> = once_lazy::new(|| {
    let es_pool_size: usize = ES_POOL_SIZE.parse::<usize>()
            .expect("[Error][ELASTICSEARCH_CONN_SEMAPHORE_POOL] There was a problem converting ES_POOL_SIZE to usize.");

    Arc::new(Semaphore::new(es_pool_size))
});

#[derive(Debug)]
pub struct ElasticConnGuard {
    client: Arc<EsRepositoryPub>,
    _permit: OwnedSemaphorePermit, /* drop 시 자동 반환 */
}

impl ElasticConnGuard {
    pub async fn new() -> Result<Self, anyhow::Error> {
        info!("[ElasticConnGuard] Available permits: {}", SEMAPHORE.available_permits());
        let permit: OwnedSemaphorePermit = SEMAPHORE.clone().acquire_owned().await?;
        info!("[ElasticConnGuard] Acquired semaphore");

        /* 임의로 하나의 클라이언트를 가져옴 (랜덤 선택 가능) */
        let client: Arc<EsRepositoryPub> = ELASTICSEARCH_CONN_SEMAPHORE_POOL
            .choose(&mut rand::thread_rng())
            .cloned()
            .expect("[Error][EalsticConnGuard -> new] No clients available");

        Ok(Self {
            client,
            _permit: permit, /* Drop 시 자동 반환 */
        })
    }
}

impl Deref for ElasticConnGuard {
    type Target = EsRepositoryPub;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl Drop for ElasticConnGuard {
    fn drop(&mut self) {
        info!("[ElasticConnGuard] permit dropped (semaphore released)");
    }
}

pub async fn get_elastic_guard_conn() -> Result<ElasticConnGuard, anyhow::Error> {
    info!("use elasticsearch connection");
    ElasticConnGuard::new().await
}

#[async_trait]
pub trait EsRepository {
    async fn process_response_empty(
        &self,
        function_name: &str,
        response: Response,
    ) -> Result<(), anyhow::Error>;
    async fn process_response(
        &self,
        function_name: &str,
        response: Response,
    ) -> Result<Value, anyhow::Error>;
    async fn get_search_query(
        &self,
        es_query: &Value,
        index_name: &str,
    ) -> Result<Value, anyhow::Error>;
    async fn post_query(&self, document: &Value, index_name: &str) -> Result<(), anyhow::Error>;
    async fn post_query_struct<T: Serialize + Sync>(
        &self,
        param_struct: &T,
        index_name: &str,
    ) -> Result<(), anyhow::Error>;
    async fn create_index(
        &self,
        index_name: &str,
        index_setting_json: &Value,
    ) -> Result<(), anyhow::Error>;
    async fn delete_query(&self, index_name: &str) -> Result<(), anyhow::Error>;
    async fn bulk_indexing_query<T: Serialize + Send + Sync>(
        &self,
        index_name: &str,
        data: &[T],
    ) -> Result<(), anyhow::Error>;
    async fn get_indexes_mapping_by_alias(
        &self,
        index_alias_name: &str,
    ) -> Result<Value, anyhow::Error>;
    async fn update_index_alias(
        &self,
        index_alias: &str,
        new_index_name: &str,
        old_index_name: &str,
    ) -> Result<(), anyhow::Error>;
    async fn refresh_index(&self, index_name: &str) -> Result<(), anyhow::Error>;
    async fn create_index_alias(
        &self,
        index_alias: &str,
        index_name: &str,
    ) -> Result<(), anyhow::Error>;
}

#[derive(Debug, Getters, Clone)]
pub struct EsRepositoryPub {
    es_clients: Vec<EsClient>,
}

#[derive(Debug, Getters, Clone, new)]
pub(crate) struct EsClient {
    host: String,
    es_conn: Elasticsearch,
}

impl EsRepositoryPub {
    pub fn new(es_url_vec: Vec<String>, es_id: &str, es_pw: &str) -> Result<Self, anyhow::Error> {
        let mut es_clients: Vec<EsClient> = Vec::new();

        for url in es_url_vec {
            let parse_url: String = format!("http://{es_id}:{es_pw}@{url}");

            let es_url: Url = Url::parse(&parse_url)?;
            let conn_pool: SingleNodeConnectionPool = SingleNodeConnectionPool::new(es_url);
            let transport: EsTransport = TransportBuilder::new(conn_pool)
                .timeout(Duration::new(5, 0))
                .build()?;

            let elastic_conn: Elasticsearch = Elasticsearch::new(transport);
            let es_client: EsClient = EsClient::new(url, elastic_conn);

            es_clients.push(es_client);
        }

        Ok(EsRepositoryPub { es_clients })
    }

    #[doc = "Common logic: common node failure handling and node selection"]
    async fn execute_on_any_node<F, Fut>(&self, operation: F) -> Result<Response, anyhow::Error>
    where
        F: Fn(EsClient) -> Fut + Send + Sync,
        Fut: Future<Output = Result<Response, anyhow::Error>> + Send,
    {
        let mut last_error: Option<anyhow::Error> = None;

        let mut rng: StdRng = StdRng::from_entropy();
        let mut shuffled_clients: Vec<EsClient> = self.es_clients.clone();
        shuffled_clients.shuffle(&mut rng);

        for es_client in shuffled_clients {
            match operation(es_client).await {
                Ok(response) => return Ok(response),
                Err(err) => {
                    last_error = Some(err);
                }
            }
        }

        Err(anyhow::anyhow!(
            "All Elasticsearch nodes failed. Last error: {:?}",
            last_error
        ))
    }
}

#[async_trait]
impl EsRepository for EsRepositoryPub {
    #[doc = "Function that processes responses after making a specific request to Elasticsearch.
    It processes functions that do not have a return value."]
    /// # Arguments
    /// * `function_name` - Name of the function that makes a specific request to Elasticsearch.
    /// * `response` - Query response json value.
    ///
    /// # Returns
    /// * Result<(), anyhow::Error>
    async fn process_response_empty(
        &self,
        function_name: &str,
        response: Response,
    ) -> Result<(), anyhow::Error> {
        if response.status_code().is_success() {
            Ok(())
        } else {
            let error_body: String = response.text().await?;
            Err(anyhow!(
                "[Elasticsearch Error][{}] response status is failed: {:?}",
                function_name,
                error_body
            ))
        }
    }

    #[doc = "Function that processes responses after making a specific request to Elasticsearch.
    It processes functions that have a return value."]
    /// # Arguments
    /// * `function_name` - Name of the function that makes a specific request to Elasticsearch.
    /// * `response` - Query response json value.
    ///
    /// # Returns
    /// * Result<Value, anyhow::Error>
    async fn process_response(
        &self,
        function_name: &str,
        response: Response,
    ) -> Result<Value, anyhow::Error> {
        if response.status_code().is_success() {
            let response_body = response.json::<Value>().await?;
            Ok(response_body)
        } else {
            let error_body = response.text().await?;
            Err(anyhow!(
                "[Elasticsearch Error][{}] response status is failed: {:?}",
                function_name,
                error_body
            ))
        }
    }  

    #[doc = "Function to index data to Elasticsearch at once"]
    /// # Arguments
    /// * `index_name` - index name
    /// * `data` - Data vectors to be indexed
    ///
    /// # Returns
    /// * Result<(), anyhow::Error>
    async fn bulk_indexing_query<T: Serialize + Send + Sync>(
        &self,
        index_name: &str,
        data: &[T],
    ) -> Result<(), anyhow::Error> {
        let response: Response = self
            .execute_on_any_node(|es_client| async move {
                let mut ops: Vec<BulkOperation<Value>> = Vec::with_capacity(data.len());

                for item in data {
                    /* Converting Data to JSON */
                    let json_value: Value = serde_json::to_value(item)?;

                    /* BulkOperation Generation (without ID) */
                    ops.push(BulkOperation::index(json_value).into());
                }

                let response: Response = es_client
                    .es_conn
                    .bulk(BulkParts::Index(index_name))
                    .body(ops)
                    .send()
                    .await?;

                Ok(response)
            })
            .await?;

        self.process_response_empty("bulk_query()", response).await
    }

    #[doc = "Function that EXECUTES elasticsearch queries - search"]
    async fn get_search_query(
        &self,
        es_query: &Value,
        index_name: &str,
    ) -> Result<Value, anyhow::Error> {
        let response = self
            .execute_on_any_node(|es_client| async move {
                let response = es_client
                    .es_conn
                    .search(SearchParts::Index(&[index_name]))
                    .body(es_query)
                    .send()
                    .await?;

                Ok(response)
            })
            .await?;

        if response.status_code().is_success() {
            let response_body: Value = response.json::<Value>().await?;
            Ok(response_body)
        } else {
            let error_body: String = response.text().await?;
            Err(anyhow!(
                "[Elasticsearch Error][node_search_query()] response status is failed: {:?}",
                error_body
            ))
        }
    }

    #[doc = "Function that first declares setting information and mapping information and then generates an index"]
    /// # Arguments
    /// * `index_name` - index name
    /// * `index_setting_json` - setting/mapping information of Index
    ///
    /// # Returns
    /// * Result<(), anyhow::Error>
    async fn create_index(
        &self,
        index_name: &str,
        index_setting_json: &Value,
    ) -> Result<(), anyhow::Error> {
        let response: Response = self
            .execute_on_any_node(|es_client| async move {
                let response = es_client
                    .es_conn
                    .indices()
                    .create(IndicesCreateParts::Index(index_name))
                    .body(index_setting_json)
                    .send()
                    .await?;

                Ok(response)
            })
            .await?;

        self.process_response_empty("create_index()", response)
            .await
    }

    #[doc = "Function that EXECUTES elasticsearch queries - indexing struct"]
    async fn post_query_struct<T: Serialize + Sync>(
        &self,
        param_struct: &T,
        index_name: &str,
    ) -> Result<(), anyhow::Error> {
        let struct_json: Value = convert_json_from_struct(param_struct)?;
        self.post_query(&struct_json, index_name).await?;

        Ok(())
    }

    #[doc = "Function that EXECUTES elasticsearch queries - indexing"]
    async fn post_query(&self, document: &Value, index_name: &str) -> Result<(), anyhow::Error> {
        let response = self
            .execute_on_any_node(|es_client| async move {
                let response = es_client
                    .es_conn
                    .index(IndexParts::Index(index_name))
                    .body(document)
                    .send()
                    .await?;

                Ok(response)
            })
            .await?;

        if response.status_code().is_success() {
            Ok(())
        } else {
            let error_message = format!("[Elasticsearch Error][node_post_query()] Failed to index document: Status Code: {}", response.status_code());
            Err(anyhow!(error_message))
        }
    }

    #[doc = "Functions that delete a particular index as a whole"]
    /// # Arguments
    /// * `index_name` - Index name to be deleted
    ///
    /// # Returns
    /// * Result<(), anyhow::Error>
    async fn delete_query(&self, index_name: &str) -> Result<(), anyhow::Error> {
        let response = self
            .execute_on_any_node(|es_client| async move {
                let response: Response = es_client
                    .es_conn
                    .indices()
                    .delete(IndicesDeleteParts::Index(&[index_name]))
                    .send()
                    .await?;

                Ok(response)
            })
            .await?;

        self.process_response_empty("delete_query()", response)
            .await
    }

    #[doc = "Functions that return the index name mapped to Elasticsearch alias"]
    /// # Arguments
    /// * `index_alias_name` - index alias name
    ///
    /// # Returns
    /// * Result<Value, anyhow::Error>
    async fn get_indexes_mapping_by_alias(
        &self,
        index_alias_name: &str,
    ) -> Result<Value, anyhow::Error> {
        let response: Response = self
            .execute_on_any_node(|es_client| async move {
                let response = es_client
                    .es_conn
                    .indices()
                    .get_alias(IndicesGetAliasParts::Name(&[index_alias_name]))
                    .send()
                    .await?;

                Ok(response)
            })
            .await?;

        self.process_response("get_indexes_mapping_by_alias()", response)
            .await
    }

    #[doc = "Functions that change the index specified for a particular alias"]
    /// # Arguments
    /// * `index_alias` - index alias name
    /// * `new_index_name` - Index name to be newly mapped to alias
    /// * `old_index_name` - Index name mapped to alias
    ///
    /// # Returns
    /// * Result<(), anyhow::Error>
    async fn update_index_alias(
        &self,
        index_alias: &str,
        new_index_name: &str,
        old_index_name: &str,
    ) -> Result<(), anyhow::Error> {
        let response: Response = self
            .execute_on_any_node(|es_client| async move {
                let actions = json!({
                    "actions": [
                        { "remove": { "index": old_index_name, "alias": index_alias } },
                        { "add": { "index": new_index_name, "alias": index_alias } }
                    ]
                });

                let update_response = es_client
                    .es_conn
                    .indices()
                    .update_aliases()
                    .body(actions)
                    .send()
                    .await?;

                Ok(update_response)
            })
            .await?;

        self.process_response_empty("update_index_alias()", response)
            .await
    }

    #[doc = "Functions that refresh a particular index to enable immediate search"]
    /// # Arguments
    /// * `index_name` - Index name to be refresh
    ///
    /// # Returns
    /// * Result<(), anyhow::Error>
    async fn refresh_index(&self, index_name: &str) -> Result<(), anyhow::Error> {
        let response: Response = self
            .execute_on_any_node(|es_client| async move {
                let response: Response = es_client
                    .es_conn
                    .indices()
                    .refresh(IndicesRefreshParts::Index(&[index_name]))
                    .send()
                    .await?;

                Ok(response)
            })
            .await?;

        self.process_response_empty("delete_query_doc()", response)
            .await
    }

    #[doc = "Functions that create an alias for a particular index"]
    /// # Arguments
    /// * `index_alias` - index alias name
    /// * `index_name` - Index name to be newly mapped to alias
    ///
    /// # Returns
    /// * Result<(), anyhow::Error>
    async fn create_index_alias(
        &self,
        index_alias: &str,
        index_name: &str,
    ) -> Result<(), anyhow::Error> {
        
        let response: Response = self
            .execute_on_any_node(|es_client| async move {
                let actions = json!({
                    "actions": [
                        { "add": { "index": index_name, "alias": index_alias } }
                    ]
                });

                let create_response = es_client
                    .es_conn
                    .indices()
                    .update_aliases()
                    .body(actions)
                    .send()
                    .await?;

                Ok(create_response)
            })
            .await?;
        
        self.process_response_empty("create_index_alias()", response)
            .await
    }
}

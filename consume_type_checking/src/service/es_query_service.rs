use crate::common::*;

use crate::repository::es_repository::*;

use crate::utils_module::time_utils::*;
use crate::utils_module::io_utils::*;

use crate::config::config_settings::*;

use crate::model::{
    consume_prodt_keyword::*,
    document_with_id::*
};

#[async_trait]
pub trait EsQueryService {
    async fn get_query_result_vec<T: DeserializeOwned + Debug>(
        &self,
        response_body: &Value,
    ) -> Result<Vec<DocumentWithId<T>>, anyhow::Error>;
    async fn post_indexing_data_by_bulk<T: Serialize + Send + Sync + Debug>(
        &self,
        index_alias_name: &str,
        index_settings_path: &str,
        data: &[T],
    ) -> Result<(), anyhow::Error>;
    async fn find_consume_type(&self, consume_name: &str) -> Result<Vec<DocumentWithId<ConsumeProdtKeyword>>, anyhow::Error>;
}

#[derive(Debug, new)]
pub struct EsQueryServicePub;


#[async_trait]
impl EsQueryService for EsQueryServicePub {
     #[doc = "Functions that return queried results as vectors"]
    /// # Arguments
    /// * `response_body` - Querying Results
    ///
    /// # Returns
    /// * Result<Vec<T>, anyhow::Error>
    async fn get_query_result_vec<T: DeserializeOwned + Debug>(
        &self,
        response_body: &Value,
    ) -> Result<Vec<DocumentWithId<T>>, anyhow::Error> {
        let hits: &Value = &response_body["hits"]["hits"];

        let results: Vec<DocumentWithId<T>> = hits
            .as_array()
            .ok_or_else(|| anyhow!("[Error][get_query_result_vec()] 'hits' field is not an array"))?
            .iter()
            .map(|hit| {
                let id: &str = hit.get("_id").and_then(|id| id.as_str()).ok_or_else(|| {
                    anyhow!("[Error][get_query_result_vec()] Missing '_id' field")
                })?;

                let score: f64 = hit
                    .get("_score")
                    .and_then(|score| score.as_f64())
                    .ok_or_else(|| {
                        anyhow!("[Error][get_query_result_vec()] Missing '_score' field")
                    })?;

                let source: &Value = hit.get("_source").ok_or_else(|| {
                    anyhow!("[Error][get_query_result_vec()] Missing '_source' field")
                })?;

                let source: T = serde_json::from_value(source.clone()).map_err(|e| {
                    anyhow!(
                        "[Error][get_query_result_vec()] Failed to deserialize source: {:?}",
                        e
                    )
                })?;

                Ok::<DocumentWithId<T>, anyhow::Error>(DocumentWithId {
                    id: id.to_string(),
                    score,
                    source,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    #[doc = "static index function"]
    /// # Arguments
    /// * `index_alias_name` - alias for index
    /// * `index_settings_path` - File path for setting index schema
    /// * `data` - Vector information to be indexed
    ///
    /// # Returns
    /// * Result<(), anyhow::Error>
    async fn post_indexing_data_by_bulk<T: Serialize + Send + Sync + Debug>(
        &self,
        index_alias_name: &str,
        index_settings_path: &str,
        data: &[T],
    ) -> Result<(), anyhow::Error> {
        let es_conn: ElasticConnGuard = get_elastic_guard_conn().await?;

        /* Put today's date time on the index you want to create. */
        let curr_time: String = get_current_kor_naive_datetime()
            .format("%Y%m%d%H%M%S")
            .to_string();
        let new_index_name: String = format!("{index_alias_name}-{curr_time}");

        let json_body: Value = read_json_from_file(index_settings_path)?;
        es_conn.create_index(&new_index_name, &json_body).await?;

        /* Bulk post the data to the index above at once. */
        es_conn.bulk_indexing_query(&new_index_name, data).await?;

        /* Change alias or Create alias */
        match es_conn.get_indexes_mapping_by_alias(index_alias_name).await {
            Ok(alias_resp) => {
                let old_index_name: String;
                if let Some(first_key) = alias_resp.as_object().and_then(|map| map.keys().next()) {
                    old_index_name = first_key.to_string();
                } else {
                    return Err(anyhow!("[Error][post_indexing_data_by_bulk()] Failed to extract index name within 'index-alias'"));
                }

                es_conn
                    .update_index_alias(index_alias_name, &new_index_name, &old_index_name)
                    .await?;
                es_conn.delete_query(&old_index_name).await?;

                /* Functions to enable search immediately after index */
                es_conn.refresh_index(index_alias_name).await?;
            }
            Err(e) => {
                /* This alias does not exist - It just creates an index. */
                error!("[Error][post_indexing_data_by_bulk()] Failed to get index mapping by alias: {e:?}, Create a new index.");
                es_conn
                    .create_index_alias(index_alias_name, &new_index_name)
                    .await?;
                info!("Index generation was successful.- {index_alias_name}");
            }
        };

        Ok(())
    }


    #[doc = "Functions that query consume_keyword"]
    /// # Arguments
    /// * `consume_name` - Name of consumption to be classified
    ///
    /// # Returns
    /// * Result<String, anyhow::Error>
    async fn find_consume_type(&self, consume_name: &str) -> Result<Vec<DocumentWithId<ConsumeProdtKeyword>>, anyhow::Error> {
        let es_query: Value = json!({
                "query": {
                    "match": {
                        "consume_keyword": consume_name
                    }
                }
            });
        
        let es_conn: ElasticConnGuard = get_elastic_guard_conn().await?;
        let search_res_body: Value = es_conn.get_search_query(&es_query, &CONSUME_TYPE).await?;

        let results: Vec<DocumentWithId<ConsumeProdtKeyword>> =
                self.get_query_result_vec(&search_res_body).await?;
        
        Ok(results)
    }
}
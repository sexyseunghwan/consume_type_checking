use crate::common::*;

use crate::repository::mysql_repository::*;

use crate::entity::consume_prodt_keyword;

use crate::model::consume_prodt_keyword::*;


#[async_trait]
pub trait QueryService {
    async fn get_all_consume_prodt_type(
        &self,
        batch_size: usize,
    ) -> Result<Vec<ConsumeProdtKeyword>, anyhow::Error>;
}


#[derive(Debug, new)]
pub struct QueryServicePub;

#[async_trait]
impl QueryService for QueryServicePub {
    #[doc = "Functions that select all objects in the 'ConsumeProdtKeyword' table"]
    /// # Arguments
    /// * `batch_size` - batch size
    ///
    /// # Returns
    /// * Result<Vec<T>, anyhow::Error>
    async fn get_all_consume_prodt_type(
        &self,
        batch_size: usize,
    ) -> Result<Vec<ConsumeProdtKeyword>, anyhow::Error> {
        let db: &DatabaseConnection = establish_connection().await;

        let mut total_consume_prodt_keyword: Vec<ConsumeProdtKeyword> = Vec::new();
        let mut last_keyword_type: Option<String> = None;
        let mut last_keyword: Option<String> = None;

        loop {
            let mut query: Select<consume_prodt_keyword::Entity> =
                consume_prodt_keyword::Entity::find()
                    .order_by_asc(consume_prodt_keyword::Column::ConsumeKeywordType)
                    .order_by_asc(consume_prodt_keyword::Column::ConsumeKeyword)
                    .limit(batch_size as u64)
                    .select_only()
                    .columns([
                        consume_prodt_keyword::Column::ConsumeKeywordType,
                        consume_prodt_keyword::Column::ConsumeKeyword,
                    ]);

            if let (Some(last_type), Some(last_keyword_val)) = (&last_keyword_type, &last_keyword) {
                query = query.filter(
                    Condition::any()
                        .add(
                            consume_prodt_keyword::Column::ConsumeKeywordType.gt(last_type.clone()),
                        )
                        .add(
                            Condition::all()
                                .add(
                                    consume_prodt_keyword::Column::ConsumeKeywordType
                                        .eq(last_type.clone()),
                                )
                                .add(
                                    consume_prodt_keyword::Column::ConsumeKeyword
                                        .gt(last_keyword_val.clone()),
                                ),
                        ),
                );
            }

            let mut batch_data: Vec<ConsumeProdtKeyword> = query
                .into_model()
                .all(db)
                .await
                .map_err(|e| anyhow!("[Error][get_all_consume_prodt_type()] {e:?}"))?;

            if batch_data.is_empty() {
                break;
            }

            if let Some(last) = batch_data.last() {
                last_keyword_type = Some(last.consume_keyword_type.clone());
                last_keyword = Some(last.consume_keyword.clone());
            }

            total_consume_prodt_keyword.append(&mut batch_data);
        }

        Ok(total_consume_prodt_keyword)
    }
}
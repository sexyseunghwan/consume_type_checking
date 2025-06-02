use crate::common::*;

#[doc = "Structures to map to the `CONSUME_PRODT_KEYWORD` table"]
#[derive(Debug, FromQueryResult, Serialize, Deserialize, Getters)]
#[getset(get = "pub")]
pub struct ConsumeProdtKeyword {
    pub consume_keyword_type: String,
    pub consume_keyword: String,
}
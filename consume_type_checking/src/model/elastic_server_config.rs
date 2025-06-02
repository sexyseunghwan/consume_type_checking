use crate::common::*;

// 필요없을 거 같은ㄷ[ㅔ...]
#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct ElasticServerConfig {
    pub elastic_host: Vec<String>,
    pub elastic_id: Option<String>,
    pub elastic_pw: Option<String>,
    pub elastic_pool_cnt: i32,
    pub elastic_cluster_name: String,
}

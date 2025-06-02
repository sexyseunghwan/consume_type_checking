use crate::common::*;

use crate::service::es_query_service::*;
use crate::service::query_service::*;

use crate::model::
{
    consume_prodt_keyword::*,
    document_with_id::*,
    score_manager::*
};

use crate::config::config_settings::*;

#[derive(Debug, new)]
pub struct MainController<Q: QueryService, E: EsQueryService> {
    query_service: Q,
    es_query_service: E,
}

impl<Q: QueryService, E: EsQueryService> MainController<Q, E> {
    
    #[doc = "Main Batch Function"]
    pub async fn main_task(&self) -> Result<(), anyhow::Error> {
        
        println!("[=======================================================]");
        println!("[================ CONSUME_TYPE_CHECKING ================]");
        println!("[=======================================================]");
        
        loop {
            println!("Please write down the words you want to classify.");
            print!("keyword: ");
            io::stdout().flush().unwrap();
            
            let mut input: String = String::new();

            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");
            
            let spinner: tokio::task::JoinHandle<()> = tokio::spawn(async {

                    for _ in 0.. {
                        print!(". ");
                        io::stdout().flush().unwrap();
                        sleep(Duration::from_millis(300)).await;
                    }
            });

            let type_res: String = self.classify_keyword_by_rules(&input).await?;
            
            spinner.abort();
            println!();
            println!("type: {type_res}");
            
            println!("Press Enter to continue (or type 'q' to quit): ");
            print!("> ");
            io::stdout().flush().unwrap();

            let mut cont_input = String::new();
            io::stdin().read_line(&mut cont_input).unwrap();

            if cont_input.trim().eq_ignore_ascii_case("q") {
                println!("Exit the program.");
                break;
            }    

            for _i in 1..=100 {
                println!();    
            }
        }

        Ok(())
    }

    #[doc = "Function that categorizes keywords according to rules"]
    /// # Arguments
    /// * `consume_name` - Name of consumption
    ///
    /// # Returns
    /// * Result<String, anyhow::Error>
    async fn classify_keyword_by_rules(&self, consume_name: &str) -> Result<String, anyhow::Error> {
        
        /* 1. consuming_index_prod_type */
        let consume_prodt_type: Vec<ConsumeProdtKeyword> = self
            .query_service
            .get_all_consume_prodt_type(1000)
            .await?;

        self.es_query_service
            .post_indexing_data_by_bulk::<ConsumeProdtKeyword>(
                &CONSUME_TYPE,
                &CONSUME_TYPE_SETTINGS,
                &consume_prodt_type,
            )
            .await?;

        let type_result: String = self.analyze_keyword(consume_name).await?;
        
        Ok(type_result)
    }
    
    #[doc = "Function that analyzes which consumption type fits the keyword"]
    /// # Arguments
    /// * `consume_name` - Name of consumption
    ///
    /// # Returns
    /// * Result<String, anyhow::Error>
    async fn analyze_keyword(&self, consume_name: &str) -> Result<String, anyhow::Error> {

        let types: Vec<DocumentWithId<ConsumeProdtKeyword>>
            = self.es_query_service.find_consume_type(consume_name).await?;

        if types.is_empty() {
            Ok(String::from("etc"))
        } else {
            let mut manager: ScoreManager<ConsumeProdtKeyword> = 
                ScoreManager::<ConsumeProdtKeyword>::new();

            for consume_type in types {
                let keyword: &String = consume_type.source.consume_keyword();
                let score: i64 = consume_type.score as i64;
                let score_i64: i64 = score * -1;

                /* Use the 'levenshtein' algorithm to determine word match */
                let word_dist: usize = levenshtein(keyword, consume_name);
                let word_dist_i64: i64 = word_dist.try_into()?;
                manager.insert(word_dist_i64 + score_i64, consume_type.source);
            }
            
            let score_data_keyword: ScoredData<ConsumeProdtKeyword> = match manager.pop_lowest() {
                Some(score_data_keyword) => score_data_keyword,
                None => {
                    error!("[Error][MainController->score_keyword] The mapped data for variable 'score_data_keyword' does not exist.");
                    panic!("[Error][MainController->score_keyword] The mapped data for variable 'score_data_keyword' does not exist.")
                }
            };
            
            Ok(score_data_keyword.data().consume_keyword_type().to_string())
        }
    }

    

}
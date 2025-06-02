use crate::common::*;


#[doc = "Function to globally initialize the 'ES_DB_URL' variable"]
pub static ES_DB_URL: once_lazy<String> = once_lazy::new(|| {
    env::var("ES_DB_URL")
        .expect("[ENV file read Error] 'ES_DB_URL' must be set")
});

#[doc = "Function to globally initialize the 'ES_ID' variable"]
pub static ES_ID: once_lazy<String> = once_lazy::new(|| {
    env::var("ES_ID")
        .expect("[ENV file read Error] 'ES_ID' must be set")
});

#[doc = "Function to globally initialize the 'ES_PW' variable"]
pub static ES_PW: once_lazy<String> = once_lazy::new(|| {
    env::var("ES_PW")
        .expect("[ENV file read Error] 'ES_PW' must be set")
});

#[doc = "Function to globally initialize the 'ES_POOL_SIZE' variable"]
pub static ES_POOL_SIZE: once_lazy<String> = once_lazy::new(|| {
    env::var("ES_POOL_SIZE")
        .expect("[ENV file read Error] 'ES_POOL_SIZE' must be set")
});

#[doc = "Function to globally initialize the 'MY_SQL_HOST' variable"]
pub static MY_SQL_HOST: once_lazy<String> = once_lazy::new(|| {
    env::var("MY_SQL_HOST")
        .expect("[ENV file read Error] 'MY_SQL_HOST' must be set")
});

#[doc = "Function to globally initialize the 'DATABASE_URL' variable"]
pub static DATABASE_URL: once_lazy<String> = once_lazy::new(|| {
    env::var("DATABASE_URL")
        .expect("[ENV file read Error] 'DATABASE_URL' must be set")
});

#[doc = "Function to globally initialize the 'BATCH_SIZE' variable"]
pub static BATCH_SIZE: once_lazy<String> = once_lazy::new(|| {
    env::var("BATCH_SIZE")
        .expect("[ENV file read Error] 'BATCH_SIZE' must be set")
});


#[doc = "Function to globally initialize the 'CONSUME_DETAIL' variable"]
pub static CONSUME_DETAIL: once_lazy<String> = once_lazy::new(|| {
    env::var("CONSUME_DETAIL").expect("[ENV file read Error] 'CONSUME_DETAIL' must be set")
});

#[doc = "Function to globally initialize the 'CONSUME_TYPE' variable"]
pub static CONSUME_TYPE: once_lazy<String> = once_lazy::new(|| {
    env::var("CONSUME_TYPE").expect("[ENV file read Error] 'CONSUME_TYPE' must be set")
});

#[doc = "Function to globally initialize the 'CONSUME_TYPE_SETTINGS' variable"]
pub static CONSUME_TYPE_SETTINGS: once_lazy<String> = once_lazy::new(|| {
    env::var("CONSUME_TYPE_SETTINGS")
        .expect("[ENV file read Error] 'CONSUME_TYPE_SETTINGS' must be set")
});

#[doc = "Function to globally initialize the 'CONSUME_DETAIL_SETTINGS' variable"]
pub static CONSUME_DETAIL_SETTINGS: once_lazy<String> = once_lazy::new(|| {
    env::var("CONSUME_DETAIL_SETTINGS")
        .expect("[ENV file read Error] 'CONSUME_DETAIL_SETTINGS' must be set")
});
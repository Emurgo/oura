use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub enum Naming {
    Hash,
    SlotHash,
    BlockHash,
    BlockNumber,
    EpochHash,
    EpochSlotHash,
    EpochBlockHash,
}

#[derive(Deserialize, Debug, Clone)]
pub enum ContentType {
    Cbor,
    CborHex,
    Json,
}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct Config {
    pub s3_region: String,
    pub s3_bucket: String,
    pub s3_prefix: Option<String>,
    pub s3_naming: Option<Naming>,
    pub s3_content: Option<ContentType>,
    pub s3_max_retries: Option<u32>,

    pub sqs_region: String,
    pub sqs_queue_url: String,
    pub sqs_fifo: Option<bool>,
    pub sqs_group_id: Option<String>,
    pub sqs_max_retries: Option<u32>,
}

use super::Config;
use crate::model::BlockRecord;
use crate::sinks::aws_s3_sqs::{ContentType, Naming};
use crate::Error;
use aws_sdk_s3::types::ByteStream as S3ByteStream;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::Region as S3Region;
use aws_sdk_s3::RetryConfig as S3RetryConfig;
use aws_sdk_sqs::Client as SqsClient;
use aws_sdk_sqs::Region as SqsRegion;
use aws_sdk_sqs::RetryConfig as SqsRetryConfig;
use serde::{Deserialize, Serialize};
use serde_json::json;

const DEFAULT_MAX_RETRIES: u32 = 5;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct SqsMessage {
    s3_key: String,
    block_hash: String,
    previous_hash: String,
    block_number: u64,
    slot: u64,
    tip: Option<i64>,
}

impl From<&ContentType> for String {
    fn from(other: &ContentType) -> Self {
        match other {
            ContentType::Cbor => "application/cbor".to_string(),
            ContentType::CborHex => "text/plain".to_string(),
            ContentType::Json => "application/json".to_string(),
        }
    }
}

pub(super) struct CombinedClient {
    s3: S3Client,
    sqs: SqsClient,
    config: Config,
    naming: Naming,
    content_type: ContentType,
    sqs_group_id: String,
    s3_prefix: String,
}

impl CombinedClient {
    pub fn new(config: &Config) -> Result<CombinedClient, Error> {
        let s3 = setup_s3_client(config)?;
        let sqs = setup_sqs_client(config)?;
        let naming = config.s3_naming.clone().unwrap_or(Naming::Hash);
        let content_type = config.s3_content.clone().unwrap_or(ContentType::Cbor);
        let group_id = config
            .sqs_group_id
            .clone()
            .unwrap_or_else(|| "oura-sink".to_string());
        let s3_prefix = config.s3_prefix.clone().unwrap_or_default();
        Ok(CombinedClient {
            s3,
            sqs,
            config: config.clone(),
            naming,
            content_type,
            sqs_group_id: group_id,
            s3_prefix,
        })
    }

    pub async fn send_block(
        self: &Self,
        record: &BlockRecord,
        tip: Option<i64>,
    ) -> Result<(), Error> {
        let key = self.get_s3_key(record);
        self.send_s3_object(&key, record).await?;
        self.send_sqs_message(&key, record, tip).await?;
        Ok(())
    }

    async fn send_s3_object(self: &Self, key: &str, record: &BlockRecord) -> Result<(), Error> {
        let content_type: String = String::from(&self.content_type);
        let content = encode_block(&self.content_type, record);
        let req = self
            .s3
            .put_object()
            .bucket(&self.config.s3_bucket)
            .key(key)
            .body(content)
            .metadata("era", record.era.to_string())
            .metadata("issuer_vkey", &record.issuer_vkey)
            .metadata("tx_count", record.tx_count.to_string())
            .metadata("slot", record.slot.to_string())
            .metadata("hash", &record.hash)
            .metadata("number", record.number.to_string())
            .metadata("previous_hash", &record.previous_hash)
            .content_type(content_type);

        let res = req.send().await?;

        log::trace!("S3 put response: {:?}", res);

        Ok(())
    }

    async fn send_sqs_message(
        self: &Self,
        key: &str,
        record: &BlockRecord,
        tip: Option<i64>,
    ) -> Result<(), Error> {
        let message = SqsMessage {
            s3_key: key.to_string(),
            block_hash: record.hash.to_string(),
            previous_hash: record.previous_hash.to_string(),
            block_number: record.number,
            slot: record.slot,
            tip: tip,
        };

        let body = json!(message).to_string();

        let mut req = self
            .sqs
            .send_message()
            .queue_url(&self.config.sqs_queue_url)
            .message_body(body);

        if self.config.sqs_fifo.unwrap_or_default() {
            req = req
                .message_group_id(&self.sqs_group_id)
                .message_deduplication_id(key);
        }

        let res = req.send().await?;

        log::trace!("SQS send response: {:?}", res);

        Ok(())
    }

    fn get_s3_key(&self, record: &BlockRecord) -> String {
        define_obj_key(&self.s3_prefix, &self.naming, record)
    }
}

fn encode_block(content_type: &ContentType, record: &BlockRecord) -> S3ByteStream {
    let hex = match record.cbor_hex.as_ref() {
        Some(x) => x,
        None => {
            log::error!(
                "found block record without CBOR, please enable CBOR in source mapper options"
            );
            panic!()
        }
    };

    match content_type {
        ContentType::Cbor => {
            let cbor = hex::decode(hex).expect("valid hex value");
            S3ByteStream::from(cbor)
        }
        ContentType::CborHex => S3ByteStream::from(hex.as_bytes().to_vec()),
        ContentType::Json => {
            let json = json!(record).to_string().as_bytes().to_vec();
            S3ByteStream::from(json)
        }
    }
}

fn setup_s3_client(config: &Config) -> Result<S3Client, Error> {
    let explicit_region = config.s3_region.to_owned();

    let aws_config = tokio::runtime::Builder::new_current_thread()
        .build()?
        .block_on(
            aws_config::from_env()
                .region(S3Region::new(explicit_region))
                .load(),
        );

    let retry_config = S3RetryConfig::new()
        .with_max_attempts(config.s3_max_retries.unwrap_or(DEFAULT_MAX_RETRIES));

    let s3_config = aws_sdk_s3::config::Builder::from(&aws_config)
        .retry_config(retry_config)
        .build();

    Ok(S3Client::from_conf(s3_config))
}

fn setup_sqs_client(config: &Config) -> Result<SqsClient, Error> {
    let explicit_region = config.sqs_region.to_owned();

    let aws_config = tokio::runtime::Builder::new_current_thread()
        .build()?
        .block_on(
            aws_config::from_env()
                .region(SqsRegion::new(explicit_region))
                .load(),
        );

    let retry_config = SqsRetryConfig::new()
        .with_max_attempts(config.sqs_max_retries.unwrap_or(DEFAULT_MAX_RETRIES));

    let sqs_config = aws_sdk_sqs::config::Builder::from(&aws_config)
        .retry_config(retry_config)
        .build();

    Ok(SqsClient::from_conf(sqs_config))
}

fn define_obj_key(prefix: &str, policy: &Naming, record: &BlockRecord) -> String {
    match policy {
        Naming::Hash => format!("{}{}", prefix, record.hash),
        Naming::SlotHash => format!("{}{}.{}", prefix, record.slot, record.hash),
        Naming::BlockHash => format!("{}{}.{}", prefix, record.number, record.hash),
        Naming::BlockNumber => format!("{}", record.number),
        Naming::EpochHash => format!(
            "{}{}.{}",
            prefix,
            record.epoch.unwrap_or_default(),
            record.hash
        ),
        Naming::EpochSlotHash => format!(
            "{}{}.{}.{}",
            prefix,
            record.epoch.unwrap_or_default(),
            record.slot,
            record.hash
        ),
        Naming::EpochBlockHash => {
            format!(
                "{}{}.{}.{}",
                prefix,
                record.epoch.unwrap_or_default(),
                record.number,
                record.hash
            )
        }
    }
}

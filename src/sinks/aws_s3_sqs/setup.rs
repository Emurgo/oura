use super::config::Config;
use crate::sinks::aws_s3_sqs::combined_client::CombinedClient;
use crate::{
    pipelining::{BootstrapResult, SinkProvider, StageReceiver},
    utils::WithUtils,
};

use super::run::writer_loop;

impl SinkProvider for WithUtils<Config> {
    fn bootstrap(&self, input: StageReceiver) -> BootstrapResult {
        let client = CombinedClient::new(&self.inner)?;
        let utils = self.utils.clone();

        let handle = std::thread::spawn(move || {
            writer_loop(input, client, utils).expect("writer loop failed")
        });

        Ok(handle)
    }
}

use std::sync::Arc;

use crate::{
    model::{EventData},
    pipelining::StageReceiver,
    utils::Utils,
    Error,
};
use crate::sinks::aws_s3_sqs::combined_client::CombinedClient;


pub(super) fn writer_loop(
    input: StageReceiver,
    client: CombinedClient,
    utils: Arc<Utils>,
) -> Result<(), Error> {
    let client = Arc::new(client);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()?;

    for event in input.iter() {
        if let EventData::Block(record) = &event.data {

            let client = client.clone();
            let tip = utils.metrics.as_ref().map(
                |metrics| metrics.chain_tip.get(),
            );

            let result = rt.block_on(async move {
                client.send_block(record, tip).await
            });

            match result {
                Ok(_) => {
                    // notify the pipeline where we are
                    utils.track_sink_progress(&event);
                }
                Err(err) => {
                    log::error!("unrecoverable error sending block to S3 and SQS: {:?}", err);
                    return Err(err);
                }
            }
        }
    }

    Ok(())
}

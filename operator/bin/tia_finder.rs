use core::time::Duration;
use serde_json;
use std::{fs::File, io::Write};
use tendermint_light_client_verifier::{
    options::Options, types::LightBlock, ProdVerifier, Verdict, Verifier,
};
use tendermint_operator::util::TendermintRPCClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("creating rpc client");
    let client = TendermintRPCClient::default();
    println!("getting latest block height...");
    let latest_height = client.get_latest_block_height().await;
    let mut height_to_try = latest_height;
    println!("latest block height = {}", &height_to_try);

    let peer_id = client.fetch_peer_id().await.unwrap();
    let genesis = client.fetch_light_block(1, peer_id).await.unwrap();
    let mut starting_block = genesis;

    let mut keep_trying = true;

    while keep_trying {
        let block_to_try = client
            .fetch_light_block(height_to_try, peer_id)
            .await
            .unwrap();

        let vp = ProdVerifier::default();
        let opt = Options {
            trust_threshold: Default::default(),
            // 2 week trusting period.
            trusting_period: Duration::from_secs(14 * 24 * 60 * 60),
            clock_drift: Default::default(),
        };
        let verify_time = block_to_try.time() + Duration::from_secs(20);
        println!("trying verification...");
        let verdict = vp.verify_update_header(
            block_to_try.as_untrusted_state(),
            starting_block.as_trusted_state(),
            &opt,
            verify_time.unwrap(),
        );
        println!("verification done");

        match verdict {
            Verdict::Success => {
                println!("height {} looks good", height_to_try);

                let json_filename = format!("{}.json", block_to_try.height().value());
                let json_data = serde_json::to_string(&block_to_try).unwrap();
                let mut file = File::create(json_filename).unwrap();
                file.write_all(json_data.as_bytes()).unwrap();

                starting_block = block_to_try;
                height_to_try = height_to_try + ((latest_height - height_to_try) / 2);
            }
            _ => {
                println!("height {} didn't work", height_to_try);
                height_to_try = starting_block.height().value()
                    + ((height_to_try - starting_block.height().value()) / 2);
            }
        }
        if (latest_height - height_to_try) < 100 {
            println!("i think we're done");
            keep_trying = false;
        }
    }

    Ok(())
}

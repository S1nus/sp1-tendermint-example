// TODO: import from sp1_sdk when these are public in the future
use crate::util::TendermintRPCClient;
use sp1_sdk::{ProverClient, SP1Groth16Proof, SP1ProvingKey, SP1Stdin, SP1VerifyingKey};
use tendermint_light_client_verifier::types::LightBlock;

pub mod client;
mod types;
pub mod util;

// The path to the ELF file for the Succinct zkVM program.
pub const TENDERMINT_ELF: &[u8] = include_bytes!("../../program/elf/riscv32im-succinct-zkvm-elf");

pub struct TendermintProver {
    prover_client: ProverClient,
    pkey: SP1ProvingKey,
    vkey: SP1VerifyingKey,
}

impl Default for TendermintProver {
    fn default() -> Self {
        Self::new()
    }
}

impl TendermintProver {
    pub fn new() -> Self {
        log::info!("Initializing SP1 ProverClient...");
        let prover_client = ProverClient::new();
        let (pkey, vkey) = prover_client.setup(TENDERMINT_ELF);
        log::info!("SP1 ProverClient initialized");
        Self {
            prover_client,
            pkey,
            vkey,
        }
    }

    /// Using the trusted_header_hash, fetch the latest block and generate a proof for that.
    pub async fn generate_header_update_proof_to_latest_block(
        &self,
        trusted_header_hash: &[u8],
    ) -> anyhow::Result<SP1Groth16Proof> {
        let tendermint_client = TendermintRPCClient::default();
        let latest_block_height = tendermint_client.get_latest_block_height().await;
        // Get the block height corresponding to the trusted header hash.
        let trusted_block_height = tendermint_client
            .get_block_height_from_hash(trusted_header_hash)
            .await;
        self.generate_header_update_proof_between_blocks(trusted_block_height, latest_block_height)
            .await
    }

    /// Given a trusted block height and a target block height, generate a proof of the update.
    pub async fn generate_header_update_proof_between_blocks(
        &self,
        trusted_block_height: u64,
        target_block_height: u64,
    ) -> anyhow::Result<SP1Groth16Proof> {
        log::info!(
            "Generating proof for blocks {} to {}",
            trusted_block_height,
            target_block_height
        );
        let tendermint_client = TendermintRPCClient::default();
        let (trusted_light_block, target_light_block) = tendermint_client
            .get_light_blocks(trusted_block_height, target_block_height)
            .await;
        let proof_data = self
            .generate_header_update_proof(&trusted_light_block, &target_light_block)
            .await;
        Ok(proof_data)
    }

    /// Generate a proof of an update from trusted_light_block to target_light_block. Returns the
    /// public values and proof of the update.
    pub async fn generate_header_update_proof(
        &self,
        trusted_light_block: &LightBlock,
        target_light_block: &LightBlock,
    ) -> SP1Groth16Proof {
        // Encode the light blocks to be input into our program.
        let encoded_1 = serde_cbor::to_vec(&trusted_light_block).unwrap();
        let encoded_2 = serde_cbor::to_vec(&target_light_block).unwrap();

        // Write the encoded light blocks to stdin.
        let mut stdin = SP1Stdin::new();
        stdin.write_vec(encoded_1);
        stdin.write_vec(encoded_2);

        // Generate proof.
        self.generate_proof(stdin).await
    }

    // This function is used to generate a proof. Implementations of this trait can implement
    // this method with either a local or network prover.
    async fn generate_proof(&self, stdin: SP1Stdin) -> SP1Groth16Proof {
        // Generate the proof. Depending on SP1_PROVER env, this may be a local or network proof.
        let proof = self
            .prover_client
            .prove_groth16(&self.pkey, stdin)
            .expect("proving failed");
        println!("Successfully generated proof!");

        // Verify proof.
        self.prover_client
            .verify_groth16(&proof, &self.vkey)
            .expect("Verification failed");

        // Return the proof.
        proof
    }
}

/// Utility method for converting u32 words to bytes in little endian.
pub fn words_to_bytes_be(words: &[u32; 8]) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for i in 0..8 {
        let word_bytes = words[i].to_be_bytes();
        bytes[i * 4..(i + 1) * 4].copy_from_slice(&word_bytes);
    }
    bytes
}
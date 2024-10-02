use core::time::Duration;
use serde_json;
use std::{fs, path::Path};
use tendermint_light_client_verifier::{
    options::Options, types::LightBlock, ProdVerifier, Verdict, Verifier,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir = fs::read_dir(".")?;
    let mut files = vec![];
    for entry in dir {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().unwrap_or_default() == "json" {
            let filename = path
                .file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();
            files.push(filename.to_string());
        }
    }
    files.sort_by(|a, b| a.parse::<u32>().unwrap().cmp(&b.parse::<u32>().unwrap()));
    let mut dels: Vec<String> = vec![];
    for i in 0..files.len() {
        let file = fs::File::open(format!("{}.json", files[i])).unwrap();
        let reader = std::io::BufReader::new(file);
        let first_block: LightBlock = serde_json::from_reader(reader).unwrap();
        let mut closest_reject_height: Option<u64> = None;
        let mut num_dels = 0;
        for j in i + 1..files.len() {
            let file2 = fs::File::open(format!("{}.json", files[j])).unwrap();
            let reader2 = std::io::BufReader::new(file2);
            let second_block: LightBlock = serde_json::from_reader(reader2).unwrap();
            let vp = ProdVerifier::default();
            let opt = Options {
                trust_threshold: Default::default(),
                // 2 week trusting period.
                trusting_period: Duration::from_secs(14 * 24 * 60 * 60),
                clock_drift: Default::default(),
            };
            let verify_time = second_block.time() + Duration::from_secs(20);
            let verdict = vp.verify_update_header(
                second_block.as_untrusted_state(),
                first_block.as_trusted_state(),
                &opt,
                verify_time.unwrap(),
            );
            match verdict {
                Verdict::Success => {
                    num_dels += 1;
                    dels.push(format!("{}.json", files[j]));
                }
                _ => {
                    closest_reject_height = Some(second_block.height().value());
                    /*println!(
                        "closest reject height for {} is {}",
                        first_block.height(),
                        second_block.height()
                    );*/
                    num_dels -= 1;
                    dels.pop();
                    break;
                }
            }
        }
    }
    println!("files to delete:");
    for f in dels {
        print!("{} ", f);
    }
    /*for (i, f) in files.iter().enumerate() {
        let file = fs::File::open(format!("{}.json", f)).unwrap();
        let reader = std::io::BufReader::new(file);
        let deserialized: LightBlock = serde_json::from_reader(reader).unwrap();
        for (j, f2) in files.
        }
    }*/
    Ok(())
}

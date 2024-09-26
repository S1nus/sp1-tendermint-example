use std::{fs, path::Path};

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
    files.sort_by(|a, b| a.parse::<u32>().cmp(&b.parse::<u32>()));
    println!("{:?}", files);
    Ok(())
}

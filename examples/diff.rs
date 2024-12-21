fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        println!("Usage: diff <ORIGFILE> <FILE> <PATCHFILE>");
        return;
    }
    diff_files(&args[1], &args[2], &args[3]).unwrap();
}

fn diff_files(orig_file: &str, file: &str, patch_file: &str) -> std::io::Result<()> {
    let old = std::fs::read(orig_file)?;
    let new = std::fs::read(file)?;
    let mut patch = Vec::new();
    let mut encoded = Vec::new();

    bsdiff::diff(&old, &new, &mut patch)?;
    aehobak::encode(&patch, &mut encoded)?;
    std::fs::write(patch_file, &encoded)
}

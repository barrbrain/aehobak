fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        println!("Usage: patch <ORIGFILE> <PATCHFILE> <FILE>");
        return;
    }
    patch_file(&args[1], &args[2], &args[3]).unwrap();
}

fn patch_file(orig_file: &str, patch_file: &str, file: &str) -> std::io::Result<()> {
    let old = std::fs::read(orig_file)?;
    let patch = std::fs::read(patch_file)?;
    let mut new = Vec::with_capacity(10_000_000);

    aehobak::patch(&old, &patch, &mut new)?;
    std::fs::write(file, &new)
}

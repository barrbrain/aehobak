fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage: bench <ORIGFILE> <FILE>");
        return;
    }
    bench(&args[1], &args[2]).unwrap();
}

fn bench(orig_file: &str, file: &str) -> std::io::Result<()> {
    let old = std::fs::read(orig_file)?;
    let new = std::fs::read(file)?;
    let mut patch = Vec::new();
    let mut encoded = Vec::new();

    aehobak::diff(&old, &new, &mut encoded)?;
    aehobak::decode(&mut encoded.as_slice(), &mut patch)?;
    println!("bsdiff:      {} bytes", patch.len());
    println!("aehobak:     {} bytes", encoded.len());
    let bsdiff_lz4 = lz4_flex::block::compress(&patch).len();
    println!("bsdiff+lz4:  {} bytes", bsdiff_lz4);
    let aehobak_lz4 = lz4_flex::block::compress(&encoded).len();
    println!("aehobak+lz4: {} bytes", aehobak_lz4);

    Ok(())
}

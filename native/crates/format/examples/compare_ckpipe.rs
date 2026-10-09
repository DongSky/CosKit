//! Compare every restored version, including raw tiles/precision/ICC/layers, across schema upgrades.
use photocraft_format::{load_path, pipeline};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [before, after] = args.as_slice() else { return Err("usage: compare_ckpipe BEFORE AFTER".into()) };
    let a = load_path(std::path::Path::new(before))?;
    let b = load_path(std::path::Path::new(after))?;
    let pa = a.pipeline.as_ref().ok_or("missing before history")?;
    let pb = b.pipeline.as_ref().ok_or("missing after history")?;
    if pa.versions.len() != pb.versions.len() {
        return Err("version count mismatch".into());
    }
    for v in &pa.versions {
        let (mut left, _) = pipeline::restore(&a, &v.id)?;
        let (mut right, _) = pipeline::restore(&b, &v.id)?;
        left.pipeline = None;
        right.pipeline = None;
        right.id = left.id;
        right.name = left.name.clone();
        if left != right {
            return Err(format!("{} restored document differs", v.id).into());
        }
    }
    println!("{} restored versions are identical: {}x{}, {:?}, {:?}", pa.versions.len(), a.size.width, a.size.height, a.mode, a.depth);
    Ok(())
}

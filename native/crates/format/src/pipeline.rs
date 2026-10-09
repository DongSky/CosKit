//! CKPipe v2 shares native compressed objects across versions. v1 ZIP snapshots remain readable.
use crate::store::Source;
use crate::{FormatError, Result, SaveOptions};
use photocraft_doc::{
    Document,
    pipeline::{Pipeline, Version},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct SnapshotRecord {
    pub manifest: String,
    pub objects: Vec<String>,
}

pub(crate) fn valid_object_path(path: &str) -> bool {
    let Some((kind, file)) = path.split_once('/') else { return false };
    matches!(kind, "tiles" | "blobs") && file.strip_suffix(".zst").is_some_and(crate::convert::is_valid_hash)
}

struct SnapshotSource<'a> {
    record: &'a SnapshotRecord,
    pool: &'a BTreeMap<String, Arc<Vec<u8>>>,
}
impl Source for SnapshotSource<'_> {
    fn get(&self, path: &str, max: usize) -> Result<Vec<u8>> {
        let bytes = if path == "manifest.json" {
            self.record.manifest.as_bytes()
        } else if valid_object_path(path) && self.record.objects.binary_search_by(|p| p.as_str().cmp(path)).is_ok() {
            self.pool.get(path).ok_or_else(|| FormatError::corrupt("missing shared snapshot object"))?.as_slice()
        } else {
            return Err(FormatError::corrupt("invalid snapshot object reference"));
        };
        if bytes.len() > max {
            return Err(FormatError::LimitExceeded("snapshot object".into()));
        }
        Ok(bytes.to_vec())
    }
}

fn record(bytes: &[u8]) -> Result<SnapshotRecord> {
    if bytes.len() > 32 << 20 {
        return Err(FormatError::LimitExceeded("snapshot descriptor".into()));
    }
    Ok(serde_json::from_slice(bytes)?)
}

/// Upgrade old snapshots without decoding their full-resolution surfaces.
fn upgrade(p: &mut Pipeline) -> Result<()> {
    if p.schema == 2 {
        return Ok(());
    }
    validate(p)?;
    let mut snapshots = BTreeMap::new();
    let mut remap = BTreeMap::new();
    for (old, bytes) in &p.snapshots {
        let zip = crate::zip::ZipReader::new(bytes)?;
        let manifest = String::from_utf8(zip.read_by_name("manifest.json", 32 << 20)?).map_err(|e| FormatError::corrupt(e.to_string()))?;
        let mut paths = BTreeSet::new();
        for entry in &zip.entries {
            if valid_object_path(&entry.name) {
                let data = zip.read(entry, 1 << 30)?;
                p.objects.entry(entry.name.clone()).or_insert_with(|| Arc::new(data));
                paths.insert(entry.name.clone());
            }
        }
        let bytes = Arc::new(serde_json::to_vec(&SnapshotRecord { manifest, objects: paths.into_iter().collect() })?);
        let hash = blake3::hash(&bytes).to_hex().to_string();
        remap.insert(old.clone(), hash.clone());
        snapshots.insert(hash, bytes);
    }
    for v in &mut p.versions {
        v.snapshot = remap.get(&v.snapshot).ok_or_else(|| FormatError::corrupt("missing migrated version"))?.clone();
    }
    p.previews = p.previews.iter().filter_map(|(old, bytes)| remap.get(old).map(|hash| (hash.clone(), bytes.clone()))).collect();
    p.snapshots = snapshots;
    p.schema = 2;
    Ok(())
}

/// Decode only the requested version. The shared object pool remains compressed.
pub fn load_snapshot(p: &Pipeline, hash: &str) -> Result<Document> {
    if !matches!(p.schema, 1 | 2) { return Err(FormatError::TooNew { found: p.schema, supported: 2 }); }
    let bytes = p.snapshots.get(hash).ok_or_else(|| FormatError::corrupt("missing CKPipe version snapshot"))?;
    let options = crate::LoadOptions { max_manifest_bytes: 32 << 20, max_total_bytes: 2 << 30, ..Default::default() };
    let doc = if p.schema == 1 {
        crate::load_from_bytes_with(bytes, &options)?
    } else {
        let r = record(bytes)?;
        crate::store::load(&SnapshotSource { record: &r, pool: &p.objects }, &options)?
    };
    if doc.pipeline.is_some() {
        return Err(FormatError::corrupt("recursive CKPipe snapshot"));
    }
    Ok(doc)
}

pub fn validate(p: &Pipeline) -> Result<()> {
    if !matches!(p.schema, 1 | 2) {
        return Err(FormatError::TooNew { found: p.schema, supported: 2 });
    }
    if p.versions.len() > 256 || p.snapshots.len() > 256 || p.operations.len() > 100_000 || p.conversations.len() > 10_000 || p.runs.len() > 1000 {
        return Err(FormatError::LimitExceeded("CKPipe project history limit".into()));
    }
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    if p.objects.len() > 1_000_000 || p.previews.len() > 256 {
        return Err(FormatError::LimitExceeded("project object count".into()));
    }
    for (path, bytes) in &p.objects {
        if !valid_object_path(path) || bytes.len() > 1 << 30 {
            return Err(FormatError::corrupt("invalid shared object"));
        }
        total = total.saturating_add(bytes.len() as u64);
    }
    for (hash, bytes) in &p.previews {
        if !p.snapshots.contains_key(hash) || bytes.len() > 16 << 20 {
            return Err(FormatError::corrupt("invalid project preview"));
        }
        total = total.saturating_add(bytes.len() as u64);
    }
    if total > 2 << 30 {
        return Err(FormatError::LimitExceeded("CKPipe project exceeds 2 GiB".into()));
    }
    for (hash, bytes) in &p.snapshots {
        if bytes.len() > 1 << 30 {
            return Err(FormatError::LimitExceeded("CKPipe snapshot exceeds 1 GiB".into()));
        }
        total = total.saturating_add(bytes.len() as u64);
        if total > 2 << 30 {
            return Err(FormatError::LimitExceeded("CKPipe snapshots exceed 2 GiB".into()));
        }
        if blake3::hash(bytes).to_hex().as_str() != hash {
            return Err(FormatError::corrupt("CKPipe snapshot hash mismatch"));
        }
        // Inspect only the bounded manifest: pixels are decoded lazily on restoration.
        let manifest = if p.schema == 1 {
            crate::store::read_manifest(&crate::store::ZipSource::new(bytes)?, &crate::LoadOptions { max_manifest_bytes: 32 << 20, ..Default::default() })?
        } else {
            let r = record(bytes)?;
            if r.objects.windows(2).any(|v| v.first() >= v.get(1)) || r.objects.iter().any(|path| !valid_object_path(path) || !p.objects.contains_key(path)) {
                return Err(FormatError::corrupt("invalid or missing shared version objects"));
            }
            crate::store::read_manifest(
                &SnapshotSource { record: &r, pool: &p.objects },
                &crate::LoadOptions { max_manifest_bytes: 32 << 20, ..Default::default() },
            )?
        };
        if manifest.document.pipeline.is_some() {
            return Err(FormatError::corrupt("recursive CKPipe snapshot"));
        }
    }
    for v in &p.versions {
        if v.id.is_empty()
            || seen.contains(&v.id)
            || v.parent.as_ref().is_some_and(|parent| !seen.contains(parent))
            || !p.snapshots.contains_key(&v.snapshot)
            || v.operation_count > p.operations.len()
        {
            return Err(FormatError::corrupt("invalid CKPipe version graph"));
        }
        seen.insert(v.id.clone());
    }
    if p.head.as_ref().is_some_and(|h| !seen.contains(h)) || p.operations.iter().any(|op| op.base.as_ref().is_some_and(|b| !seen.contains(b))) {
        return Err(FormatError::corrupt("unknown CKPipe version reference"));
    }
    Ok(())
}

/// Explicit checkpoint. Same pixels share the compressed snapshot blob, but keep labels/parents.
pub fn checkpoint(doc: &mut Document, label: &str, active: Option<u64>, selected: Vec<u64>) -> Result<String> {
    let mut plain = doc.clone();
    plain.pipeline = None;
    let mut p = doc.pipeline.as_deref().cloned().unwrap_or_default();
    upgrade(&mut p)?;
    let bytes = Arc::new(crate::PcraftWriter::default().capture(&plain, &mut p.objects)?);
    let hash = blake3::hash(&bytes).to_hex().to_string();
    if label == "Saved state"
        && let Some(v) = p.versions.last().filter(|v| Some(&v.id) == p.head.as_ref() && v.snapshot == hash && v.operation_count == p.operations.len())
    {
        let id = v.id.clone();
        doc.pipeline = Some(Arc::new(p));
        return Ok(id);
    }
    if p.versions.len() >= 256 {
        return Err(FormatError::LimitExceeded("256 project versions; start a new project to continue checkpointing".into()));
    }
    let id = format!("v{}", p.versions.len() + 1);
    p.snapshots.entry(hash.clone()).or_insert(bytes);
    p.versions.push(Version {
        id: id.clone(),
        parent: p.head.clone(),
        label: label.chars().take(256).collect(),
        snapshot: hash,
        operation_count: p.operations.len(),
        active_layer: active,
        selected_layers: selected,
    });
    p.head = Some(id.clone());
    validate(&p)?;
    doc.pipeline = Some(Arc::new(p));
    Ok(id)
}

pub fn restore(doc: &Document, id: &str) -> Result<(Document, Version)> {
    let p = doc.pipeline.as_ref().ok_or_else(|| FormatError::corrupt("no CKPipe project history"))?;
    let v = p.versions.iter().find(|v| v.id == id).ok_or_else(|| FormatError::corrupt("unknown CKPipe version"))?.clone();
    let mut restored = load_snapshot(p, &v.snapshot)?;
    restored.id = doc.id;
    restored.name = doc.name.clone();
    let mut branch = (**p).clone();
    branch.head = Some(id.to_string());
    restored.pipeline = Some(Arc::new(branch));
    Ok((restored, v))
}

pub fn save(doc: &Document, opts: &SaveOptions) -> Result<Vec<u8>> {
    let mut copy = doc.clone();
    // Capture unsaved pixels even if the caller has never made an explicit checkpoint.
    checkpoint(&mut copy, "Saved state", None, vec![])?;
    crate::save_to_bytes(&copy, opts)
}

/// Prepare before encoding; the caller adopts this metadata only after a successful write.
pub fn prepare_save(doc: &Document, name: &str) -> Result<Document> {
    let mut prepared = doc.clone();
    if name.rsplit('.').next().is_some_and(|s| s.eq_ignore_ascii_case("ckpipe")) {
        checkpoint(&mut prepared, "Saved state", None, vec![])?;
    }
    Ok(prepared)
}

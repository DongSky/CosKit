//! One history/cache allowance for the session. Never evict current pixels or durable versions.
use crate::Session;
#[derive(Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUsage {
    pub allowance_bytes: usize,
    pub project_bytes: usize,
    pub pixel_history_bytes: usize,
    pub history_budget_per_document: usize,
    pub preview_budget_bytes: usize,
    pub effect_budget_bytes: usize,
    pub documents: usize,
}
impl Session {
    /// Compressed durable versions plus previews. Cross-document sharing is counted
    /// conservatively; this is accounted storage, not operating-system RSS.
    pub fn project_bytes(&self) -> usize {
        self.documents()
            .iter()
            .filter_map(|s| s.pipeline.as_ref())
            .fold(0usize, |n, p| p.objects.values().chain(p.snapshots.values()).chain(p.previews.values()).fold(n, |n, bytes| n.saturating_add(bytes.len())))
    }
    pub fn rebalance_memory(&mut self) {
        let perf = &self.prefs().performance;
        let effects = perf.effect_budget_bytes();
        let previews = perf.preview_budget_bytes();
        let states = perf.history_states.max(1) as usize;
        let history =
            perf.history_budget_bytes().saturating_sub(self.project_bytes()).saturating_sub(effects).saturating_sub(previews) / self.docs.len().max(1);
        for st in &mut self.docs {
            st.history.max_states = states;
            // Zero means unlimited in History, so exhausted budgets must be one byte.
            st.history.max_bytes = history.max(1);
            st.history.trim(&st.doc);
        }
        photocraft_compose::set_effect_cache_budget(effects);
    }
    /// Called for diagnostics or full canvas refreshes, never for each partial brush frame.
    pub fn resource_usage(&self) -> ResourceUsage {
        let perf = &self.prefs().performance;
        ResourceUsage {
            allowance_bytes: perf.history_budget_bytes(),
            project_bytes: self.project_bytes(),
            pixel_history_bytes: self.documents().iter().fold(0usize, |n, s| n.saturating_add(s.history.pixel_bytes(&s.doc))),
            history_budget_per_document: self.documents().first().map_or(0, |s| s.history.max_bytes),
            preview_budget_bytes: perf.preview_budget_bytes(),
            effect_budget_bytes: perf.effect_budget_bytes(),
            documents: self.documents().len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn documents_share_allowance_and_durable_versions_are_accounted() {
        let mut s = Session::new();
        s.execute("file.new", json!({"width":8,"height":8})).unwrap();
        let first = s.active().unwrap().history.max_bytes;
        s.execute("file.new", json!({"width":8,"height":8})).unwrap();
        assert!(s.documents().iter().all(|d| d.history.max_bytes <= first / 2));
        s.execute("coskit.project.checkpoint", json!({})).unwrap();
        assert!(s.resource_usage().project_bytes > 0);
        assert!(s.documents().iter().all(|d| d.history.max_bytes < first / 2));
        s.close(1);
        assert_eq!(s.active().unwrap().history.max_bytes, first);
    }
}

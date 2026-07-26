/// Snapshot-based undo/redo history for Owerlayer.
///
/// Each entry is a full clone of the project state tagged with a
/// human-readable label.  On undo/redo or jump-to-entry, the stored
/// snapshot is restored verbatim.  The ring-buffer is capped at
/// MAX_HISTORY entries to bound memory use.

use crate::project::Project;

const MAX_HISTORY: usize = 50;

/// A single history entry.
#[derive(Clone)]
pub struct HistoryEntry {
    pub label: String,
    pub snapshot: Project,
}

pub struct History {
    /// All saved states, oldest at index 0.
    pub entries: Vec<HistoryEntry>,
    /// The index of the state that is *currently live*.
    /// `None` means the history is empty.
    pub cursor: Option<usize>,
}

impl History {
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(MAX_HISTORY + 1),
            cursor: None,
        }
    }

    pub fn format_action_label(project: &Project, raw_action: &str) -> String {
        let layer_idx = project.active_layer;
        let layer_name = project.layers.get(layer_idx)
            .map(|l| if l.name.is_empty() { format!("Layer {}", layer_idx + 1) } else { l.name.clone() })
            .unwrap_or_else(|| format!("Layer {}", layer_idx + 1));

        if let Some(sel) = project.selected_object {
            if let Some(layer) = project.layers.get(sel.layer_idx) {
                match sel.object_type {
                    crate::types::ObjectType::Image => {
                        if let Some(img) = layer.placed_images.get(sel.object_idx) {
                            let obj_name = if !img.name.is_empty() {
                                img.name.clone()
                            } else {
                                format!("Object {}", sel.object_idx + 1)
                            };
                            let is_source_action = raw_action.contains("Source") || raw_action.contains("Coordinates");
                            let src_type = if is_source_action {
                                match img.capture_source {
                                    crate::types::CaptureSource::Overlay => "Source Overlay; ",
                                    crate::types::CaptureSource::Desktop => "Source Desktop; ",
                                    crate::types::CaptureSource::Origin  => "Source Window; ",
                                }
                            } else {
                                ""
                            };
                            return format!("{}; {}; {}{}", layer_name, obj_name, src_type, raw_action);
                        }
                    }
                    crate::types::ObjectType::Stroke => {
                        return format!("{}; Stroke {}; {}", layer_name, sel.object_idx + 1, raw_action);
                    }
                    crate::types::ObjectType::Text => {
                        return format!("{}; Text {}; {}", layer_name, sel.object_idx + 1, raw_action);
                    }
                }
            }
        }

        format!("{}; {}", layer_name, raw_action)
    }

    /// Push the current project state with a descriptive label.
    /// Any redo candidates (entries after the cursor) are discarded.
    pub fn push(&mut self, project: &Project, label: impl Into<String>) {
        let raw = label.into();
        let formatted = Self::format_action_label(project, &raw);

        // Discard redo entries after cursor.
        if let Some(c) = self.cursor {
            self.entries.truncate(c + 1);
        }

        // Evict oldest if at capacity.
        if self.entries.len() >= MAX_HISTORY {
            self.entries.remove(0);
            // cursor stays at the same index — it now points to the
            // same relative position in the shorter list.
            if let Some(c) = self.cursor.as_mut() {
                *c = c.saturating_sub(1);
            }
        }

        self.entries.push(HistoryEntry {
            label: formatted,
            snapshot: project.clone(),
        });
        self.cursor = Some(self.entries.len() - 1);
    }

    /// Returns true if undo is possible.
    pub fn can_undo(&self) -> bool {
        matches!(self.cursor, Some(c) if c > 0)
    }

    /// Returns true if redo is possible.
    pub fn can_redo(&self) -> bool {
        match self.cursor {
            Some(c) => c + 1 < self.entries.len(),
            None => false,
        }
    }

    /// Step back one entry.  Returns the snapshot to restore, or None.
    pub fn undo(&mut self) -> Option<&Project> {
        let c = self.cursor?;
        if c == 0 {
            return None;
        }
        self.cursor = Some(c - 1);
        self.entries.get(c - 1).map(|e| &e.snapshot)
    }

    /// Step forward one entry.  Returns the snapshot to restore, or None.
    pub fn redo(&mut self) -> Option<&Project> {
        let c = self.cursor?;
        let next = c + 1;
        if next >= self.entries.len() {
            return None;
        }
        self.cursor = Some(next);
        self.entries.get(next).map(|e| &e.snapshot)
    }

    /// Jump directly to any entry by index.
    /// Returns the snapshot to restore, or None if index is out of range.
    pub fn jump_to(&mut self, index: usize) -> Option<&Project> {
        if index < self.entries.len() {
            self.cursor = Some(index);
            self.entries.get(index).map(|e| &e.snapshot)
        } else {
            None
        }
    }
}

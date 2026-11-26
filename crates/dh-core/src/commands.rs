#[derive(Debug, Clone, PartialEq)]
pub struct UndoEntry<T> {
    pub label: String,
    pub state: T,
}

const MAX_UNDO_STEPS: usize = 80;

/// Bounded command-labeled undo/redo stack for project and parameter edits.
pub struct UndoHistory<T> {
    undo_stack: Vec<UndoEntry<T>>,
    redo_stack: Vec<UndoEntry<T>>,
    last_committed: Option<T>,
}

impl<T: Clone + PartialEq> UndoHistory<T> {
    pub fn new(initial: T) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_committed: Some(initial),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo_stack.last().map(|e| e.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo_stack.last().map(|e| e.label.as_str())
    }

    /// Commits a new state snapshot with an action label if it differs from the latest baseline.
    pub fn commit(&mut self, label: impl Into<String>, state: T) {
        if self.last_committed.as_ref() == Some(&state) {
            return;
        }
        if let Some(prev) = self.last_committed.take() {
            self.undo_stack.push(UndoEntry {
                label: label.into(),
                state: prev,
            });
            if self.undo_stack.len() > MAX_UNDO_STEPS {
                self.undo_stack.remove(0);
            }
        }
        self.last_committed = Some(state);
        self.redo_stack.clear();
    }

    pub fn set_baseline(&mut self, state: T) {
        self.last_committed = Some(state);
    }

    pub fn reset(&mut self, initial: T) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.last_committed = Some(initial);
    }

    pub fn undo(&mut self, current: T) -> Option<(String, T)> {
        let entry = self.undo_stack.pop()?;
        let label = entry.label.clone();
        self.redo_stack.push(UndoEntry {
            label: label.clone(),
            state: current,
        });
        self.last_committed = Some(entry.state.clone());
        Some((label, entry.state))
    }

    pub fn redo(&mut self, current: T) -> Option<(String, T)> {
        let entry = self.redo_stack.pop()?;
        let label = entry.label.clone();
        self.undo_stack.push(UndoEntry {
            label: label.clone(),
            state: current,
        });
        self.last_committed = Some(entry.state.clone());
        Some((label, entry.state))
    }
}

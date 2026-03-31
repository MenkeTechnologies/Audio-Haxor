// ── Notes & Tags ──
// Store notes and tags on any item, persisted in prefs

function getNotes() {
  return prefs.getObject('itemNotes', {});
}

function getNote(path) {
  return getNotes()[path] || null;
}

function setNote(path, note, tags) {
  const notes = getNotes();
  if ((!note || !note.trim()) && (!tags || tags.length === 0)) {
    delete notes[path];
  } else {
    notes[path] = { note: note || '', tags: tags || [], updatedAt: new Date().toISOString() };
  }
  prefs.setItem('itemNotes', notes);
}

function getAllTags() {
  const notes = getNotes();
  const tags = new Set();
  for (const entry of Object.values(notes)) {
    if (entry.tags) entry.tags.forEach(t => tags.add(t));
  }
  return [...tags].sort();
}

let _noteModalPath = null;

function showNoteEditor(path, name) {
  let existing = document.getElementById('noteModal');
  if (existing) existing.remove();

  _noteModalPath = path;
  const current = getNote(path);
  const noteText = current?.note || '';
  const tags = current?.tags?.join(', ') || '';

  const html = `<div class="modal-overlay" id="noteModal" data-action-modal="closeNote">
    <div class="modal-content modal-small">
      <div class="modal-header">
        <h2>Notes: ${escapeHtml(name)}</h2>
        <button class="modal-close" data-action-modal="closeNote">&#10005;</button>
      </div>
      <div class="modal-body">
        <label class="note-label">Note</label>
        <textarea class="note-textarea" id="noteText" rows="4" placeholder="Add a note...">${escapeHtml(noteText)}</textarea>
        <label class="note-label">Tags <span style="color: var(--text-muted); font-weight: 400;">(comma-separated)</span></label>
        <input type="text" class="note-input" id="noteTags" placeholder="kick, bass, favorite" value="${escapeHtml(tags)}">
        <div class="note-actions">
          <button class="btn btn-primary" data-action-modal="saveNote">Save</button>
          <button class="btn btn-secondary" data-action-modal="closeNote">Cancel</button>
          ${current ? '<button class="btn btn-stop" data-action-modal="deleteNote">Delete Note</button>' : ''}
        </div>
      </div>
    </div>
  </div>`;
  document.body.insertAdjacentHTML('beforeend', html);
  document.getElementById('noteText').focus();
}

function closeNoteModal() {
  const modal = document.getElementById('noteModal');
  if (modal) modal.remove();
  _noteModalPath = null;
}

function saveNoteFromModal() {
  if (!_noteModalPath) return;
  const note = document.getElementById('noteText').value;
  const tagsStr = document.getElementById('noteTags').value;
  const tags = tagsStr.split(',').map(t => t.trim()).filter(Boolean);
  setNote(_noteModalPath, note, tags);
  closeNoteModal();
  showToast('Note saved');
}

function deleteNoteFromModal() {
  if (!_noteModalPath) return;
  setNote(_noteModalPath, '', []);
  closeNoteModal();
  showToast('Note deleted');
}

// Event delegation for note modal
document.addEventListener('click', (e) => {
  const action = e.target.closest('[data-action-modal]');
  if (!action) return;
  const act = action.dataset.actionModal;
  if (act === 'closeNote') {
    // Only close if clicking overlay background or close/cancel button
    if (e.target === action || action.classList.contains('modal-close') || action.classList.contains('btn-secondary')) {
      closeNoteModal();
    }
  } else if (act === 'saveNote') {
    saveNoteFromModal();
  } else if (act === 'deleteNote') {
    deleteNoteFromModal();
  }
});

// Close on Escape
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && document.getElementById('noteModal')) {
    closeNoteModal();
  }
});

// Get note indicator HTML for a row
function noteIndicator(path) {
  const note = getNote(path);
  if (!note) return '';
  const tagHtml = note.tags?.length ? ` [${note.tags.join(', ')}]` : '';
  return `<span class="note-icon" title="${escapeHtml(note.note + tagHtml)}">&#128221;</span>`;
}

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

// ── Notes Tab ──
function renderNotesTab() {
  const list = document.getElementById('notesList');
  const empty = document.getElementById('notesEmptyState');
  if (!list) return;

  const notes = getNotes();
  const entries = Object.entries(notes);
  const search = (document.getElementById('noteSearchInput')?.value || '').toLowerCase();
  const activeTag = list._activeTag || null;

  const filtered = entries.filter(([path, n]) => {
    if (activeTag && (!n.tags || !n.tags.includes(activeTag))) return false;
    if (search) {
      const name = path.split('/').pop() || '';
      if (!name.toLowerCase().includes(search) &&
          !path.toLowerCase().includes(search) &&
          !(n.note || '').toLowerCase().includes(search) &&
          !(n.tags || []).some(t => t.toLowerCase().includes(search))) return false;
    }
    return true;
  }).sort((a, b) => (b[1].updatedAt || '').localeCompare(a[1].updatedAt || ''));

  if (filtered.length === 0) {
    if (empty) empty.style.display = entries.length === 0 ? '' : 'none';
    list.innerHTML = entries.length > 0 && filtered.length === 0
      ? '<div class="state-message"><div class="state-icon">&#128269;</div><h2>No matching notes</h2></div>'
      : '';
    if (entries.length === 0 && empty) empty.style.display = '';
    return;
  }
  if (empty) empty.style.display = 'none';

  // Tag cloud
  const allTags = getAllTags();
  let tagCloud = '';
  if (allTags.length > 0) {
    tagCloud = `<div style="display:flex;flex-wrap:wrap;gap:6px;margin-bottom:16px;">
      <span class="note-tag" style="cursor:pointer;${!activeTag ? 'background:var(--yellow);color:var(--bg)' : ''}" data-action-tag="all">All</span>
      ${allTags.map(t => `<span class="note-tag" style="cursor:pointer;${activeTag === t ? 'background:var(--yellow);color:var(--bg)' : ''}" data-action-tag="${escapeHtml(t)}">${escapeHtml(t)}</span>`).join('')}
    </div>`;
  }

  list.innerHTML = tagCloud + filtered.map(([path, n]) => {
    const name = path.split('/').pop().replace(/\.[^.]+$/, '') || path;
    const tags = (n.tags || []).map(t => `<span class="note-tag">${escapeHtml(t)}</span>`).join('');
    const date = n.updatedAt ? new Date(n.updatedAt).toLocaleString() : '';
    return `<div class="note-card">
      <div class="note-card-header">
        <span class="note-card-name">${escapeHtml(name)}</span>
        <span class="note-card-date">${date}</span>
        <div class="note-card-actions">
          <button class="btn-small btn-secondary" data-action-note="edit" data-path="${escapeHtml(path)}" data-name="${escapeHtml(name)}" title="Edit note" style="padding:3px 8px;font-size:10px;">Edit</button>
          <button class="btn-small btn-stop" data-action-note="delete" data-path="${escapeHtml(path)}" title="Delete note" style="padding:3px 8px;font-size:10px;">&#10005;</button>
        </div>
      </div>
      <div class="note-card-path" title="${escapeHtml(path)}">${escapeHtml(path)}</div>
      ${n.note ? `<div class="note-card-body">${escapeHtml(n.note)}</div>` : ''}
      ${tags ? `<div class="note-card-tags">${tags}</div>` : ''}
    </div>`;
  }).join('');
}

function clearAllNotes() {
  if (!confirm('Delete all notes and tags?')) return;
  prefs.setItem('itemNotes', {});
  renderNotesTab();
  showToast('All notes deleted');
}

// Tag click filtering + note card actions
document.addEventListener('click', (e) => {
  const tag = e.target.closest('[data-action-tag]');
  if (tag) {
    const list = document.getElementById('notesList');
    const val = tag.dataset.actionTag;
    list._activeTag = val === 'all' ? null : val;
    renderNotesTab();
    return;
  }
  const noteAction = e.target.closest('[data-action-note]');
  if (noteAction) {
    const act = noteAction.dataset.actionNote;
    const path = noteAction.dataset.path;
    if (act === 'edit') {
      showNoteEditor(path, noteAction.dataset.name);
    } else if (act === 'delete') {
      setNote(path, '', []);
      renderNotesTab();
      showToast('Note deleted');
    }
  }
});

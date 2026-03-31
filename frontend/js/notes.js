// ── Notes & Tags ──

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

function getStandaloneTags() {
  return prefs.getObject('standaloneTags', []);
}

function setStandaloneTags(tags) {
  prefs.setItem('standaloneTags', tags);
}

function getAllTags() {
  const notes = getNotes();
  const tags = new Set(getStandaloneTags());
  for (const entry of Object.values(notes)) {
    if (entry.tags) entry.tags.forEach(t => tags.add(t));
  }
  return [...tags].sort();
}

function getTagCounts() {
  const notes = getNotes();
  const counts = {};
  // Include standalone tags with 0 count
  for (const t of getStandaloneTags()) counts[t] = 0;
  for (const entry of Object.values(notes)) {
    if (entry.tags) entry.tags.forEach(t => { counts[t] = (counts[t] || 0) + 1; });
  }
  return counts;
}

function getItemsWithTag(tag) {
  const notes = getNotes();
  return Object.entries(notes)
    .filter(([, n]) => n.tags && n.tags.includes(tag))
    .map(([path, n]) => ({ path, ...n }));
}

function hasTag(path, tag) {
  const note = getNote(path);
  return note?.tags?.includes(tag) || false;
}

function addTagToItem(path, tag) {
  const note = getNote(path) || { note: '', tags: [] };
  if (!note.tags.includes(tag)) {
    note.tags.push(tag);
    setNote(path, note.note, note.tags);
  }
}

function removeTagFromItem(path, tag) {
  const note = getNote(path);
  if (!note) return;
  note.tags = note.tags.filter(t => t !== tag);
  setNote(path, note.note, note.tags);
}

function renameTag(oldTag, newTag) {
  const notes = getNotes();
  let changed = 0;
  for (const [path, n] of Object.entries(notes)) {
    if (n.tags && n.tags.includes(oldTag)) {
      n.tags = n.tags.map(t => t === oldTag ? newTag : t);
      n.tags = [...new Set(n.tags)]; // dedupe
      changed++;
    }
  }
  if (changed > 0) {
    prefs.setItem('itemNotes', notes);
    showToast(`Renamed "${oldTag}" → "${newTag}" on ${changed} items`);
  }
}

function deleteTag(tag) {
  const notes = getNotes();
  let changed = 0;
  for (const [path, n] of Object.entries(notes)) {
    if (n.tags && n.tags.includes(tag)) {
      n.tags = n.tags.filter(t => t !== tag);
      changed++;
    }
  }
  if (changed > 0) {
    prefs.setItem('itemNotes', notes);
  }
  // Also remove from standalone tags
  const standalone = getStandaloneTags();
  const idx = standalone.indexOf(tag);
  if (idx !== -1) {
    standalone.splice(idx, 1);
    setStandaloneTags(standalone);
  }
  showToast(`Removed tag "${tag}"${changed > 0 ? ` from ${changed} items` : ''}`);
}

// ── Note Editor Modal ──

let _noteModalPath = null;

function showNoteEditor(path, name) {
  let existing = document.getElementById('noteModal');
  if (existing) existing.remove();

  _noteModalPath = path;
  const current = getNote(path);
  const noteText = current?.note || '';
  const tags = current?.tags?.join(', ') || '';
  const allTags = getAllTags();
  const suggestions = allTags.length > 0
    ? `<div class="note-tag-suggestions" id="noteSuggestions">
        <label class="note-label" style="margin-bottom:4px;">Existing tags <span style="color:var(--text-muted);font-weight:400;">(click to add)</span></label>
        <div style="display:flex;flex-wrap:wrap;gap:4px;margin-bottom:12px;">
          ${allTags.map(t => `<span class="note-tag" style="cursor:pointer;" data-action-suggest="${escapeHtml(t)}">${escapeHtml(t)}</span>`).join('')}
        </div>
      </div>` : '';

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
        ${suggestions}
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
  // Refresh notes tab if visible
  if (document.getElementById('tabNotes')?.classList.contains('active')) renderNotesTab();
}

function deleteNoteFromModal() {
  if (!_noteModalPath) return;
  setNote(_noteModalPath, '', []);
  closeNoteModal();
  showToast('Note deleted');
  if (document.getElementById('tabNotes')?.classList.contains('active')) renderNotesTab();
}

// Event delegation for note modal + tag suggestions
document.addEventListener('click', (e) => {
  const action = e.target.closest('[data-action-modal]');
  if (action) {
    const act = action.dataset.actionModal;
    if (act === 'closeNote') {
      if (e.target === action || action.classList.contains('modal-close') || action.classList.contains('btn-secondary')) {
        closeNoteModal();
      }
    } else if (act === 'saveNote') {
      saveNoteFromModal();
    } else if (act === 'deleteNote') {
      deleteNoteFromModal();
    }
  }

  // Tag suggestion click — append to tag input
  const suggest = e.target.closest('[data-action-suggest]');
  if (suggest) {
    const tag = suggest.dataset.actionSuggest;
    const input = document.getElementById('noteTags');
    if (input) {
      const current = input.value.split(',').map(t => t.trim()).filter(Boolean);
      if (!current.includes(tag)) {
        current.push(tag);
        input.value = current.join(', ');
      }
    }
  }
});

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
    if (entries.length === 0) {
      list.innerHTML = '';
      if (empty) empty.style.display = '';
    } else {
      if (empty) empty.style.display = 'none';
      list.innerHTML = '<div class="state-message"><div class="state-icon">&#128269;</div><h2>No matching notes</h2></div>';
    }
    return;
  }
  if (empty) empty.style.display = 'none';

  // Tag cloud with counts
  const tagCounts = getTagCounts();
  const allTags = Object.keys(tagCounts).sort();
  let tagCloud = '';
  if (allTags.length > 0) {
    tagCloud = `<div class="notes-tag-cloud">
      <span class="note-tag ${!activeTag ? 'tag-active' : ''}" style="cursor:pointer;" data-action-tag="all">All (${entries.length})</span>
      ${allTags.map(t => `<span class="note-tag ${activeTag === t ? 'tag-active' : ''}" style="cursor:pointer;" data-action-tag="${escapeHtml(t)}">${escapeHtml(t)} (${tagCounts[t]})</span>`).join('')}
    </div>`;
  }

  // Stats summary
  const statsHtml = `<div class="notes-stats">${entries.length} notes | ${allTags.length} tags${activeTag ? ` | Filtering: "${escapeHtml(activeTag)}"` : ''}</div>`;

  list.innerHTML = tagCloud + statsHtml + filtered.map(([path, n]) => {
    const name = path.split('/').pop().replace(/\.[^.]+$/, '') || path;
    const tags = (n.tags || []).map(t =>
      `<span class="note-tag" style="cursor:pointer;" data-action-tag="${escapeHtml(t)}">${escapeHtml(t)}</span>`
    ).join('');
    const date = n.updatedAt ? new Date(n.updatedAt).toLocaleString() : '';
    return `<div class="note-card">
      <div class="note-card-header">
        <span class="note-card-name" title="${escapeHtml(path)}">${escapeHtml(name)}</span>
        <span class="note-card-date">${date}</span>
        <div class="note-card-actions">
          <button class="btn-small btn-secondary" data-action-note="edit" data-path="${escapeHtml(path)}" data-name="${escapeHtml(name)}" title="Edit note" style="padding:3px 8px;font-size:10px;">Edit</button>
          <button class="btn-small btn-stop" data-action-note="delete" data-path="${escapeHtml(path)}" title="Delete note" style="padding:3px 8px;font-size:10px;">&#10005;</button>
        </div>
      </div>
      <div class="note-card-path">${escapeHtml(path)}</div>
      ${n.note ? `<div class="note-card-body">${escapeHtml(n.note)}</div>` : ''}
      ${tags ? `<div class="note-card-tags">${tags}</div>` : ''}
    </div>`;
  }).join('');
}

function clearAllNotes() {
  if (!confirm('Delete all notes and tags?')) return;
  prefs.setItem('itemNotes', {});
  setStandaloneTags([]);
  renderNotesTab();
  renderGlobalTagBar();
  showToast('All notes and tags deleted');
}

// ── Tags Manager Tab ──
function renderTagsManager() {
  const container = document.getElementById('tagsManager');
  const empty = document.getElementById('tagsEmptyState');
  if (!container) return;

  const tagCounts = getTagCounts();
  const allTags = Object.keys(tagCounts).sort();
  const search = (document.getElementById('tagSearchInput')?.value || '').toLowerCase();
  const filtered = search ? allTags.filter(t => t.toLowerCase().includes(search)) : allTags;

  if (filtered.length === 0) {
    if (empty) empty.style.display = allTags.length === 0 ? '' : 'none';
    container.innerHTML = allTags.length > 0
      ? '<div class="state-message"><div class="state-icon">&#128269;</div><h2>No matching tags</h2></div>'
      : '';
    if (allTags.length === 0 && empty) empty.style.display = '';
    return;
  }
  if (empty) empty.style.display = 'none';

  const notes = getNotes();
  const totalItems = Object.keys(notes).length;

  let html = `<div class="tag-stats">${allTags.length} tags across ${totalItems} items</div>`;

  html += filtered.map(tag => {
    const count = tagCounts[tag];
    const items = getItemsWithTag(tag);
    return `<div class="tag-manager-card">
      <div class="tag-manager-header">
        <span class="tag-manager-name">${escapeHtml(tag)}</span>
        <span class="tag-manager-count">${count} item${count !== 1 ? 's' : ''}</span>
        <button class="btn-small btn-secondary" data-tag-action="rename" data-tag="${escapeHtml(tag)}" style="padding:3px 8px;font-size:10px;">Rename</button>
        <button class="btn-small btn-secondary" data-tag-action="filter" data-tag="${escapeHtml(tag)}" style="padding:3px 8px;font-size:10px;">Filter All Tabs</button>
        <button class="btn-small btn-stop" data-tag-action="delete" data-tag="${escapeHtml(tag)}" style="padding:3px 8px;font-size:10px;">&#10005;</button>
      </div>
      <div class="tag-manager-items">
        ${items.slice(0, 20).map(item => {
          const name = item.path.split('/').pop().replace(/\.[^.]+$/, '');
          return `<div class="tag-manager-item">
            <span class="tag-manager-item-name" title="${escapeHtml(item.path)}">${escapeHtml(name)}</span>
            <button class="btn-small" data-tag-action="remove-from" data-tag="${escapeHtml(tag)}" data-path="${escapeHtml(item.path)}" style="padding:2px 6px;font-size:9px;border:1px solid var(--border);background:transparent;color:var(--text-muted);cursor:pointer;">&#10005;</button>
          </div>`;
        }).join('')}
        ${items.length > 20 ? `<div style="color:var(--text-muted);font-size:11px;padding:4px 8px;">...and ${items.length - 20} more</div>` : ''}
      </div>
    </div>`;
  }).join('');

  container.innerHTML = html;
}

function createNewTag() {
  const tag = prompt('Enter new tag name:');
  if (!tag || !tag.trim()) return;
  const name = tag.trim();
  const existing = getAllTags();
  if (existing.includes(name)) {
    showToast(`Tag "${name}" already exists`);
    return;
  }
  const standalone = getStandaloneTags();
  standalone.push(name);
  setStandaloneTags(standalone);
  showToast(`Tag "${name}" created — right-click any item to apply it`);
  renderTagsManager();
  renderGlobalTagBar();
}

// ── Global Tag Filter ──
let _globalActiveTag = null;

function getGlobalActiveTag() { return _globalActiveTag; }

function renderGlobalTagBar() {
  const bar = document.getElementById('globalTagBar');
  const list = document.getElementById('globalTagList');
  if (!bar || !list) return;

  const allTags = getAllTags();
  if (allTags.length === 0) {
    bar.style.display = 'none';
    return;
  }
  bar.style.display = 'flex';
  list.innerHTML = allTags.map(t =>
    `<span class="global-tag-item${_globalActiveTag === t ? ' active' : ''}" data-action-global-tag="${escapeHtml(t)}">${escapeHtml(t)}</span>`
  ).join('');
}

function setGlobalTag(tag) {
  _globalActiveTag = _globalActiveTag === tag ? null : tag;
  renderGlobalTagBar();
  // Re-filter the active tab
  const active = document.querySelector('.tab-content.active');
  if (active) {
    if (active.id === 'tabPlugins') filterPlugins();
    else if (active.id === 'tabSamples') filterAudioSamples();
    else if (active.id === 'tabDaw') filterDawProjects();
    else if (active.id === 'tabPresets') filterPresets();
    else if (active.id === 'tabFavorites') renderFavorites();
    else if (active.id === 'tabNotes') renderNotesTab();
  }
}

function clearGlobalTag() {
  _globalActiveTag = null;
  renderGlobalTagBar();
  // Re-filter active tab
  setGlobalTag(null);
}

// Check if an item passes the global tag filter
function passesGlobalTagFilter(path) {
  if (!_globalActiveTag) return true;
  return hasTag(path, _globalActiveTag);
}

// Render global tag bar on tab switch
const _origSwitchTabForTags = switchTab;
switchTab = function(tab) {
  _origSwitchTabForTags(tab);
  renderGlobalTagBar();
};

// Tag click filtering + note card actions + tag management + global tag
document.addEventListener('click', (e) => {
  // Global tag bar
  const globalTag = e.target.closest('[data-action-global-tag]');
  if (globalTag) {
    setGlobalTag(globalTag.dataset.actionGlobalTag);
    return;
  }

  const tag = e.target.closest('[data-action-tag]');
  if (tag) {
    const list = document.getElementById('notesList');
    if (!list) return;
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
      renderGlobalTagBar();
      showToast('Note deleted');
    }
  }

  // Tag manager actions
  const tagAction = e.target.closest('[data-tag-action]');
  if (tagAction) {
    const act = tagAction.dataset.tagAction;
    const tag = tagAction.dataset.tag;
    if (act === 'rename') {
      const newName = prompt(`Rename tag "${tag}" to:`, tag);
      if (newName && newName.trim() && newName.trim() !== tag) {
        renameTag(tag, newName.trim());
        renderTagsManager();
        renderGlobalTagBar();
      }
    } else if (act === 'delete') {
      if (confirm(`Remove tag "${tag}" from all items?`)) {
        deleteTag(tag);
        renderTagsManager();
        renderGlobalTagBar();
      }
    } else if (act === 'filter') {
      setGlobalTag(tag);
      switchTab('plugins'); // Switch to plugins to see the filter in action
    } else if (act === 'remove-from') {
      const path = tagAction.dataset.path;
      removeTagFromItem(path, tag);
      renderTagsManager();
      renderGlobalTagBar();
      showToast(`Tag "${tag}" removed`);
    }
  }

  // Create tag button
  if (e.target.closest('[data-action="createTag"]')) {
    createNewTag();
  }
});

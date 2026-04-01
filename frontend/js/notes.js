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
        <textarea class="note-textarea" id="noteText" rows="4" placeholder="Add a note..." autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false">${escapeHtml(noteText)}</textarea>
        <label class="note-label">Tags <span style="color: var(--text-muted); font-weight: 400;">(comma-separated)</span></label>
        <input type="text" class="note-input" id="noteTags" placeholder="kick, bass, favorite" value="${escapeHtml(tags)}" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false">
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

  let filtered = entries.filter(([path, n]) => {
    if (activeTag && (!n.tags || !n.tags.includes(activeTag))) return false;
    return true;
  });
  if (search) {
    const scored = filtered.map(([path, n]) => {
      const name = path.split('/').pop() || '';
      const fields = [name, path, n.note || '', ...(n.tags || [])];
      return { entry: [path, n], score: searchScore(search, fields, 'fuzzy') };
    }).filter(s => s.score > 0);
    scored.sort((a, b) => b.score - a.score);
    filtered = scored.map(s => s.entry);
  } else {
    filtered.sort((a, b) => (b[1].updatedAt || '').localeCompare(a[1].updatedAt || ''));
  }

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

function exportNotes() {
  const notes = getNotes();
  const tags = getStandaloneTags();
  const entries = Object.entries(notes);
  if (entries.length === 0 && tags.length === 0) { showToast('No notes or tags to export'); return; }
  const data = { notes, standaloneTags: tags };
  const count = entries.length + tags.length;
  _exportCtx = {
    title: 'Notes & Tags',
    defaultName: exportFileName('notes', count),
    exportFn: async (fmt, filePath) => {
      if (fmt === 'pdf') {
        const headers = ['Path', 'Note', 'Tags', 'Updated'];
        const rows = entries.map(([path, n]) => [path, n.note || '', (n.tags || []).join(', '), n.updatedAt || '']);
        if (tags.length > 0) rows.push(['[Standalone Tags]', tags.join(', '), '', '']);
        await window.vstUpdater.exportPdf('Notes & Tags', headers, rows, filePath);
      } else if (fmt === 'csv' || fmt === 'tsv') {
        const sep = fmt === 'tsv' ? '\t' : ',';
        const esc = (v) => { const s = String(v || ''); return s.includes(sep) || s.includes('"') || s.includes('\n') ? '"' + s.replace(/"/g, '""') + '"' : s; };
        const lines = ['Path' + sep + 'Note' + sep + 'Tags' + sep + 'Updated'];
        for (const [path, n] of entries) lines.push([path, n.note || '', (n.tags || []).join(', '), n.updatedAt || ''].map(esc).join(sep));
        await window.__TAURI__.core.invoke('write_text_file', { filePath, contents: lines.join('\n') });
      } else if (fmt === 'toml') {
        await window.vstUpdater.exportToml(data, filePath);
      } else {
        await window.__TAURI__.core.invoke('write_text_file', { filePath, contents: JSON.stringify(data, null, 2) });
      }
    }
  };
  showExportModal('notes', 'Notes & Tags', count);
}

async function importNotes() {
  const dialogApi = window.__TAURI_PLUGIN_DIALOG__;
  if (!dialogApi) return;
  const selected = await dialogApi.open({ title: 'Import Notes & Tags', multiple: false, filters: ALL_IMPORT_FILTERS });
  if (!selected) return;
  const filePath = typeof selected === 'string' ? selected : selected.path;
  if (!filePath) return;
  try {
    let imported;
    if (filePath.endsWith('.toml')) {
      imported = await window.vstUpdater.importToml(filePath);
    } else {
      const text = await window.__TAURI__.core.invoke('read_text_file', { filePath }));
      imported = JSON.parse(text);
    }
    // Merge notes
    const existing = getNotes();
    let added = 0;
    if (imported.notes && typeof imported.notes === 'object') {
      for (const [path, note] of Object.entries(imported.notes)) {
        if (!existing[path]) { existing[path] = note; added++; }
      }
      prefs.setItem('itemNotes', existing);
    }
    // Merge standalone tags
    if (Array.isArray(imported.standaloneTags)) {
      const current = new Set(getStandaloneTags());
      let tagAdded = 0;
      for (const t of imported.standaloneTags) {
        if (!current.has(t)) { current.add(t); tagAdded++; }
      }
      setStandaloneTags([...current]);
      added += tagAdded;
    }
    renderNotesTab();
    renderGlobalTagBar();
    showToast(`Imported ${added} notes/tags`);
  } catch (e) {
    showToast(`Import failed: ${e.message || e}`, 4000, 'error');
  }
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
  let filtered;
  if (search) {
    const scored = allTags.map(t => ({ t, score: searchScore(search, [t], 'fuzzy') })).filter(s => s.score > 0);
    scored.sort((a, b) => b.score - a.score);
    filtered = scored.map(s => s.t);
  } else {
    filtered = allTags;
  }

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
        <button class="btn-small btn-secondary" data-tag-action="rename" data-tag="${escapeHtml(tag)}" style="padding:3px 8px;font-size:10px;" title="Rename this tag">Rename</button>
        <button class="btn-small btn-secondary" data-tag-action="filter" data-tag="${escapeHtml(tag)}" style="padding:3px 8px;font-size:10px;" title="Filter all tabs by this tag">Filter All Tabs</button>
        <button class="btn-small btn-stop" data-tag-action="delete" data-tag="${escapeHtml(tag)}" style="padding:3px 8px;font-size:10px;" title="Delete this tag from all items">&#10005;</button>
      </div>
      <div class="tag-manager-items">
        ${items.slice(0, 20).map(item => {
          const name = item.path.split('/').pop().replace(/\.[^.]+$/, '');
          return `<div class="tag-manager-item">
            <span class="tag-manager-item-name" title="${escapeHtml(item.path)}">${escapeHtml(name)}</span>
            <button class="btn-small" data-tag-action="remove-from" data-tag="${escapeHtml(tag)}" data-path="${escapeHtml(item.path)}" style="padding:2px 6px;font-size:9px;border:1px solid var(--border);background:transparent;color:var(--text-muted);cursor:pointer;" title="Remove this tag from item">&#10005;</button>
          </div>`;
        }).join('')}
        ${items.length > 20 ? `<div style="color:var(--text-muted);font-size:11px;padding:4px 8px;">...and ${items.length - 20} more</div>` : ''}
      </div>
    </div>`;
  }).join('');

  container.innerHTML = html;
}

// ── Tag Wizard Modal ──

let _tagWizardRenaming = null; // tag name being renamed, or null

function createNewTag() {
  showTagWizard();
}

function showTagWizard() {
  let existing = document.getElementById('tagWizardModal');
  if (existing) existing.remove();
  _tagWizardRenaming = null;

  const html = `<div class="modal-overlay" id="tagWizardModal" data-action-modal="closeTagWizard">
    <div class="modal-content modal-wide">
      <div class="modal-header">
        <h2>Tag Manager</h2>
        <button class="modal-close" data-action-modal="closeTagWizard">&#10005;</button>
      </div>
      <div class="modal-body">
        <div class="tag-wizard-add">
          <input type="text" id="tagWizardInput" placeholder="New tag name..." autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false">
          <button class="btn btn-primary" id="tagWizardAddBtn" data-action-tw="add">+ Add</button>
        </div>
        <div class="tag-wizard-list" id="tagWizardList"></div>
      </div>
    </div>
  </div>`;
  document.body.insertAdjacentHTML('beforeend', html);
  renderTagWizardList();
  document.getElementById('tagWizardInput').focus();
}

function closeTagWizard() {
  const modal = document.getElementById('tagWizardModal');
  if (modal) modal.remove();
  _tagWizardRenaming = null;
  renderTagsManager();
  renderGlobalTagBar();
}

function renderTagWizardList() {
  const list = document.getElementById('tagWizardList');
  if (!list) return;

  const tagCounts = getTagCounts();
  const allTags = Object.keys(tagCounts).sort();

  if (allTags.length === 0) {
    list.innerHTML = '<div class="tag-wizard-empty">No tags yet. Type a name above and click Add.</div>';
    return;
  }

  list.innerHTML = allTags.map(tag => {
    const count = tagCounts[tag];
    const isRenaming = _tagWizardRenaming === tag;
    const nameHtml = isRenaming
      ? `<input type="text" class="tag-wizard-rename-input" data-tw-rename-tag="${escapeHtml(tag)}" value="${escapeHtml(tag)}" autofocus autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false">`
      : escapeHtml(tag);
    return `<div class="tag-wizard-row">
      <span class="tag-wizard-name">${nameHtml}</span>
      <span class="tag-wizard-count">${count} item${count !== 1 ? 's' : ''}</span>
      <div class="tag-wizard-actions">
        ${isRenaming
          ? `<button class="btn-small btn-primary" data-action-tw="confirmRename" data-tag="${escapeHtml(tag)}" style="padding:3px 8px;font-size:10px;" title="Save new tag name">Save</button>
             <button class="btn-small btn-secondary" data-action-tw="cancelRename" style="padding:3px 8px;font-size:10px;" title="Cancel rename">Cancel</button>`
          : `<button class="btn-small btn-secondary" data-action-tw="startRename" data-tag="${escapeHtml(tag)}" style="padding:3px 8px;font-size:10px;" title="Rename this tag">Rename</button>
             <button class="btn-small btn-stop" data-action-tw="delete" data-tag="${escapeHtml(tag)}" style="padding:3px 8px;font-size:10px;" title="Delete this tag permanently">Delete</button>`
        }
      </div>
    </div>`;
  }).join('');

  // Focus rename input if active
  if (_tagWizardRenaming) {
    const inp = list.querySelector('.tag-wizard-rename-input');
    if (inp) { inp.focus(); inp.select(); }
  }
}

function tagWizardAdd() {
  const input = document.getElementById('tagWizardInput');
  if (!input) return;
  const name = input.value.trim();
  if (!name) return;

  const existing = getAllTags();
  if (existing.includes(name)) {
    showToast(`Tag "${name}" already exists`);
    return;
  }

  const standalone = getStandaloneTags();
  standalone.push(name);
  setStandaloneTags(standalone);
  input.value = '';
  renderTagWizardList();
  showToast(`Tag "${name}" created`);
}

function tagWizardDelete(tag) {
  if (!confirm(`Delete tag "${tag}" from all items?`)) return;
  deleteTag(tag);
  renderTagWizardList();
}

function tagWizardStartRename(tag) {
  _tagWizardRenaming = tag;
  renderTagWizardList();
}

function tagWizardCancelRename() {
  _tagWizardRenaming = null;
  renderTagWizardList();
}

function tagWizardConfirmRename(oldTag) {
  const input = document.querySelector(`.tag-wizard-rename-input[data-tw-rename-tag="${CSS.escape(oldTag)}"]`);
  if (!input) return;
  const newName = input.value.trim();
  if (!newName || newName === oldTag) {
    tagWizardCancelRename();
    return;
  }
  const existing = getAllTags();
  if (existing.includes(newName)) {
    showToast(`Tag "${newName}" already exists`);
    return;
  }
  renameTag(oldTag, newName);
  // Also rename in standalone tags
  const standalone = getStandaloneTags();
  const idx = standalone.indexOf(oldTag);
  if (idx !== -1) {
    standalone[idx] = newName;
    setStandaloneTags(standalone);
  }
  _tagWizardRenaming = null;
  renderTagWizardList();
}

// Event delegation for tag wizard
document.addEventListener('click', (e) => {
  // Check tag wizard actions first (before modal close)
  const tw = e.target.closest('[data-action-tw]');
  if (tw) {
    const act = tw.dataset.actionTw;
    const tag = tw.dataset.tag;
    if (act === 'add') tagWizardAdd();
    else if (act === 'delete') tagWizardDelete(tag);
    else if (act === 'startRename') tagWizardStartRename(tag);
    else if (act === 'cancelRename') tagWizardCancelRename();
    else if (act === 'confirmRename') tagWizardConfirmRename(tag);
    return;
  }

  // Close modal on overlay/close button click
  const closeAction = e.target.closest('[data-action-modal="closeTagWizard"]');
  if (closeAction) {
    if (e.target === closeAction || e.target.classList.contains('modal-close')) {
      closeTagWizard();
    }
  }
});

// Enter key in add input or rename input
document.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && e.target.id === 'tagWizardInput') {
    e.preventDefault();
    tagWizardAdd();
    return;
  }
  if (e.key === 'Enter' && e.target.classList.contains('tag-wizard-rename-input')) {
    e.preventDefault();
    const oldTag = e.target.dataset.twRenameTag;
    tagWizardConfirmRename(oldTag);
    return;
  }
  if (e.key === 'Escape' && _tagWizardRenaming) {
    tagWizardCancelRename();
    return;
  }
  if (e.key === 'Escape' && document.getElementById('tagWizardModal')) {
    closeTagWizard();
  }
});

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

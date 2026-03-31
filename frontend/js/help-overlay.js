// ── Help Overlay ──
// Press ? to show keyboard shortcuts reference

function toggleHelpOverlay() {
  let existing = document.getElementById('helpOverlay');
  if (existing) { existing.remove(); return; }

  const html = `<div class="modal-overlay" id="helpOverlay" data-action-modal="closeHelp">
    <div class="modal-content">
      <div class="modal-header">
        <h2>Keyboard Shortcuts</h2>
        <button class="modal-close" data-action-modal="closeHelp">&#10005;</button>
      </div>
      <div class="modal-body">
        <div class="help-grid">
          <div class="help-section">
            <h3>Navigation</h3>
            <div class="help-row"><kbd>&#8984;1</kbd>-<kbd>&#8984;7</kbd> <span>Switch tabs</span></div>
            <div class="help-row"><kbd>&#8984;F</kbd> <span>Focus search</span></div>
            <div class="help-row"><kbd>&#8593;</kbd> / <kbd>k</kbd> <span>Previous item</span></div>
            <div class="help-row"><kbd>&#8595;</kbd> / <kbd>j</kbd> <span>Next item</span></div>
            <div class="help-row"><kbd>Home</kbd> <span>First item</span></div>
            <div class="help-row"><kbd>End</kbd> <span>Last item</span></div>
            <div class="help-row"><kbd>Enter</kbd> <span>Open / activate item</span></div>
            <div class="help-row"><kbd>Space</kbd> <span>Play / pause sample</span></div>
          </div>
          <div class="help-section">
            <h3>Actions</h3>
            <div class="help-row"><kbd>Esc</kbd> <span>Clear search / close / stop</span></div>
            <div class="help-row"><kbd>?</kbd> <span>Toggle this help</span></div>
          </div>
          <div class="help-section">
            <h3>Search Operators (fzf)</h3>
            <div class="help-row"><code>term</code> <span>Fuzzy match</span></div>
            <div class="help-row"><code>'exact</code> <span>Exact substring</span></div>
            <div class="help-row"><code>^prefix</code> <span>Starts with</span></div>
            <div class="help-row"><code>suffix$</code> <span>Ends with</span></div>
            <div class="help-row"><code>!term</code> <span>Exclude</span></div>
            <div class="help-row"><code>a | b</code> <span>OR match</span></div>
            <div class="help-row"><code>.*</code> <span>Toggle regex mode</span></div>
          </div>
          <div class="help-section">
            <h3>Mouse</h3>
            <div class="help-row"><span style="color:var(--cyan);">Click</span> <span>Play sample / expand metadata</span></div>
            <div class="help-row"><span style="color:var(--cyan);">Double-click</span> <span>Open in DAW / KVR / Finder</span></div>
            <div class="help-row"><span style="color:var(--cyan);">Right-click</span> <span>Context menu everywhere</span></div>
            <div class="help-row"><span style="color:var(--cyan);">Drag tabs</span> <span>Reorder tabs</span></div>
            <div class="help-row"><span style="color:var(--cyan);">Drag player</span> <span>Dock to any corner</span></div>
          </div>
        </div>
      </div>
    </div>
  </div>`;
  document.body.insertAdjacentHTML('beforeend', html);
}

// Event delegation for help modal
document.addEventListener('click', (e) => {
  const action = e.target.closest('[data-action-modal="closeHelp"]');
  if (action) {
    if (e.target === action || action.classList.contains('modal-close')) {
      const overlay = document.getElementById('helpOverlay');
      if (overlay) overlay.remove();
    }
  }
});

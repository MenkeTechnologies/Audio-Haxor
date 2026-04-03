// ── Help Overlay ──
// Press ? to show keyboard shortcuts reference

function toggleHelpOverlay() {
  let existing = document.getElementById('helpOverlay');
  if (existing) { existing.remove(); return; }

  const html = `<div class="modal-overlay" id="helpOverlay" data-action-modal="closeHelp">
    <div class="modal-content">
      <div class="modal-header">
        <h2>Keyboard Shortcuts</h2>
        <button class="modal-close" data-action-modal="closeHelp" title="Close">&#10005;</button>
      </div>
      <div class="modal-body">
        <div class="help-grid">
          <div class="help-section">
            <h3>Navigation</h3>
            <div class="help-row"><kbd>&#8984;1</kbd>-<kbd>&#8984;8</kbd> <span>Switch tabs</span></div>
            <div class="help-row"><kbd>&#8984;K</kbd> <span>Command palette</span></div>
            <div class="help-row"><kbd>&#8984;F</kbd> <span>Focus search</span></div>
            <div class="help-row"><kbd>j</kbd> / <kbd>&#8595;</kbd> <span>Next item</span></div>
            <div class="help-row"><kbd>k</kbd> / <kbd>&#8593;</kbd> <span>Previous item</span></div>
            <div class="help-row"><kbd>gg</kbd> <span>First item</span></div>
            <div class="help-row"><kbd>G</kbd> <span>Last item</span></div>
            <div class="help-row"><kbd>Ctrl+D</kbd> <span>Half-page down</span></div>
            <div class="help-row"><kbd>Ctrl+U</kbd> <span>Half-page up</span></div>
            <div class="help-row"><kbd>/</kbd> <span>Focus search</span></div>
            <div class="help-row"><kbd>Enter</kbd> <span>Open / activate item</span></div>
            <div class="help-row"><kbd>o</kbd> <span>Reveal in Finder</span></div>
            <div class="help-row"><kbd>y</kbd> <span>Yank (copy path)</span></div>
            <div class="help-row"><kbd>p</kbd> <span>Play / preview</span></div>
            <div class="help-row"><kbd>x</kbd> <span>Toggle favorite</span></div>
            <div class="help-row"><kbd>v</kbd> <span>Toggle select</span></div>
            <div class="help-row"><kbd>V</kbd> <span>Select all</span></div>
          </div>
          <div class="help-section">
            <h3>Playback</h3>
            <div class="help-row"><kbd>Space</kbd> <span>Play / pause</span></div>
            <div class="help-row"><kbd>&#8984;&#8594;</kbd> <span>Next track</span></div>
            <div class="help-row"><kbd>&#8984;&#8592;</kbd> <span>Previous track</span></div>
            <div class="help-row"><kbd>L</kbd> <span>Toggle loop</span></div>
            <div class="help-row"><kbd>M</kbd> <span>Mute / unmute</span></div>
            <div class="help-row"><kbd>&#8984;&#8593;</kbd> <span>Volume up</span></div>
            <div class="help-row"><kbd>&#8984;&#8595;</kbd> <span>Volume down</span></div>
          </div>
          <div class="help-section">
            <h3>Actions</h3>
            <div class="help-row"><kbd>&#8984;S</kbd> <span>Scan all</span></div>
            <div class="help-row"><kbd>&#8984;.</kbd> <span>Stop all scans</span></div>
            <div class="help-row"><kbd>&#8984;A</kbd> <span>Select all visible</span></div>
            <div class="help-row"><kbd>&#8984;E</kbd> <span>Export current tab</span></div>
            <div class="help-row"><kbd>&#8984;I</kbd> <span>Import to current tab</span></div>
            <div class="help-row"><kbd>&#8984;D</kbd> <span>Find duplicates</span></div>
            <div class="help-row"><kbd>&#8984;G</kbd> <span>Dependency graph</span></div>
            <div class="help-row"><kbd>&#8984;T</kbd> <span>Toggle theme</span></div>
            <div class="help-row"><kbd>&#8984;,</kbd> <span>Open preferences file</span></div>
            <div class="help-row"><kbd>Tab</kbd> <span>Next tab</span></div>
            <div class="help-row"><kbd>Shift+Tab</kbd> <span>Previous tab</span></div>
            <div class="help-row"><kbd>R</kbd> <span>Reveal in Finder</span></div>
            <div class="help-row"><kbd>C</kbd> <span>Copy path</span></div>
            <div class="help-row"><kbd>F</kbd> <span>Toggle favorite</span></div>
            <div class="help-row"><kbd>N</kbd> <span>Add note</span></div>
            <div class="help-row"><kbd>S</kbd> <span>Toggle shuffle</span></div>
            <div class="help-row"><kbd>W</kbd> <span>Find similar samples</span></div>
            <div class="help-row"><kbd>Del</kbd> <span>Delete selected</span></div>
            <div class="help-row"><kbd>Esc</kbd> <span>Close / clear / stop</span></div>
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

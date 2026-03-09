<script lang="ts">
</script>

<div class="shortcuts-panel">
  <div class="panel-header">
    <span>Shortcuts</span>
  </div>
  <div class="shortcuts-list">
    <div class="empty-state">
      <p>No shortcuts configured.</p>
      <p class="hint">Touch actions from KLWP imports will appear here once the shortcut system is wired up.</p>
    </div>
  </div>
</div>

<style>
  .shortcuts-panel {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex: 1;
  }
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    font-weight: 600;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border);
    user-select: none;
  }
  .shortcuts-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  .empty-state {
    padding: 32px 16px;
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.6;
  }
  .hint {
    font-size: 11px;
    opacity: 0.7;
    margin-top: 4px;
  }
</style>

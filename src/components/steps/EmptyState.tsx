export function EmptyState() {
  return (
    <div className="empty-state">
      <div className="empty-icon" aria-hidden>
        <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
          <rect x="8" y="6" width="32" height="38" rx="4" stroke="currentColor" strokeWidth="2" opacity="0.35" />
          <circle cx="18" cy="18" r="3" fill="currentColor" opacity="0.5" />
          <rect x="24" y="16" width="12" height="4" rx="2" fill="currentColor" opacity="0.25" />
          <circle cx="18" cy="28" r="3" fill="currentColor" opacity="0.35" />
          <rect x="24" y="26" width="10" height="4" rx="2" fill="currentColor" opacity="0.2" />
          <circle cx="18" cy="38" r="3" fill="currentColor" opacity="0.2" />
          <rect x="24" y="36" width="8" height="4" rx="2" fill="currentColor" opacity="0.15" />
        </svg>
      </div>
      <h2>暂无录制步骤</h2>
      <p>点击左侧「开始录制」，应用会最小化并显示悬浮控制栏，桌面上的鼠标点击将自动记录为图文步骤。</p>
    </div>
  );
}

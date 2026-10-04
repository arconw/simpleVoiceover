import EffectsPanel from '../EffectsPanel'
import FilesPanel from './FilesPanel'
import MixerPanel from './MixerPanel'
import type { StudioController } from '../useStudio'

export default function Inspector(panel: StudioController) {
  const { tracks, selectedTrackId, setSelectedTrackId, tab, setTab, selectedTrack, updateTrack } =
    panel
  return (
    <aside className="inspector">
      <div className="inspector-tabs" role="tablist" aria-label="Панели студии">
        {[
          ['files', 'Файлы'],
          ['mixer', 'Микшер'],
          ['effects', 'Эффекты'],
        ].map(([id, label]) => (
          <button key={id} role="tab" aria-selected={tab === id} onClick={() => setTab(id)}>
            {label}
          </button>
        ))}
        <button
          className="empty-tab"
          disabled
          title="Резервная вкладка"
          aria-label="Резервная вкладка"
        />
      </div>
      <div className="inspector-body" role="tabpanel">
        {tab === 'files' && <FilesPanel {...panel} />}
        {tab === 'mixer' && <MixerPanel {...panel} />}
        {tab === 'effects' && (
          <>
            <div className="selected-track">
              <span className="eyebrow">ДОРОЖКА</span>
              <select
                aria-label="Дорожка для эффектов"
                value={selectedTrackId}
                onChange={(e) => setSelectedTrackId(e.target.value)}
              >
                {tracks.map((t) => (
                  <option value={t.id} key={t.id}>
                    {t.name}
                  </option>
                ))}
              </select>
              <span className="track-color" style={{ background: selectedTrack.color }} />
            </div>
            <EffectsPanel
              track={selectedTrack}
              onChange={(patch) => updateTrack(selectedTrack.id, patch)}
            />
          </>
        )}
      </div>
      <div className="inspector-foot">
        <span className="small-dot" />
        Локальная студия · Rust
      </div>
    </aside>
  )
}

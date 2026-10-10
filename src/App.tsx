import { t } from './i18n'
import { Upload } from 'lucide-react'
import Timeline from './Timeline'
import { useStudio } from './useStudio'
import StudioHeader from './components/StudioHeader'
import VideoPreview from './components/VideoPreview'
import Transport from './components/Transport'
import StatusBar from './components/StatusBar'
import Inspector from './components/Inspector'
import HelpDialog from './components/HelpDialog'
import CloseDialog from './components/CloseDialog'
import Notice from './components/Notice'
import WindowControls from './components/WindowControls'
import PreviewInfo from './components/PreviewInfo'
import SettingsDialog from './components/SettingsDialog'
import RemoveTrackDialog from './components/RemoveTrackDialog'
import { translateMessage } from './i18n'

export default function App() {
  const studio = useStudio()
  if (!studio.snapshot)
    return (
      <main className="studio connecting" onContextMenu={(event) => event.preventDefault()}>
        <WindowControls {...studio} />
        <h1>simpleVoiceover</h1>
        <p>{studio.error ? translateMessage(studio.error) : t('studio.connecting')}</p>
        {studio.confirmMode && <CloseDialog {...studio} />}
      </main>
    )
  return (
    <main
      className={`studio ${studio.draggingFiles ? 'file-drag' : ''}`}
      onContextMenu={(event) => event.preventDefault()}
    >
      <StudioHeader {...studio} />
      <WindowControls {...studio} />
      <section className="workspace-top">
        <Inspector {...studio} />
        <VideoPreview {...studio} />
      </section>
      <Timeline
        transport={<Transport {...studio} preview={<PreviewInfo {...studio} />} />}
        tracks={studio.tracks}
        assets={studio.assets}
        selectedTrackId={studio.selectedTrackId}
        selectedClipId={studio.selectedClipId}
        selection={studio.selection}
        onSelectRegion={studio.setSelection}
        onMoveRegions={studio.moveRegions}
        onCopy={studio.copyClips}
        onPaste={() => void studio.pasteClips()}
        canPaste={studio.canPaste}
        onSelectTrack={studio.setSelectedTrackId}
        onSelectClip={studio.setSelectedClipId}
        onUpdateTrack={studio.updateTrack}
        onEditClip={studio.editClip}
        onSplitClip={studio.splitClip}
        onSeek={studio.seek}
        position={studio.position}
        duration={studio.duration}
        recording={studio.recording}
        operationPending={studio.operationPending}
        recordStart={studio.recordStart}
        tool={studio.tool}
        onToolChange={studio.setTool}
        snapping={studio.snapping}
        onSnappingChange={studio.setSnapping}
        onRemoveTrack={studio.requestRemoveTrack}
        onAddTrack={studio.addTrack}
        onRemoveClip={studio.removeClip}
        getLevel={studio.getLevel}
      />
      <StatusBar {...studio} />
      <Notice {...studio} />
      {studio.draggingFiles && (
        <div className="drop-overlay">
          <Upload size={40} />
          <h2>{t('drop.release')}</h2>
          <p>{t('drop.description')}</p>
        </div>
      )}
      {studio.help && <HelpDialog {...studio} />}
      {studio.settings && <SettingsDialog {...studio} />}
      {studio.confirmMode && <CloseDialog {...studio} />}
      {studio.trackToRemove && <RemoveTrackDialog {...studio} />}
    </main>
  )
}

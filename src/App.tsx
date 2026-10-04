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

export default function App() {
  const studio = useStudio()
  if (!studio.selectedTrack)
    return (
      <main className="studio connecting">
        <h1>simpleVoiceover</h1>
        <p>{studio.error || 'Подключаю локальную студию…'}</p>
      </main>
    )
  return (
    <main className={`studio ${studio.draggingFiles ? 'file-drag' : ''}`}>
      <StudioHeader {...studio} />
      <section className="workspace-top">
        <Inspector {...studio} />
        <VideoPreview {...studio} />
      </section>
      <Transport {...studio} />
      <Timeline
        tracks={studio.tracks}
        assets={studio.assets}
        selectedTrackId={studio.selectedTrackId}
        selectedClipId={studio.selectedClipId}
        onSelectTrack={studio.setSelectedTrackId}
        onSelectClip={studio.setSelectedClipId}
        onUpdateTrack={studio.updateTrack}
        onEditClip={studio.editClip}
        onSplitClip={studio.splitClip}
        onSeek={studio.seek}
        position={studio.position}
        duration={studio.duration}
        recording={studio.recording}
        recordStart={studio.recordStart}
        tool={studio.tool}
        onToolChange={studio.setTool}
        onAddTrack={studio.addTrack}
        onRemoveClip={studio.removeClip}
        getLevel={studio.getLevel}
      />
      <StatusBar {...studio} />
      <Notice {...studio} />
      {studio.draggingFiles && (
        <div className="drop-overlay">
          <Upload size={40} />
          <h2>Отпусти файлы здесь</h2>
          <p>Видео и звук появятся на дорожках</p>
        </div>
      )}
      {studio.help && <HelpDialog {...studio} />}
      {studio.confirmMode && <CloseDialog {...studio} />}
    </main>
  )
}

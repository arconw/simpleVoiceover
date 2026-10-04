import { Check, FileAudio, FileVideo, FolderOpen, Plus, Upload } from 'lucide-react'
import { formatTime } from '../types'
import type { StudioController } from '../useStudio'

type Props = Pick<
  StudioController,
  | 'assets'
  | 'recording'
  | 'busy'
  | 'saveProject'
  | 'placeAsset'
  | 'chooseWorkingDirectory'
  | 'importNative'
  | 'snapshot'
>

export default function FilesPanel({
  assets,
  recording,
  busy,
  saveProject,
  placeAsset,
  chooseWorkingDirectory,
  importNative,
  snapshot,
}: Props) {
  return (
    <div className="files-panel">
      <div className="panel-heading">
        <span className="eyebrow">МЕДИА ПРОЕКТА</span>
        <span className="counter">{assets.length}</span>
      </div>
      <button
        className="drop-zone"
        disabled={recording || !!busy}
        onClick={() => void importNative()}
      >
        <span className="upload-icon">
          <Upload size={22} />
        </span>
        <strong>Добавь видео или аудио</strong>
        <span>Выбери файл или перетащи его сюда</span>
        <small>MP4/AAC, WAV, MP3, FLAC · до 100 ГБ</small>
      </button>
      {assets.length ? (
        <div className="asset-list">
          {assets.map((asset) => (
            <div className="asset-item" key={asset.id}>
              <span className={`asset-icon ${asset.kind}`}>
                {asset.kind === 'video' ? <FileVideo size={19} /> : <FileAudio size={19} />}
              </span>
              <div>
                <strong title={asset.name}>{asset.name}</strong>
                <small>
                  {formatTime(asset.duration)} · {`${asset.sampleRate / 1000} кГц`}
                </small>
              </div>
              <button
                title="Добавить файл на выбранную дорожку в позицию курсора"
                aria-label={`Добавить ${asset.name} на дорожку`}
                disabled={recording || !!busy}
                onClick={() => placeAsset(asset)}
              >
                <Plus size={15} />
              </button>
            </div>
          ))}
        </div>
      ) : (
        <div className="files-empty">
          <span className="empty-file-lines">
            <i />
            <i />
            <i />
          </span>
          <p>
            Здесь будут твои исходники
            <br />и записанные дубли
          </p>
        </div>
      )}
      <div className="workspace-settings">
        <button
          className="button secondary"
          disabled={recording || !!busy}
          onClick={() => void importNative()}
        >
          <FolderOpen size={15} />
          Открыть файл с диска
        </button>
        <button
          className="text-button"
          disabled={recording || !!busy}
          onClick={() => void chooseWorkingDirectory()}
        >
          <FolderOpen size={14} /> Рабочий каталог
        </button>
        <small title={snapshot?.config.workingDirectory}>{snapshot?.config.workingDirectory}</small>
        <p className="hint muted">
          {snapshot?.config.projectFile
            ? `Проект: ${snapshot.config.projectFile}. Кэш изменений появится рядом; Ctrl+S упакует и уберёт его.`
            : 'Здесь хранится кэш несохранённого проекта. Ctrl+S упакует его в файл.'}
        </p>
        <button
          className="text-button"
          disabled={recording || !!busy}
          onClick={() => void saveProject(true)}
        >
          Сохранить как…
        </button>
      </div>
      <div className="local-note">
        <Check size={14} />
        <span>Медиа остаются на этом устройстве</span>
      </div>
    </div>
  )
}

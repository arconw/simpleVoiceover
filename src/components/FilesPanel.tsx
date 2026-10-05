import { t } from '../i18n'
import { Check, FileAudio, FileVideo, FolderOpen, Plus, Trash2, Upload } from 'lucide-react'
import { formatTime } from '../types'
import type { StudioController } from '../useStudio'

type Props = Pick<
  StudioController,
  | 'assets'
  | 'recording'
  | 'busy'
  | 'saveProject'
  | 'placeAsset'
  | 'removeAsset'
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
  removeAsset,
  chooseWorkingDirectory,
  importNative,
  snapshot,
}: Props) {
  return (
    <div className="files-panel">
      <div className="panel-heading">
        <span className="eyebrow">{t('files.heading')}</span>
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
        <strong>{t('files.addTitle')}</strong>
        <span>{t('files.addHint')}</span>
        <small>{t('files.formats')}</small>
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
                  {formatTime(asset.duration)} ·{' '}
                  {t('units.khzValue', { value0: asset.sampleRate / 1000 })}
                </small>
              </div>
              <button
                title={t('files.placeTitle')}
                aria-label={t('files.placeLabel', { value0: asset.name })}
                disabled={recording || !!busy}
                onClick={() => placeAsset(asset)}
              >
                <Plus size={15} />
              </button>
              <button
                title={t('files.removeTitle')}
                aria-label={t('files.removeLabel', { value0: asset.name })}
                disabled={recording || !!busy}
                onClick={() => removeAsset(asset)}
              >
                <Trash2 size={15} />
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
            {t('files.emptySources')} <br />
            {t('files.emptyTakes')}{' '}
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
          {t('files.openDisk')}{' '}
        </button>
        <button
          className="text-button"
          disabled={recording || !!busy}
          onClick={() => void chooseWorkingDirectory()}
        >
          <FolderOpen size={14} /> {t('project.workingDirectory')}{' '}
        </button>
        <small title={snapshot?.config.workingDirectory}>{snapshot?.config.workingDirectory}</small>
        <p className="hint muted">
          {snapshot?.config.projectFile
            ? t('files.savedCache', { value0: snapshot.config.projectFile })
            : t('files.unsavedCache')}
        </p>
        <button
          className="text-button"
          disabled={recording || !!busy}
          onClick={() => void saveProject(true)}
        >
          {t('project.saveAs')}{' '}
        </button>
      </div>
      <div className="local-note">
        <Check size={14} />
        <span>{t('files.localOnly')}</span>
      </div>
    </div>
  )
}

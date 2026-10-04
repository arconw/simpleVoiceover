import { X } from 'lucide-react'
import type { StudioController } from '../useStudio'

type Props = Pick<StudioController, 'setHelp'>

export default function HelpDialog({ setHelp }: Props) {
  return (
    <div className="modal-backdrop" onClick={() => setHelp(false)}>
      <section
        className="help-modal"
        role="dialog"
        aria-modal="true"
        aria-label="Как пользоваться студией"
        onClick={(e) => e.stopPropagation()}
      >
        <button className="modal-close" aria-label="Закрыть справку" onClick={() => setHelp(false)}>
          <X size={18} />
        </button>
        <span className="eyebrow">SIMPLEVOICEOVER · БЫСТРЫЙ СТАРТ</span>
        <h2>От видео до готового войса</h2>
        <ol>
          <li>
            <strong>Добавь видео.</strong> Его звук появится отдельной дорожкой.
          </li>
          <li>
            <strong>Нажми R на дорожке голоса.</strong> Запись идёт только в одну дорожку. Включи
            наушники для мониторинга.
          </li>
          <li>
            <strong>Поставь курсор и нажми «Запись».</strong> Видео и остальные дорожки будут
            звучать вместе с микрофоном.
          </li>
          <li>
            <strong>Выбери дорожку и открой «Эффекты».</strong> Пресет регулируется; исходник
            остаётся нетронутым.
          </li>
          <li>
            <strong>Сохрани проект или экспортируй WAV / MP3.</strong> Проект включает все исходники
            и монтаж. Каждая дорожка экспортируется отдельно из микшера.
          </li>
        </ol>
        <div className="shortcut-grid">
          <span>
            <kbd>Space</kbd>Пуск / пауза
          </span>
          <span>
            <kbd>R</kbd>Запись / завершить
          </span>
          <span>
            <kbd>V</kbd>Стрелка
          </span>
          <span>
            <kbd>X</kbd>Ножницы
          </span>
          <span>
            <kbd>Delete</kbd>Убрать клип
          </span>
          <span>
            <kbd>Ctrl Z</kbd>Отменить
          </span>
          <span>
            <kbd>Ctrl Shift Z</kbd>Повторить
          </span>
          <span>
            <kbd>Ctrl S</kbd>Сохранить и очистить историю
          </span>
        </div>
        <p className="muted hint">
          Перетаскивай клипы и их края для монтажа. Края полосы прокрутки меняют масштаб. Lock
          защищает монтаж клипов, Mute заглушает только воспроизведение. При закрытии окна
          приложение предложит сохранить .justspeak. Экспорт — аудио; исходное видео хранится в
          проекте.
        </p>
      </section>
    </div>
  )
}

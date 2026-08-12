# Meetily speaker diarization: synthesis

**Status:** COMPLETE
**Last updated:** 2026-08-12

## Executive summary

Мы не изобрели функцию с нуля, но и готового решения, которое можно просто забрать из Community `main`, нет.

- В актуальном исходном коде [`Zackriya-Solutions/meetily`](https://github.com/Zackriya-Solutions/meetily) полноценной многоспикерной диаризации нет. Есть только старый stereo-energy режим whisper.cpp и TinyDiarize; это не замена кластеризации нескольких людей в общем аудиоканале ([server.cpp](https://github.com/Zackriya-Solutions/meetily/blob/main/backend/whisper-custom/server/server.cpp)).
- Описание репозитория и продуктовая документация уже рекламируют beta-диаризацию. Это относится к продуктовой/PRO-линии или к коду, который пока не присутствует в Community `main`. Поэтому маркетинговое описание и документацию нельзя считать доказательством доступной OSS-реализации.
- Самый полезный публичный форк — [`TylerBuza/Meetily-ActuallyFree`](https://github.com/TylerBuza/Meetily-ActuallyFree). Там уже сделаны локальная диаризация, автоматический запуск после окончания записи, обновление UI, переименование спикеров, live labels и voiceprints.
- Наш текущий вариант на `sherpa-onnx` не совпадает с этим форком и остаётся хорошей базой: в нём меньше собственного ML/DSP-кода, уже есть безопасная загрузка моделей, защита от повторного запуска для одной встречи, восстановление статуса, сохранение имён и транзакционная замена результатов.
- Лучший путь — не заменять нашу реализацию, а перенести несколько оркестрационных идей: настройку «включено по умолчанию», автоматическую постановку в очередь после сохранения финальной транскрипции, глобальное ограничение тяжёлых задач и уведомление интерфейса о стадиях/завершении.

## Что уже сделали другие

### Community upstream

Полноценной реализации общего случая нет. Старый `--diarize` в whisper.cpp сравнивает энергию двух стереоканалов, то есть может приблизительно разделить «микрофон / системный звук», но не нескольких собеседников в одном канале. TinyDiarize распознаёт смену говорящего и требует специальную модель, но не даёт устойчивую глобальную кластеризацию спикеров ([исходник](https://github.com/Zackriya-Solutions/meetily/blob/main/backend/whisper-custom/server/server.cpp)). Запрос на идентификацию известных участников [#56](https://github.com/Zackriya-Solutions/meetily/issues/56) всё ещё открыт.

При этом продуктовый workflow уже спроектирован разумно: настройка включается до обработки, затем после сохранения транскрипта автоматически запускается speaker identification как отдельная фоновая стадия; импорт и ретранскрибация идут через очередь с прогрессом и отменой ([Meeting Workspace](https://docs.meetily.ai/features/meeting-workspace), [Retranscription](https://docs.meetily.ai/features/retranscription), [Audio Import](https://docs.meetily.ai/features/audio-import)). Это полезный UX-прецедент, но не готовый Community-код.

### Meetily-ActuallyFree

Форк действительно реализует то, о чём мы говорим:

- локальный pyannote segmentation + WeSpeaker embeddings + собственный agglomerative clustering ([первый commit](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/729ec67c1c96));
- автоматический fire-and-forget запуск после сохранения встречи и событие для обновления открытого экрана ([auto-run commit](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/e1e08ffd3dc0));
- ожидаемое число спикеров, калибровка, именование локального пользователя, ручное переименование и дальнейшая работа с live labels/voiceprints ([история реализации](https://github.com/TylerBuza/Meetily-ActuallyFree/commits/main/));
- процессный lock, `spawn_blocking`, транзакционные записи, обработка overlap и раздельных mic/system дорожек в текущем [`diarization/mod.rs`](https://github.com/TylerBuza/Meetily-ActuallyFree/blob/main/frontend/src-tauri/src/diarization/mod.rs).

Но автоматизация там минимальная: фиксированная задержка в две секунды, нет долговечной очереди, retry/cancel, реального прогресса и явного лимита потоков. Текущие ONNX-сессии диаризации используют CPU и не задают `intra_op`/`inter_op` thread limits ([models.rs](https://github.com/TylerBuza/Meetily-ActuallyFree/blob/main/frontend/src-tauri/src/diarization/models.rs)). Поэтому заявление форка о CPU/Vulkan/CUDA не означает GPU-ускорение именно диаризации.

### Предлагаемые единые ASR + diarization модели

Запросы на MOSS-Transcribe-Diarize [#659](https://github.com/Zackriya-Solutions/meetily/issues/659) и VibeVoice-ASR [#335](https://github.com/Zackriya-Solutions/meetily/issues/335) предлагают один проход вместо двух. Это пока идеи, а не переносимый код; авторы отдельно отмечают требования к VRAM/вычислениям и необходимость бенчмарков. Для текущей задачи они слишком тяжёлые и рискованные.

## Что безопасно перенести в наш форк

1. **Автозапуск после финального сохранения транскрипта.** Точка запуска у нас надёжнее, чем `sleep(2s)`: запись уже flush/save, транскрипция завершена, после чего встреча ставится в background queue.
2. **Глобальный guard/очередь тяжёлых inference-задач.** Текущий `HashSet` защищает одну встречу от дублей, но не мешает двум встречам или разным моделям одновременно забить CPU и память. Нужен общий semaphore/worker, обычно с concurrency `1`.
3. **События состояния для UI.** Минимум: `queued`, `segmenting`, `embedding`, `clustering`, `saving`, `complete`/`failed`. Даже без честного процента пользователь увидит, что пятиминутная задача не зависла.
4. **Completion notification и refetch.** После успешной транзакции обновлять открытую встречу автоматически. Идею можно перенести, но лучше через уже используемый Tauri event/state слой, а не случайное browser custom event.
5. **Разделение diarization и identity.** Сначала устойчивые `Speaker 1/2`, затем отдельный слой ручных имён/voiceprints. Это соответствует и upstream roadmap, и нашей текущей модели данных.
6. **Отдельная обработка mic/system источников.** Сохранить известного локального пользователя и не перетирать его результатом кластеризации — у нас часть этого уже реализована.

## Что переносить не стоит

- Не заменять `sherpa-onnx` на собственные DSP, feature extraction, ONNX wrappers и agglomerative clustering из `Meetily-ActuallyFree`: это заметно увеличит объём кода, тестовую поверхность и стоимость дальнейших обновлений без доказанного выигрыша.
- Не копировать фиксированную двухсекундную задержку. Нужен явный сигнал «аудио и финальный транскрипт сохранены».
- Не запускать тяжёлую offline-диаризацию одновременно с live ASR по умолчанию. Это конкуренция за CPU, memory bandwidth и runtime threads; upstream также двигается к очереди и post-processing для слабого железа ([v0.3.0](https://github.com/Zackriya-Solutions/meetily/releases/tag/v0.3.0), [issue #519](https://github.com/Zackriya-Solutions/meetily/issues/519)).
- Не копировать CPU-only ONNX session setup форка как «ускорение»: явного thread cap или GPU provider для диаризации там нет.
- Не заменять многоспикерную модель на whisper.cpp stereo diarization: она отвечает на другую задачу.
- Не бандлить сторонние веса только потому, что это сделал другой форк, пока отдельно не проверены лицензии моделей, условия распространения и влияние на размер приложения.
- Не брать пороги кластеризации `0.60/0.65` вслепую: они зависят от модели, нормализации, аудио и алгоритма; для `sherpa-onnx` нужны свои тестовые записи и метрики.

## Конкретная рекомендация: автозапуск

Рекомендуемый pipeline:

`recording stopped → audio finalized → transcription finalized and saved → diarization queued → diarization result saved transactionally → UI refreshed`

- Для новых установок `speaker_diarization_enabled = true`; в Settings остаётся явный выключатель.
- При live transcription диаризация начинается только после stop/final save.
- При post-processing transcription она последовательно продолжает тот же job после ASR.
- Ошибка диаризации не должна отменять или портить готовую транскрипцию; состояние job становится `failed`, доступен retry.
- Ручная кнопка остаётся как `Run again`/восстановление после ошибки, а не как обязательный основной путь.
- Одновременно выполняется максимум одна тяжёлая локальная inference-задача, если только позже бенчмарки не докажут пользу параллелизма.

Именно последовательный вариант использует продуктовая документация Meetily: при ретранскрибации сначала сохраняется новый transcript, затем начинается speaker identification ([Retranscription](https://docs.meetily.ai/features/retranscription)). Публичный форк также выбрал post-recording автозапуск ([commit](https://github.com/TylerBuza/Meetily-ActuallyFree/commit/e1e08ffd3dc0)).

## Конкретная рекомендация: половина ядер

Базовая политика:

```rust
let logical_cores = std::thread::available_parallelism()
    .map(usize::from)
    .unwrap_or(2);
let inference_threads = std::cmp::max(1, logical_cores / 2);
```

Передать `inference_threads` в оба доступных thread-параметра `sherpa-onnx`: segmentation и speaker embedding. Эти стадии выполняются последовательно, поэтому одинаковый лимит не означает удвоение бюджета. Дополнительно ограничить число одновременно активных тяжёлых jobs глобальным semaphore/worker.

Это относительный, переносимый default: на 8 логических ядрах — 4 потока, на 10 — 5, на 2 — 1. Настройку можно позже вынести как `Eco / Balanced / Fast`, но первый релиз не нуждается в сложном scheduler.

Важно: ограничение только внутри diarization не спасёт от переподписки, если Whisper/Parakeet/llama.cpp параллельно создают собственные пулы. Поэтому очередь должна не допускать пересечения offline ASR, diarization и summarization, либо общий resource manager должен раздавать им один бюджет. У upstream уже есть прецедент относительного выбора потоков через `available_parallelism()` в [`llama-helper`](https://github.com/Zackriya-Solutions/meetily/blob/main/llama-helper/src/main.rs), хотя его формула `(cores / 2) + 2` агрессивнее нужной нам.

## Итоговое решение

Оставить нашу реализацию `sherpa-onnx` и сделать следующий небольшой слой поверх неё:

1. persisted default-on setting;
2. автоматический enqueue после успешной транскрипции/save;
3. global single-worker queue;
4. `max(1, logical_cores / 2)` для segmentation и embedding;
5. stage events, error/retry и автоматический UI refresh.

Так мы используем уже проверенный другими workflow, но не тащим их более хрупкий кастомный ML-стек. Это не «велосипед»: публичный форк подтвердил полезность автоматического post-processing, а наша текущая реализация закрывает его слабые места — целостность загрузок/записей, восстановление состояния и более компактный inference layer.

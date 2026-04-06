#[path = "common/mod.rs"]
mod common;

use clients::DownloadProgress;
use composer::progress::ProgressBridge;

use common::{CallCounter, init_test_tracing};

#[test]
fn noop_bridge_initial_state() {
    let _guard = init_test_tracing();

    // 1. Создаём noop bridge
    let bridge = ProgressBridge::noop();

    // 2. Проверяем начальное состояние
    assert_eq!(bridge.downloaded(), 0);
    assert_eq!(bridge.total(), None);
    assert_eq!(bridge.fraction(), None);
}

#[test]
fn sink_updates_bridge() {
    let _guard = init_test_tracing();

    // 1. Создаём bridge и получаем sink
    let bridge = ProgressBridge::noop();
    let sink = bridge.sink();

    // 2. Отправляем update через sink
    sink.update(DownloadProgress {
        downloaded: 42,
        total: Some(100),
    });

    // 3. Проверяем что snapshot отражает отправленные значения
    let snap = bridge.snapshot();
    assert_eq!(snap.downloaded, 42);
    assert_eq!(snap.total, Some(100));

    // 4. Проверяем отдельные геттеры
    assert_eq!(bridge.downloaded(), 42);
    assert_eq!(bridge.total(), Some(100));
}

#[test]
fn fraction_calculation() {
    let _guard = init_test_tracing();

    // 1. Создаём bridge и sink
    let bridge = ProgressBridge::noop();
    let sink = bridge.sink();

    // 2. Отправляем downloaded=50, total=100
    sink.update(DownloadProgress {
        downloaded: 50,
        total: Some(100),
    });

    // 3. Проверяем что fraction == Some(0.5)
    let fraction = bridge.fraction();
    assert_eq!(fraction, Some(0.5));
}

#[test]
fn fraction_none_when_total_unknown() {
    let _guard = init_test_tracing();

    // 1. Создаём bridge и sink
    let bridge = ProgressBridge::noop();
    let sink = bridge.sink();

    // 2. Отправляем update без total (total = None)
    sink.update(DownloadProgress {
        downloaded: 1024,
        total: None,
    });

    // 3. Проверяем что fraction == None (total неизвестен)
    assert_eq!(bridge.fraction(), None);

    // 4. Проверяем что downloaded обновился корректно
    assert_eq!(bridge.downloaded(), 1024);
    assert_eq!(bridge.total(), None);
}

#[test]
fn fraction_with_total_zero() {
    let _guard = init_test_tracing();

    // 1. Создаём bridge и sink
    let bridge = ProgressBridge::noop();
    let sink = bridge.sink();

    // 2. Сначала устанавливаем ненулевой total чтобы он сохранился в атомике
    sink.update(DownloadProgress {
        downloaded: 0,
        total: Some(200),
    });

    // 3. Проверяем промежуточное состояние
    assert_eq!(bridge.total(), Some(200));

    // 4. Теперь отправляем downloaded=0, total=Some(0)
    //    Внутренняя реализация: sink.update сохраняет total=0 в атомик,
    //    а snapshot/total() читают 0 как None (sentinel value).
    //    Поэтому fraction() вернёт None, а не Some(0.0).
    sink.update(DownloadProgress {
        downloaded: 0,
        total: Some(0),
    });

    // 5. Проверяем: атомик хранит 0, bridge интерпретирует это как "total неизвестен"
    assert_eq!(bridge.total(), None);
    assert_eq!(bridge.fraction(), None);
}

#[test]
fn reset_clears_counters() {
    let _guard = init_test_tracing();

    // 1. Создаём bridge и sink
    let bridge = ProgressBridge::noop();
    let sink = bridge.sink();

    // 2. Отправляем update с ненулевыми значениями
    sink.update(DownloadProgress {
        downloaded: 500,
        total: Some(1000),
    });

    // 3. Убеждаемся что значения записались
    assert_eq!(bridge.downloaded(), 500);
    assert_eq!(bridge.total(), Some(1000));

    // 4. Вызываем reset
    bridge.reset();

    // 5. Проверяем что счётчики обнулились
    assert_eq!(bridge.downloaded(), 0);
    assert_eq!(bridge.total(), None);
    assert_eq!(bridge.fraction(), None);
}

#[test]
fn repaint_hook_called() {
    let _guard = init_test_tracing();

    // 1. Создаём CallCounter и bridge с его hook'ом
    let counter = CallCounter::new();
    let bridge = ProgressBridge::new(counter.hook());

    // 2. Проверяем что счётчик начинается с нуля
    assert_eq!(counter.get(), 0);

    // 3. Получаем sink и отправляем update
    let sink = bridge.sink();
    sink.update(DownloadProgress {
        downloaded: 10,
        total: Some(100),
    });

    // 4. Проверяем что repaint hook был вызван один раз
    assert_eq!(counter.get(), 1);

    // 5. Отправляем ещё один update
    sink.update(DownloadProgress {
        downloaded: 50,
        total: Some(100),
    });

    // 6. Проверяем что счётчик увеличился до двух
    assert_eq!(counter.get(), 2);
}

#[test]
fn multiple_sinks_share_state() {
    let _guard = init_test_tracing();

    // 1. Создаём bridge и два sink'а
    let bridge = ProgressBridge::noop();
    let sink_a = bridge.sink();
    let sink_b = bridge.sink();

    // 2. Первый sink отправляет update
    sink_a.update(DownloadProgress {
        downloaded: 100,
        total: Some(500),
    });

    // 3. Проверяем что bridge отражает значения от первого sink'а
    assert_eq!(bridge.downloaded(), 100);
    assert_eq!(bridge.total(), Some(500));

    // 4. Второй sink отправляет update — перезаписывает значения
    sink_b.update(DownloadProgress {
        downloaded: 300,
        total: Some(600),
    });

    // 5. Проверяем что snapshot содержит последний update (от второго sink'а)
    let snap = bridge.snapshot();
    assert_eq!(snap.downloaded, 300);
    assert_eq!(snap.total, Some(600));
}

#[test]
fn concurrent_updates() {
    let _guard = init_test_tracing();

    // 1. Создаём bridge с CallCounter для отслеживания repaint вызовов
    let counter = CallCounter::new();
    let bridge = ProgressBridge::new(counter.hook());

    // 2. Определяем количество потоков и итераций
    let num_threads: u64 = 8;
    let iterations_per_thread: u64 = 1000;

    // 3. Запускаем N потоков, каждый делает update через свой sink
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let sink = bridge.sink();
            std::thread::spawn(move || {
                for i in 0..iterations_per_thread {
                    sink.update(DownloadProgress {
                        downloaded: thread_id * iterations_per_thread + i,
                        total: Some(num_threads * iterations_per_thread),
                    });
                }
            })
        })
        .collect();

    // 4. Ждём завершения всех потоков
    for handle in handles {
        handle.join().expect("поток запаниковал");
    }

    // 5. Проверяем что bridge не запаниковал и содержит корректные значения
    let snap = bridge.snapshot();
    assert_eq!(snap.total, Some(num_threads * iterations_per_thread));
    // downloaded — одно из записанных значений (последнее по happens-before)
    // проверяем только что оно в допустимом диапазоне
    assert!(snap.downloaded < num_threads * iterations_per_thread);

    // 6. Проверяем что repaint hook вызван ровно N * iterations раз
    assert_eq!(counter.get(), num_threads * iterations_per_thread);

    // 7. Проверяем что fraction вычисляется без паники
    let fraction = bridge.fraction();
    assert!(fraction.is_some(), "fraction должен быть Some, т.к. total задан");
    let f = fraction.unwrap();
    assert!((0.0..=1.0).contains(&f), "fraction должен быть в диапазоне [0.0, 1.0], получили {f}");
}

use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use tokio::runtime::Runtime;

use sheet_system::index_source::IndexSource;
use sheet_system::index_source::alias::IndexSourceAliasesManager;

fn alias_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("write_single_alias", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::new(0, 0);

            for _ in 0..iters {
                let test_dir = get_test_dir("benchmark_write_single_alias");
                let start = std::time::Instant::now();

                rt.block_on(async {
                    IndexSourceAliasesManager::write_alias(&test_dir, 100, 200)
                        .await
                        .unwrap();
                });

                cleanup_test_dir("benchmark_write_single_alias");
                total_duration += start.elapsed();
            }

            total_duration
        })
    });

    c.bench_function("read_single_alias", |b| {
        let test_dir = get_test_dir("benchmark_read_single_alias");
        rt.block_on(async {
            IndexSourceAliasesManager::write_alias(&test_dir, 100, 200)
                .await
                .unwrap();
        });

        b.iter_custom(|iters| {
            let mut total_duration = Duration::new(0, 0);

            for _ in 0..iters {
                let start = std::time::Instant::now();

                rt.block_on(async {
                    let exists = IndexSourceAliasesManager::alias_exists(&test_dir, 100)
                        .await
                        .unwrap();
                    black_box(exists);
                });

                total_duration += start.elapsed();
            }

            total_duration
        });

        cleanup_test_dir("benchmark_read_single_alias");
    });

    c.bench_function("convert_to_remote", |b| {
        let test_dir = get_test_dir("benchmark_convert_to_remote");
        rt.block_on(async {
            IndexSourceAliasesManager::write_alias(&test_dir, 100, 200)
                .await
                .unwrap();
        });

        b.iter_custom(|iters| {
            let mut total_duration = Duration::new(0, 0);

            for _ in 0..iters {
                let start = std::time::Instant::now();

                rt.block_on(async {
                    let remote_id = convert_to_remote(test_dir.clone(), 100).await.unwrap();
                    black_box(remote_id);
                });

                total_duration += start.elapsed();
            }

            total_duration
        });

        cleanup_test_dir("benchmark_convert_to_remote");
    });

    c.bench_function("delete_alias", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::new(0, 0);

            for _ in 0..iters {
                let test_dir = get_test_dir("benchmark_delete_alias");
                rt.block_on(async {
                    IndexSourceAliasesManager::write_alias(&test_dir, 100, 200)
                        .await
                        .unwrap();
                });

                let start = std::time::Instant::now();

                rt.block_on(async {
                    IndexSourceAliasesManager::delete_alias(&test_dir, 100)
                        .await
                        .unwrap();
                });

                cleanup_test_dir("benchmark_delete_alias");
                total_duration += start.elapsed();
            }

            total_duration
        })
    });

    let mut group = c.benchmark_group("batch_operations");
    group.throughput(Throughput::Elements(1));

    for count in [10, 100, 1000, 10000].iter() {
        group.bench_with_input(format!("write_{}_aliases", count), count, |b, &count| {
            b.iter_custom(|iters| {
                let mut total_duration = Duration::new(0, 0);

                for _ in 0..iters {
                    let test_dir = get_test_dir(&format!("benchmark_write_{}_aliases", count));
                    let start = std::time::Instant::now();

                    rt.block_on(async {
                        for i in 0..count {
                            IndexSourceAliasesManager::write_alias(&test_dir, i, i + 1000)
                                .await
                                .unwrap();
                        }
                    });

                    cleanup_test_dir(&format!("benchmark_write_{}_aliases", count));
                    total_duration += start.elapsed();
                }

                total_duration
            })
        });
    }

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(format!("read_{}_aliases", count), count, |b, &count| {
            let test_dir = get_test_dir(&format!("benchmark_read_{}_aliases", count));

            rt.block_on(async {
                for i in 0..count {
                    IndexSourceAliasesManager::write_alias(&test_dir, i, i + 1000)
                        .await
                        .unwrap();
                }
            });

            b.iter_custom(|iters| {
                let mut total_duration = Duration::new(0, 0);

                for _ in 0..iters {
                    let start = std::time::Instant::now();

                    rt.block_on(async {
                        for i in 0..count {
                            let exists = IndexSourceAliasesManager::alias_exists(&test_dir, i)
                                .await
                                .unwrap();
                            black_box(exists);
                        }
                    });

                    total_duration += start.elapsed();
                }

                total_duration
            });

            cleanup_test_dir(&format!("benchmark_read_{}_aliases", count));
        });
    }

    group.finish();

    c.bench_function("write_alias_cross_file_boundary", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::new(0, 0);

            for _ in 0..iters {
                let test_dir = get_test_dir("benchmark_cross_file_boundary");
                let start = std::time::Instant::now();

                rt.block_on(async {
                    IndexSourceAliasesManager::write_alias(&test_dir, 65535, 70000)
                        .await
                        .unwrap();
                    IndexSourceAliasesManager::write_alias(&test_dir, 65536, 70001)
                        .await
                        .unwrap();
                    IndexSourceAliasesManager::write_alias(&test_dir, 131071, 140000)
                        .await
                        .unwrap();
                });

                cleanup_test_dir("benchmark_cross_file_boundary");
                total_duration += start.elapsed();
            }

            total_duration
        })
    });

    c.bench_function("indexsource_to_remote_namespace", |b| {
        let test_dir = get_test_dir("benchmark_to_remote_namespace");
        rt.block_on(async {
            IndexSourceAliasesManager::write_alias(&test_dir, 500, 600)
                .await
                .unwrap();
        });

        b.iter_custom(|iters| {
            let mut total_duration = Duration::new(0, 0);

            for _ in 0..iters {
                let start = std::time::Instant::now();

                rt.block_on(async {
                    let index_source = IndexSource::new(false, 500, 1);
                    let result = index_source
                        .to_remote_namespace(test_dir.clone())
                        .await
                        .unwrap();
                    black_box(result);
                });

                total_duration += start.elapsed();
            }

            total_duration
        });

        cleanup_test_dir("benchmark_to_remote_namespace");
    });

    c.bench_function("concurrent_alias_operations", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::new(0, 0);

            for _ in 0..iters {
                let test_dir = get_test_dir("benchmark_concurrent_operations");
                let start = std::time::Instant::now();

                rt.block_on(async {
                    let dir = Arc::new(test_dir);
                    let mut handles = vec![];

                    for i in 0..10 {
                        let dir_clone = Arc::clone(&dir);
                        let handle = tokio::spawn(async move {
                            let local_id = i * 65536;
                            IndexSourceAliasesManager::write_alias(
                                &*dir_clone,
                                local_id,
                                local_id + 1000,
                            )
                            .await
                            .unwrap();

                            let exists =
                                IndexSourceAliasesManager::alias_exists(&*dir_clone, local_id)
                                    .await
                                    .unwrap();
                            black_box(exists);
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.unwrap();
                    }
                });

                cleanup_test_dir("benchmark_concurrent_operations");
                total_duration += start.elapsed();
            }

            total_duration
        })
    });

    c.bench_function("alias_not_found_error", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::new(0, 0);

            for _ in 0..iters {
                let test_dir = get_test_dir("benchmark_alias_not_found");
                let start = std::time::Instant::now();

                rt.block_on(async {
                    let result = convert_to_remote(test_dir, 9999).await;
                    let _ = black_box(result);
                });

                cleanup_test_dir("benchmark_alias_not_found");
                total_duration += start.elapsed();
            }

            total_duration
        })
    });
}

use sheet_system::index_source::alias::convert_to_remote;

fn get_test_dir(test_name: &str) -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let test_dir = current_dir
        .join(".temp")
        .join("benchmarks")
        .join("alias")
        .join(test_name);

    std::fs::create_dir_all(&test_dir).expect("Failed to create test directory");
    test_dir
}

fn cleanup_test_dir(test_name: &str) {
    let test_dir = get_test_dir(test_name);
    if test_dir.exists() {
        std::fs::remove_dir_all(&test_dir).ok();
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3));
    targets = alias_benchmark
}

criterion_main!(benches);

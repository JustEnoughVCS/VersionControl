use std::{env::current_dir, hint::black_box, path::PathBuf};

use criterion::{Criterion, criterion_group, criterion_main};
use sheet_system::{
    index_source::IndexSource,
    mapping::{LocalMapping, LocalMappingForward},
    sheet::SheetData,
};

fn sheet_file() -> PathBuf {
    current_dir().unwrap().join(".temp").join("benchmark.sheet")
}

fn sheet_benchmark(c: &mut Criterion) {
    let sheet_file = sheet_file();
    std::fs::write(&sheet_file, sheet_write(u16::MAX as usize).as_bytes()).unwrap();

    c.bench_function("sheet insert and apply", |b| b.iter(|| sheet_write(25)));
    c.bench_function("sheet full read", |b| {
        b.iter(|| sheet_full_read(&sheet_file))
    });
    c.bench_function("sheet mmap read", |b| {
        b.iter(|| sheet_mmap_read(&sheet_file))
    });
    c.bench_function("sheet mmap copy read", |b| {
        b.iter(|| sheet_mmap_copy_read(&sheet_file))
    });
    c.bench_function("sheet mmap read x10000", |b| {
        let mmap = SheetData::mmap(&sheet_file).unwrap();
        let key = ["docs", "my_file_12.txt"];

        b.iter(|| {
            for _ in 0..10000 {
                black_box(mmap.mp_c(&key).unwrap());
            }
        })
    });
}

fn sheet_write(count: usize) -> SheetData {
    let mut sheet = SheetData::empty().pack("empty");
    (0..count).for_each(|n| {
        sheet
            .insert_mapping(
                LocalMapping::new(
                    vec!["docs".to_string(), format!("my_file_{}.txt", n)],
                    IndexSource::new(true, n as u32, 2u16),
                    LocalMappingForward::Latest,
                )
                .unwrap(),
            )
            .unwrap();
    });
    sheet.apply().unwrap();
    sheet.unpack()
}

async fn sheet_full_read(path: &PathBuf) {
    let mut sheet_data = SheetData::empty();
    sheet_data.full_read(path).await.unwrap();
}

fn sheet_mmap_read(path: &PathBuf) {
    let mmap = SheetData::mmap(path).unwrap();
    mmap.mp(&["docs", "my_file_12.txt"]).unwrap().unwrap();
}

fn sheet_mmap_copy_read(path: &PathBuf) {
    let mmap = SheetData::mmap(path).unwrap();
    mmap.mp_c(&["docs", "my_file_12.txt"]).unwrap().unwrap();
}

criterion_group!(benches, sheet_benchmark);
criterion_main!(benches);

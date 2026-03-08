use std::{env::current_dir, hint::black_box, path::PathBuf};

use criterion::{Criterion, criterion_group, criterion_main};
use sheet_system::{
    index_source::IndexSource,
    mapping::{LocalMapping, LocalMappingForward},
    sheet::SheetData,
};

fn sheet_benchmark(c: &mut Criterion) {
    benchmark_file(c, 1);
    benchmark_file(c, 16);
    benchmark_file(c, 256);
    benchmark_file(c, 65535);

    c.bench_function("sheet insert and apply", |b| b.iter(|| sheet_write(25)));
}

fn sheet_file(count: u32) -> PathBuf {
    let path = current_dir()
        .unwrap()
        .join(".temp")
        .join(format!("benchmark_{}mappings.sheet", count));
    std::fs::write(&path, sheet_write(count).as_bytes()).unwrap();
    path
}

fn benchmark_file(c: &mut Criterion, count: u32) {
    let sheet_file = sheet_file(count);
    c.bench_function(format!("full read {} mappings", count).as_str(), |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(sheet_full_read(&sheet_file))
        })
    });
    c.bench_function(format!("mmap find in {} mappings", count).as_str(), |b| {
        b.iter(|| sheet_mmap_read(&sheet_file, count))
    });
    c.bench_function(
        format!("mmap find in {} mappings (copy)", count).as_str(),
        |b| b.iter(|| sheet_mmap_copy_read(&sheet_file, count)),
    );
}

fn sheet_write(count: u32) -> SheetData {
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
    let median = 0;
    let key = vec!["docs".to_string(), format!("my_file_{}.txt", median)];
    black_box(sheet_data.mappings().get(&key).unwrap());
}

fn sheet_mmap_read(path: &PathBuf, count: u32) {
    let median = (count / 2) as usize;
    let mmap = SheetData::mmap(path).unwrap();
    let result = mmap
        .mp(&["docs", &format!("my_file_{}.txt", median)])
        .unwrap()
        .unwrap();
    black_box(result);
}

fn sheet_mmap_copy_read(path: &PathBuf, count: u32) {
    let median = (count / 2) as usize;
    let mmap = SheetData::mmap(path).unwrap();
    let result = mmap
        .mp_c(&["docs", &format!("my_file_{}.txt", median)])
        .unwrap()
        .unwrap();
    black_box(result);
}

criterion_group!(benches, sheet_benchmark);
criterion_main!(benches);

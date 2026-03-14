use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use sheet_system::{
    index_source::IndexSource,
    mapping::{LocalMapping, LocalMappingForward},
    sheet::SheetData,
    sheet_tree::SheetDataTree,
};

fn sheet_tree_benchmark(c: &mut Criterion) {
    c.bench_function("tree build 10 mappings", |b| {
        b.iter(|| build_and_bench_tree(10))
    });

    c.bench_function("tree build 100 mappings", |b| {
        b.iter(|| build_and_bench_tree(100))
    });

    c.bench_function("tree build 1000 mappings", |b| {
        b.iter(|| build_and_bench_tree(1000))
    });

    c.bench_function("tree build 10000 mappings", |b| {
        b.iter(|| build_and_bench_tree(10000))
    });

    c.bench_function("tree find shallow path (10 mappings)", |b| {
        let sheet_data = create_sheet_data(10);
        let tree = SheetDataTree::from(&sheet_data);
        b.iter(|| find_shallow_path(&tree))
    });

    c.bench_function("tree find shallow path (100 mappings)", |b| {
        let sheet_data = create_sheet_data(100);
        let tree = SheetDataTree::from(&sheet_data);
        b.iter(|| find_shallow_path(&tree))
    });

    c.bench_function("tree find shallow path (1000 mappings)", |b| {
        let sheet_data = create_sheet_data(1000);
        let tree = SheetDataTree::from(&sheet_data);
        b.iter(|| find_shallow_path(&tree))
    });

    c.bench_function("tree find deep path (10 mappings)", |b| {
        let sheet_data = create_sheet_data(10);
        let tree = SheetDataTree::from(&sheet_data);
        b.iter(|| find_deep_path(&tree))
    });

    c.bench_function("tree find deep path (100 mappings)", |b| {
        let sheet_data = create_sheet_data(100);
        let tree = SheetDataTree::from(&sheet_data);
        b.iter(|| find_deep_path(&tree))
    });

    c.bench_function("tree find deep path (1000 mappings)", |b| {
        let sheet_data = create_sheet_data(1000);
        let tree = SheetDataTree::from(&sheet_data);
        b.iter(|| find_deep_path(&tree))
    });

    c.bench_function("tree find in flat structure (1000 mappings)", |b| {
        let sheet_data = create_flat_structure_sheet_data(1000);
        let tree = SheetDataTree::from(&sheet_data);
        b.iter(|| find_in_flat_structure(&tree))
    });

    c.bench_function("tree find in deep structure (100 mappings)", |b| {
        let sheet_data = create_deep_structure_sheet_data(100);
        let tree = SheetDataTree::from(&sheet_data);
        b.iter(|| find_in_deep_structure(&tree))
    });

    c.bench_function("direct mapping find (1000 mappings)", |b| {
        let sheet_data = create_sheet_data(1000);
        b.iter(|| direct_mapping_find(&sheet_data))
    });
}

fn build_and_bench_tree(count: u32) {
    let sheet_data = create_sheet_data(count);
    let tree = SheetDataTree::from(&sheet_data);
    black_box(tree);
}

fn create_sheet_data(count: u32) -> SheetData {
    let mut sheet = SheetData::empty().pack("benchmark_sheet");

    for n in 0..count {
        let depth = (n % 3) + 1;
        let mut path_parts = Vec::new();

        for d in 0..depth {
            path_parts.push(format!("dir_{}", d));
        }
        path_parts.push(format!("file_{}.txt", n));

        sheet
            .insert_mapping(
                LocalMapping::new(
                    path_parts,
                    IndexSource::new(true, n as u32, 1),
                    LocalMappingForward::Latest,
                )
                .unwrap(),
            )
            .unwrap();
    }

    sheet.apply().unwrap();
    sheet.unpack()
}

fn create_flat_structure_sheet_data(count: u32) -> SheetData {
    let mut sheet = SheetData::empty().pack("flat_structure_sheet");

    for n in 0..count {
        sheet
            .insert_mapping(
                LocalMapping::new(
                    vec![format!("file_{}.txt", n)],
                    IndexSource::new(true, n as u32, 1),
                    LocalMappingForward::Latest,
                )
                .unwrap(),
            )
            .unwrap();
    }

    sheet.apply().unwrap();
    sheet.unpack()
}

fn create_deep_structure_sheet_data(count: u32) -> SheetData {
    let mut sheet = SheetData::empty().pack("deep_structure_sheet");

    for n in 0..count {
        let mut path_parts = Vec::new();
        for d in 0..5 {
            path_parts.push(format!("level_{}", d));
        }
        path_parts.push(format!("file_{}.txt", n));

        sheet
            .insert_mapping(
                LocalMapping::new(
                    path_parts,
                    IndexSource::new(true, n as u32, 1),
                    LocalMappingForward::Latest,
                )
                .unwrap(),
            )
            .unwrap();
    }

    sheet.apply().unwrap();
    sheet.unpack()
}

fn find_shallow_path(tree: &SheetDataTree) {
    let root = tree.root();
    if let Some(dir_node) = root.next("dir_0") {
        if let Some(file_node) = dir_node.next("file_0.txt") {
            black_box(file_node.mapping());
        }
    }
}

fn find_deep_path(tree: &SheetDataTree) {
    let root = tree.root();
    if let Some(dir0) = root.next("dir_0") {
        if let Some(dir1) = dir0.next("dir_1") {
            if let Some(dir2) = dir1.next("dir_2") {
                if let Some(file_node) = dir2.next("file_0.txt") {
                    black_box(file_node.mapping());
                }
            }
        }
    }
}

fn find_in_flat_structure(tree: &SheetDataTree) {
    let root = tree.root();
    if let Some(file_node) = root.next("file_500.txt") {
        black_box(file_node.mapping());
    }
}

fn find_in_deep_structure(tree: &SheetDataTree) {
    let root = tree.root();
    if let Some(level0) = root.next("level_0") {
        if let Some(level1) = level0.next("level_1") {
            if let Some(level2) = level1.next("level_2") {
                if let Some(level3) = level2.next("level_3") {
                    if let Some(level4) = level3.next("level_4") {
                        if let Some(file_node) = level4.next("file_50.txt") {
                            black_box(file_node.mapping());
                        }
                    }
                }
            }
        }
    }
}

/// 直接使用映射查找（作为对比基准）
fn direct_mapping_find(sheet_data: &SheetData) {
    let key = vec![
        "dir_0".to_string(),
        "dir_1".to_string(),
        "dir_2".to_string(),
        "file_0.txt".to_string(),
    ];
    if let Some(mapping) = sheet_data.mappings().get(&key) {
        black_box(mapping);
    }
}

criterion_group!(benches, sheet_tree_benchmark);
criterion_main!(benches);

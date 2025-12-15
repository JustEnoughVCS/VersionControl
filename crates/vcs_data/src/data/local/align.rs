use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use data_struct::dada_sort::quick_sort_with_cmp;

use crate::data::local::file_status::AnalyzeResult;

pub type AlignTasksName = String;
pub type AlignPathBuf = PathBuf;
pub type AlignLostPathBuf = PathBuf;
pub type AlignCreatedPathBuf = PathBuf;

pub struct AlignTasks {
    pub created: Vec<(AlignTasksName, AlignPathBuf)>,
    pub lost: Vec<(AlignTasksName, AlignPathBuf)>,
    pub moved: Vec<(AlignTasksName, (AlignLostPathBuf, AlignCreatedPathBuf))>,
    pub erased: Vec<(AlignTasksName, AlignPathBuf)>,
}

impl AlignTasks {
    pub fn clone_from_analyze_result(result: &AnalyzeResult) -> Self {
        AlignTasks {
            created: path_hash_set_sort_helper(result.created.clone(), "created"),
            lost: path_hash_set_sort_helper(result.lost.clone(), "lost"),
            moved: path_hash_map_sort_helper(result.moved.clone(), "moved"),
            erased: path_hash_set_sort_helper(result.erased.clone(), "erased"),
        }
    }

    pub fn from_analyze_result(result: AnalyzeResult) -> Self {
        AlignTasks {
            created: path_hash_set_sort_helper(result.created, "created"),
            lost: path_hash_set_sort_helper(result.lost, "lost"),
            moved: path_hash_map_sort_helper(result.moved, "moved"),
            erased: path_hash_set_sort_helper(result.erased, "erased"),
        }
    }
}

fn path_hash_set_sort_helper(
    hash_set: HashSet<PathBuf>,
    prefix: impl Into<String>,
) -> Vec<(String, PathBuf)> {
    let prefix_str = prefix.into();
    let mut vec: Vec<(String, PathBuf)> = hash_set
        .into_iter()
        .map(|path| {
            let hash = sha1_hash::calc_sha1_string(path.to_string_lossy());
            let hash_prefix: String = hash.chars().take(8).collect();
            let name = format!("{}:{}", prefix_str, hash_prefix);
            (name, path)
        })
        .collect();

    quick_sort_with_cmp(&mut vec, false, |a, b| {
        // Compare by path depth first
        let a_depth = a.1.components().count();
        let b_depth = b.1.components().count();

        if a_depth != b_depth {
            return if a_depth < b_depth { -1 } else { 1 };
        }

        // If same depth, compare lexicographically
        match a.1.cmp(&b.1) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    });

    vec
}

fn path_hash_map_sort_helper(
    hash_map: HashMap<String, (PathBuf, PathBuf)>,
    prefix: impl Into<String>,
) -> Vec<(String, (PathBuf, PathBuf))> {
    let prefix_str = prefix.into();
    let mut vec: Vec<(String, (PathBuf, PathBuf))> = hash_map
        .into_values()
        .map(|(path1, path2)| {
            let hash = sha1_hash::calc_sha1_string(path1.to_string_lossy());
            let hash_prefix: String = hash.chars().take(8).collect();
            let name = format!("{}:{}", prefix_str, hash_prefix);
            (name, (path1, path2))
        })
        .collect();

    quick_sort_with_cmp(&mut vec, false, |a, b| {
        // Compare by first PathBuf's path depth first
        let a_depth = a.1.0.components().count();
        let b_depth = b.1.0.components().count();

        if a_depth != b_depth {
            return if a_depth < b_depth { -1 } else { 1 };
        }

        // If same depth, compare lexicographically by first PathBuf
        match a.1.0.cmp(&b.1.0) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    });

    vec
}

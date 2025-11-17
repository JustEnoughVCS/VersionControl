use std::{
    collections::{HashMap, HashSet},
    io::Error,
    path::PathBuf,
};

use sha1_hash::calc_sha1_multi;
use string_proc::format_path::format_path;
use walkdir::WalkDir;

use crate::data::{
    local::{LocalWorkspace, cached_sheet::CachedSheet, local_sheet::LocalSheet},
    member::MemberId,
    sheet::{SheetData, SheetName},
    vault::virtual_file::VirtualFileId,
};

pub type FromRelativePathBuf = PathBuf;
pub type ToRelativePathBuf = PathBuf;
pub type CreatedRelativePathBuf = PathBuf;
pub type LostRelativePathBuf = PathBuf;
pub type ModifiedRelativePathBuf = PathBuf;

pub struct AnalyzeResult<'a> {
    local_workspace: &'a LocalWorkspace,

    /// Moved local files
    pub moved: HashMap<VirtualFileId, (FromRelativePathBuf, ToRelativePathBuf)>,

    /// Newly created local files
    pub created: HashSet<CreatedRelativePathBuf>,

    /// Lost local files
    pub lost: HashSet<LostRelativePathBuf>,

    /// Modified local files (excluding moved files)
    /// For files that were both moved and modified, changes can only be detected after LocalSheet mapping is aligned with actual files
    pub modified: HashSet<ModifiedRelativePathBuf>,
}

struct AnalyzeContext<'a> {
    member: MemberId,
    sheet_name: SheetName,
    local_sheet: Option<LocalSheet<'a>>,
    cached_sheet_data: Option<SheetData>,
}

impl<'a> AnalyzeResult<'a> {
    /// Analyze all files, calculate the file information provided
    pub async fn analyze_local_status(
        local_workspace: &'a LocalWorkspace,
    ) -> Result<AnalyzeResult<'a>, std::io::Error> {
        // Workspace
        let workspace = local_workspace;

        // Current member, sheet
        let (member, sheet_name) = {
            let lock = workspace.config.lock().await;
            let member = lock.current_account();
            let Some(sheet) = lock.sheet_in_use().clone() else {
                return Err(Error::new(std::io::ErrorKind::NotFound, "Sheet not found"));
            };
            (member, sheet)
        };

        // Local files (RelativePaths)
        let local_path = workspace.local_path();
        let file_relative_paths = {
            let mut paths = HashSet::new();
            for entry in WalkDir::new(&local_path) {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                // Skip entries that contain ".jv" in their path
                if entry.path().to_string_lossy().contains(".jv") {
                    continue;
                }

                if entry.file_type().is_file() {
                    if let Ok(relative_path) = entry.path().strip_prefix(&local_path) {
                        let format = format_path(relative_path.to_path_buf());
                        let Ok(format) = format else {
                            continue;
                        };
                        paths.insert(format);
                    }
                }
            }

            paths
        };

        // Read local sheet
        let local_sheet = match workspace.local_sheet(&member, &sheet_name).await {
            Ok(v) => Some(v),
            Err(_) => None,
        };

        // Read cached sheet
        let cached_sheet_data = match CachedSheet::cached_sheet_data(&sheet_name).await {
            Ok(v) => Some(v),
            Err(_) => {
                return Err(Error::new(
                    std::io::ErrorKind::NotFound,
                    "Cached sheet not found",
                ));
            }
        };

        // Create new result
        let mut result = Self::none_result(&workspace);

        // Analyze entry
        let mut analyze_ctx = AnalyzeContext {
            member,
            sheet_name,
            local_sheet,
            cached_sheet_data,
        };
        Self::analyze_moved(&mut result, &file_relative_paths, &analyze_ctx).await?;
        Self::analyze_modified(&mut result, &file_relative_paths, &mut analyze_ctx).await?;

        Ok(result)
    }

    /// Track file moves by comparing recorded SHA1 hashes with actual file SHA1 hashes
    /// For files that cannot be directly matched, continue searching using fuzzy matching algorithms
    async fn analyze_moved(
        result: &mut AnalyzeResult<'_>,
        file_relative_paths: &HashSet<PathBuf>,
        analyze_ctx: &AnalyzeContext<'a>,
    ) -> Result<(), std::io::Error> {
        let local_sheet_paths: HashSet<&PathBuf> = match &analyze_ctx.local_sheet {
            Some(local_sheet) => local_sheet.data.mapping.keys().collect(),
            None => HashSet::new(),
        };
        let file_relative_paths_ref: HashSet<&PathBuf> = file_relative_paths.iter().collect();

        // 在本地表存在但实际不存在的文件，为丢失
        let mut lost_files: HashSet<&PathBuf> = local_sheet_paths
            .difference(&file_relative_paths_ref)
            .cloned()
            .collect();

        // 在本地表不存在但实际存在的文件，记录为新建
        let mut new_files: HashSet<&PathBuf> = file_relative_paths_ref
            .difference(&local_sheet_paths)
            .cloned()
            .collect();

        // 计算新增的文件 Hash
        let new_files_for_hash: Vec<PathBuf> = new_files.iter().map(|p| (*p).clone()).collect();
        let file_hashes: HashSet<(PathBuf, String)> =
            match calc_sha1_multi::<PathBuf, Vec<PathBuf>>(new_files_for_hash, 8192).await {
                Ok(hash) => hash,
                Err(e) => return Err(Error::new(std::io::ErrorKind::Other, e)),
            }
            .iter()
            .map(|r| (r.file_path.clone(), r.hash.to_string()))
            .collect();

        // 建立丢失文件的 Hash 映射表
        let mut lost_files_hash_mapping: HashMap<String, FromRelativePathBuf> =
            match &analyze_ctx.local_sheet {
                Some(local_sheet) => lost_files
                    .iter()
                    .filter_map(|f| {
                        local_sheet.mapping_data(f).ok().map(|mapping_data| {
                            (mapping_data.hash_when_updated.clone(), (*f).clone())
                        })
                    })
                    .collect(),
                None => HashMap::new(),
            };

        // 如果这些 Hash 能对应缺失文件的 Hash，那么这对新增和丢失项将被合并为移动项
        let mut moved_files: HashSet<(FromRelativePathBuf, ToRelativePathBuf)> = HashSet::new();
        for (new_path, new_hash) in file_hashes {
            // 如果新的 Hash 值命中映射，则添加移动项
            if let Some(lost_path) = lost_files_hash_mapping.remove(&new_hash) {
                // 移除该新增项和丢失项
                lost_files.remove(&lost_path);
                new_files.remove(&new_path);

                // 建立移动项
                moved_files.insert((lost_path.clone(), new_path));
            }
        }

        // 进入模糊匹配，将其他未匹配的可能移动项进行匹配
        // 如果 新增 和 缺失 数量总和能被 2 整除，则说明还存在文件被移动的可能，考虑尝试模糊匹配
        if new_files.len() + lost_files.len() % 2 == 0 {
            // 尝试模糊匹配
            // ...
        }

        // 将结果收集，并设置结果
        result.created = new_files.iter().map(|p| (*p).clone()).collect();
        result.lost = lost_files.iter().map(|p| (*p).clone()).collect();
        result.moved = moved_files
            .iter()
            .filter_map(|(from, to)| {
                let vfid = analyze_ctx
                    .local_sheet
                    .as_ref()
                    .and_then(|local_sheet| local_sheet.mapping_data(from).ok())
                    .map(|mapping_data| mapping_data.mapping_vfid.clone());
                if let Some(vfid) = vfid {
                    Some((vfid, (from.clone(), to.clone())))
                } else {
                    None
                }
            })
            .collect();

        Ok(())
    }

    /// Compare using file modification time and SHA1 hash values.
    /// Note: For files that have been both moved and modified, they can only be recognized as modified after their location is matched.
    async fn analyze_modified(
        result: &mut AnalyzeResult<'_>,
        file_relative_paths: &HashSet<PathBuf>,
        analyze_ctx: &mut AnalyzeContext<'a>,
    ) -> Result<(), std::io::Error> {
        let local_sheet = &mut analyze_ctx.local_sheet.as_mut().unwrap();
        let local_path = local_sheet.local_workspace.local_path().clone();

        for path in file_relative_paths {
            // Get mapping data
            let Ok(mapping_data) = local_sheet.mapping_data_mut(&path) else {
                continue;
            };

            // If modified time not changed, skip
            let modified_time = std::fs::metadata(local_path.join(path))?.modified()?;
            if &modified_time == mapping_data.last_modifiy_check_time() {
                if mapping_data.last_modifiy_check_result() {
                    result.modified.insert(path.clone());
                }
                continue;
            }

            // Calculate hash
            let hash_calc = match sha1_hash::calc_sha1(path, 2048).await {
                Ok(hash) => hash,
                Err(e) => return Err(Error::new(std::io::ErrorKind::Other, e)),
            };

            // If hash not match, mark as modified
            if &hash_calc.hash != mapping_data.hash_when_updated() {
                result.modified.insert(path.clone());

                // Update last modified check time to modified time
                mapping_data.last_modifiy_check_time = modified_time;
                mapping_data.last_modifiy_check_result = true;
            } else {
                // Update last modified check time to modified time
                mapping_data.last_modifiy_check_time = modified_time;
                mapping_data.last_modifiy_check_result = false;
            }
        }

        // Persist the local sheet data
        LocalSheet::write(local_sheet).await?;

        Ok(())
    }

    /// Generate a empty AnalyzeResult
    fn none_result(local_workspace: &'a LocalWorkspace) -> AnalyzeResult<'a> {
        AnalyzeResult {
            local_workspace: local_workspace,
            moved: HashMap::new(),
            created: HashSet::new(),
            lost: HashSet::new(),
            modified: HashSet::new(),
        }
    }
}

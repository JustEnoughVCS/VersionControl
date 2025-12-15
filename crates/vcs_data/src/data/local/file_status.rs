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

    /// Erased local files
    pub erased: HashSet<LostRelativePathBuf>,

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
            let mut_workspace = workspace.config.lock().await;
            let member = mut_workspace.current_account();
            let Some(sheet) = mut_workspace.sheet_in_use().clone() else {
                return Err(Error::new(std::io::ErrorKind::NotFound, "Sheet not found"));
            };
            (member, sheet)
        };

        // Local files (RelativePaths)
        let local_path = workspace.local_path();
        let file_relative_paths = {
            let mut paths = HashSet::new();
            for entry in WalkDir::new(local_path) {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                // Skip entries that contain ".jv" in their path
                if entry.path().to_string_lossy().contains(".jv") {
                    continue;
                }

                if entry.file_type().is_file()
                    && let Ok(relative_path) = entry.path().strip_prefix(local_path)
                {
                    let format = format_path(relative_path.to_path_buf());
                    let Ok(format) = format else {
                        continue;
                    };
                    paths.insert(format);
                }
            }

            paths
        };

        // Read local sheet
        let local_sheet = (workspace.local_sheet(&member, &sheet_name).await).ok();

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
        let mut result = Self::none_result(workspace);

        // Analyze entry
        let mut analyze_ctx = AnalyzeContext {
            member,
            sheet_name,
            local_sheet,
            cached_sheet_data,
        };
        Self::analyze_moved(&mut result, &file_relative_paths, &analyze_ctx, workspace).await?;
        Self::analyze_modified(
            &mut result,
            &file_relative_paths,
            &mut analyze_ctx,
            workspace,
        )
        .await?;

        Ok(result)
    }

    /// Track file moves by comparing recorded SHA1 hashes with actual file SHA1 hashes
    /// For files that cannot be directly matched, continue searching using fuzzy matching algorithms
    async fn analyze_moved(
        result: &mut AnalyzeResult<'_>,
        file_relative_paths: &HashSet<PathBuf>,
        analyze_ctx: &AnalyzeContext<'a>,
        workspace: &LocalWorkspace,
    ) -> Result<(), std::io::Error> {
        let local_sheet_paths: HashSet<&PathBuf> = match &analyze_ctx.local_sheet {
            Some(local_sheet) => local_sheet.data.mapping.keys().collect(),
            None => HashSet::new(),
        };
        let file_relative_paths_ref: HashSet<&PathBuf> = file_relative_paths.iter().collect();

        // Files that exist in the local sheet but not in reality are considered lost
        let mut lost_files: HashSet<&PathBuf> = local_sheet_paths
            .difference(&file_relative_paths_ref)
            .cloned()
            .collect();

        // Files that exist in reality but not in the local sheet are recorded as newly created
        let mut new_files: HashSet<&PathBuf> = file_relative_paths_ref
            .difference(&local_sheet_paths)
            .cloned()
            .collect();

        // Files that exist locally but not in remote
        let mut erased_files: HashSet<PathBuf> = HashSet::new();

        if let Some(cached_data) = &analyze_ctx.cached_sheet_data {
            if let Some(local_sheet) = &analyze_ctx.local_sheet {
                let cached_sheet_mapping = cached_data.mapping();
                let local_sheet_mapping = &local_sheet.data.mapping;

                // Find paths that exist in local sheet but not in cached sheet
                for local_path in local_sheet_mapping.keys() {
                    if !cached_sheet_mapping.contains_key(local_path) {
                        erased_files.insert(local_path.clone());
                    }
                }
            }
        }

        // Calculate hashes for new files
        let new_files_for_hash: Vec<PathBuf> = new_files
            .iter()
            .map(|p| workspace.local_path.join(p))
            .collect();
        let file_hashes: HashSet<(PathBuf, String)> =
            match calc_sha1_multi::<PathBuf, Vec<PathBuf>>(new_files_for_hash, 8192).await {
                Ok(hash) => hash,
                Err(e) => return Err(Error::other(e)),
            }
            .iter()
            .map(|r| (r.file_path.clone(), r.hash.to_string()))
            .collect();

        // Build hash mapping table for lost files
        let mut lost_files_hash_mapping: HashMap<String, FromRelativePathBuf> =
            match &analyze_ctx.local_sheet {
                Some(local_sheet) => lost_files
                    .iter()
                    .filter_map(|f| {
                        local_sheet.mapping_data(f).ok().map(|mapping_data| {
                            (
                                // Using the most recently recorded Hash can more accurately identify moved items,
                                // but if it doesn't exist, fall back to the initially recorded Hash
                                mapping_data
                                    .last_modifiy_check_hash
                                    .as_ref()
                                    .cloned()
                                    .unwrap_or(mapping_data.hash_when_updated.clone()),
                                (*f).clone(),
                            )
                        })
                    })
                    .collect(),
                None => HashMap::new(),
            };

        // If these hashes correspond to the hashes of missing files, then this pair of new and lost items will be merged into moved items
        let mut moved_files: HashSet<(FromRelativePathBuf, ToRelativePathBuf)> = HashSet::new();
        for (new_path, new_hash) in file_hashes {
            let new_path = new_path
                .strip_prefix(&workspace.local_path)
                .map(|p| p.to_path_buf())
                .unwrap_or(new_path);

            // If the new hash value hits the mapping, add a moved item
            if let Some(lost_path) = lost_files_hash_mapping.remove(&new_hash) {
                // Remove this new item and lost item
                lost_files.remove(&lost_path);
                new_files.remove(&new_path);

                // Create moved item
                moved_files.insert((lost_path.clone(), new_path));
            }
        }

        // Enter fuzzy matching to match other potentially moved items that haven't been matched
        // If the total number of new and lost files is divisible by 2, it indicates there might still be files that have been moved, consider trying fuzzy matching
        if new_files.len() + lost_files.len() % 2 == 0 {
            // Try fuzzy matching
            // ...
        }

        // Collect results and set the result
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
                vfid.map(|vfid| (vfid, (from.clone(), to.clone())))
            })
            .collect();
        result.erased = erased_files;

        Ok(())
    }

    /// Compare using file modification time and SHA1 hash values.
    /// Note: For files that have been both moved and modified, they can only be recognized as modified after their location is matched.
    async fn analyze_modified(
        result: &mut AnalyzeResult<'_>,
        file_relative_paths: &HashSet<PathBuf>,
        analyze_ctx: &mut AnalyzeContext<'a>,
        workspace: &LocalWorkspace,
    ) -> Result<(), std::io::Error> {
        let local_sheet = &mut analyze_ctx.local_sheet.as_mut().unwrap();
        let local_path = local_sheet.local_workspace.local_path().clone();

        for path in file_relative_paths {
            // Get mapping data
            let Ok(mapping_data) = local_sheet.mapping_data_mut(path) else {
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
            let hash_calc = match sha1_hash::calc_sha1(workspace.local_path.join(path), 2048).await
            {
                Ok(hash) => hash,
                Err(e) => return Err(Error::other(e)),
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

            // Record latest hash
            mapping_data.last_modifiy_check_hash = Some(hash_calc.hash)
        }

        // Persist the local sheet data
        LocalSheet::write(local_sheet).await?;

        Ok(())
    }

    /// Generate a empty AnalyzeResult
    fn none_result(local_workspace: &'a LocalWorkspace) -> AnalyzeResult<'a> {
        AnalyzeResult {
            local_workspace,
            moved: HashMap::new(),
            created: HashSet::new(),
            lost: HashSet::new(),
            modified: HashSet::new(),
            erased: HashSet::new(),
        }
    }
}

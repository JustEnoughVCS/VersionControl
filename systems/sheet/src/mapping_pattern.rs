// Mapping Pattern
// 是用来匹配多个 Mapping 的语法
//
// ~ 当前 Sheet
//
// 省略机制
//
// 如果上下文有sheet，那就是
// mapping/file.suffix
//
// 如果没有sheet，那就是
// sheet:/mapping/file.suffix
//
// 可以使用路径语法
//
// sheet:/mapping/../file.suffix
//
// 使用以下逻辑匹配文件
//
// sheet:/. 匹配 sheet 中 / 下所有文件
// sheet:/arts/. 匹配结果为 sheet:/arts/ 下的文件
// sheet:/arts/ 匹配结果为 sheet:/arts/ 下的文件
// sheet:/arts 匹配结果为 sheet:/arts 文件夹
//
// 文件名匹配机制
//
// *.md 匹配所有 md 文件
// [Mm]essages 匹配 Message 或 message
// 使用\[Mm\]essage 匹配 [Mm]essage
// 使用 ** 匹配目录下所有文件（递归）
// 使用 * 匹配目录下所有文件
// 使用 ![XX] 匹配排除以外的所有文件，例如：
//
// ![README.md]* 匹配除名叫 README.md 以外的所有当前目录下的文件
// ![[Rr][Ee][Aa][Dd][Mm][Ee].[Mm][Dd]]** 匹配所有 不叫 README.md 的文件
//
// ![**/temp]** 匹配所有不在temp目录的文件
//
// MappingPattern会根据MappingContext生成MappingPatternResult
// 准确来说是
// PatternResult::Single Or PatternResult::Multi
// PatternResult可以unwrap为single()或multi()

use crate::mapping::MappingBuf;

pub struct MappingPattern {}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MappingPatternResult {
    Single(MappingBuf),
    Multi(Vec<MappingBuf>),
}

impl MappingPatternResult {
    pub fn new_single(mapping: MappingBuf) -> Self {
        Self::Single(mapping)
    }

    pub fn new_multi(mappings: Vec<MappingBuf>) -> Self {
        Self::Multi(mappings)
    }

    pub fn is_single(&self) -> bool {
        match self {
            MappingPatternResult::Single(_) => true,
            MappingPatternResult::Multi(_) => false,
        }
    }

    pub fn is_multi(&self) -> bool {
        match self {
            MappingPatternResult::Single(_) => false,
            MappingPatternResult::Multi(_) => true,
        }
    }

    pub fn single(self) -> Option<MappingBuf> {
        match self {
            MappingPatternResult::Single(mapping) => Some(mapping),
            MappingPatternResult::Multi(_) => None,
        }
    }

    pub fn multi(self) -> Option<Vec<MappingBuf>> {
        match self {
            MappingPatternResult::Single(_) => None,
            MappingPatternResult::Multi(mappings) => Some(mappings),
        }
    }

    pub fn ensure_multi(self) -> Vec<MappingBuf> {
        match self {
            MappingPatternResult::Single(mapping) => vec![mapping],
            MappingPatternResult::Multi(mappings) => mappings,
        }
    }

    pub fn unwrap_single(self) -> MappingBuf {
        match self {
            MappingPatternResult::Single(mapping) => mapping,
            MappingPatternResult::Multi(_) => panic!("Called `unwrap_single()` on a `Multi` value"),
        }
    }

    pub fn unwrap_multi(self) -> Vec<MappingBuf> {
        match self {
            MappingPatternResult::Single(_) => {
                panic!("Called `unwrap_multi()` on a `Single` value")
            }
            MappingPatternResult::Multi(mappings) => mappings,
        }
    }

    pub fn unwrap_single_or(self, or: MappingBuf) -> MappingBuf {
        match self {
            MappingPatternResult::Single(mapping) => mapping,
            MappingPatternResult::Multi(_) => or,
        }
    }

    pub fn unwrap_multi_or(self, or: Vec<MappingBuf>) -> Vec<MappingBuf> {
        match self {
            MappingPatternResult::Single(_) => or,
            MappingPatternResult::Multi(mappings) => mappings,
        }
    }

    pub fn unwrap_single_or_else<F>(self, or: F) -> MappingBuf
    where
        F: FnOnce() -> MappingBuf,
    {
        match self {
            MappingPatternResult::Single(mapping) => mapping,
            MappingPatternResult::Multi(_) => or(),
        }
    }

    pub fn unwrap_multi_or_else<F>(self, or: F) -> Vec<MappingBuf>
    where
        F: FnOnce() -> Vec<MappingBuf>,
    {
        match self {
            MappingPatternResult::Single(_) => or(),
            MappingPatternResult::Multi(mappings) => mappings,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            MappingPatternResult::Single(_) => 1,
            MappingPatternResult::Multi(mappings) => mappings.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            MappingPatternResult::Single(_) => false,
            MappingPatternResult::Multi(mappings) => mappings.len() < 1,
        }
    }
}

impl IntoIterator for MappingPatternResult {
    type Item = MappingBuf;
    type IntoIter = std::vec::IntoIter<MappingBuf>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            MappingPatternResult::Single(m) => vec![m].into_iter(),
            MappingPatternResult::Multi(v) => v.into_iter(),
        }
    }
}

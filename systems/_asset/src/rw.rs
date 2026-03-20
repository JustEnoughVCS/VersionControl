use std::path::Path;

use crate::error::{DataReadError, DataWriteError};

pub trait RWData<DataType> {
    /// Implement read logic
    /// Given a path, return the specific data
    fn read(path: &Path) -> impl Future<Output = Result<DataType, DataReadError>> + Send + Sync;

    /// Implement write logic
    /// Given data and a path, write to the filesystem
    fn write(
        data: DataType,
        path: &Path,
    ) -> impl Future<Output = Result<(), DataWriteError>> + Send + Sync;

    /// Provide test data
    fn test_data() -> DataType;

    /// Given two sets of data, determine if they are equal
    ///
    /// Add RWDataTest derive to your struct to automatically generate tests
    /// ```ignore
    /// #[derive(RWDataTest)]
    /// struct FooData;
    /// ```
    fn verify_data(data_a: DataType, data_b: DataType) -> bool;
}

#[macro_export]
macro_rules! ensure_eq {
    ($a:expr, $b:expr) => {
        if $a != $b {
            return false;
        }
    };
}
// Test Data
pub struct FooData {
    pub age: i32,
    pub name: String,
}

impl RWData<FooData> for FooData {
    async fn read(path: &Path) -> Result<FooData, DataReadError> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(DataReadError::IoError)?;
        let parts: Vec<&str> = content.split('=').collect();
        if parts.len() != 2 {
            return Err(DataReadError::ParseError("Invalid format".to_string()));
        }
        let name = parts[0].to_string();
        let age: i32 = parts[1]
            .parse()
            .map_err(|_| DataReadError::ParseError("Invalid age".to_string()))?;
        Ok(FooData { age, name })
    }

    async fn write(data: FooData, path: &Path) -> Result<(), DataWriteError> {
        let content = format!("{}={}", data.name, data.age);
        tokio::fs::write(path, content)
            .await
            .map_err(DataWriteError::IoError)?;
        Ok(())
    }

    fn test_data() -> FooData {
        FooData {
            age: 24,
            name: "OneOneFourFiveOneFour".to_string(),
        }
    }

    fn verify_data(data_a: FooData, data_b: FooData) -> bool {
        crate::ensure_eq!(data_a.age, data_b.age);
        crate::ensure_eq!(data_a.name, data_b.name);
        true
    }
}

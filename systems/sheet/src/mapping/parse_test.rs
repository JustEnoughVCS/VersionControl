#[cfg(test)]
mod tests {
    use crate::{
        index_source::IndexSource,
        mapping::{LocalMapping, LocalMappingForward},
    };

    /// Helper macro for comparing two LocalMapping instances
    /// Checks equality of the mappings themselves, their forward fields, and their index sources
    macro_rules! mapping_eq {
        ($a:expr, $b:expr) => {
            assert_eq!($a, $b);
            assert_eq!($a.forward, $b.forward);
            assert_eq!($a.index_source(), $b.index_source());
        };
    }

    #[test]
    fn test_local_mapping_parse() {
        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(true, 1u32, 2u16),
            LocalMappingForward::Latest,
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\" => \"1/2\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(true, 1u32, 2u16),
            LocalMappingForward::Version { version: 2u16 },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\" == \"1/2\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(true, 1u32, 2u16),
            LocalMappingForward::Ref {
                sheet_name: "ref".to_string(),
            },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\" => \"1/2\" => \"ref\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(true, 1u32, 2u16),
            LocalMappingForward::Version { version: 3u16 },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\" => \"1/2\" == \"3\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(false, 1u32, 2u16),
            LocalMappingForward::Latest,
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\" => \"~1/2\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(false, 1u32, 2u16),
            LocalMappingForward::Version { version: 2u16 },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\" == \"~1/2\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(false, 1u32, 2u16),
            LocalMappingForward::Ref {
                sheet_name: "ref".to_string(),
            },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\" => \"~1/2\" => \"ref\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(false, 1u32, 2u16),
            LocalMappingForward::Version { version: 3u16 },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\" => \"~1/2\" == \"3\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(true, 1u32, 2u16),
            LocalMappingForward::Latest,
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"=>\"1/2\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(true, 1u32, 2u16),
            LocalMappingForward::Version { version: 2u16 },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"==\"1/2\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(true, 1u32, 2u16),
            LocalMappingForward::Ref {
                sheet_name: "ref".to_string(),
            },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"=>\"1/2\"=>\"ref\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(true, 1u32, 2u16),
            LocalMappingForward::Version { version: 3u16 },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"=>\"1/2\"==\"3\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(false, 1u32, 2u16),
            LocalMappingForward::Latest,
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"=>\"~1/2\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(false, 1u32, 2u16),
            LocalMappingForward::Version { version: 2u16 },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"==\"~1/2\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(false, 1u32, 2u16),
            LocalMappingForward::Ref {
                sheet_name: "ref".to_string(),
            },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"=>\"~1/2\"=>\"ref\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(false, 1u32, 2u16),
            LocalMappingForward::Version { version: 3u16 },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"=>\"~1/2\"==\"3\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        //

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(true, 1u32, 2u16),
            LocalMappingForward::Latest,
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"     =>\"1/2\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(true, 1u32, 2u16),
            LocalMappingForward::Version { version: 2u16 },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"==    \"1/2\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(true, 1u32, 2u16),
            LocalMappingForward::Ref {
                sheet_name: "ref".to_string(),
            },
        )
        .unwrap();
        let local_mapping_gen =
            LocalMapping::try_from("\"A.png\" =>  \"1/2\"   =>\"ref\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(true, 1u32, 2u16),
            LocalMappingForward::Version { version: 3u16 },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"=> \"1/2\"    ==\"3\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(false, 1u32, 2u16),
            LocalMappingForward::Latest,
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("   \"A.png\"=>\"~1/2\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(false, 1u32, 2u16),
            LocalMappingForward::Version { version: 2u16 },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"   ==\"~1/2\"   ").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(false, 1u32, 2u16),
            LocalMappingForward::Ref {
                sheet_name: "ref".to_string(),
            },
        )
        .unwrap();
        let local_mapping_gen =
            LocalMapping::try_from("\"A.png\"=>    \"~1/2\"=>  \"ref\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);

        let local_mapping = LocalMapping::new(
            vec!["A.png".to_string()],
            IndexSource::new(false, 1u32, 2u16),
            LocalMappingForward::Version { version: 3u16 },
        )
        .unwrap();
        let local_mapping_gen = LocalMapping::try_from("\"A.png\"=>   \"~1/2\" ==\"3\"").unwrap();
        mapping_eq!(local_mapping, local_mapping_gen);
    }
}

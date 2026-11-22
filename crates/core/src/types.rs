//! MySQL column type -> Go type mapping (spec: 类型映射).

/// Result of mapping one column type.
pub struct MappedType {
    pub go_type: String,
    pub warning: Option<String>,
}

/// Maps a MySQL column type (normalized base name + display width) to a Go
/// type. `nullable` controls pointer-ness. Spec: `decimal`->string,
/// `tinyint(1)` default int8 (bool via option), unsigned by width,
/// unknown -> any + warning.
pub fn map_type(
    base: &str,
    width: Option<u64>,
    unsigned: bool,
    nullable: bool,
    tinyint_as_bool: bool,
) -> MappedType {
    let base = base.to_ascii_lowercase();
    let go_type = match (base.as_str(), width, unsigned) {
        ("tinyint", Some(1), _) if tinyint_as_bool => "bool".to_string(),
        ("tinyint", _, true) => "uint8".to_string(),
        ("tinyint", _, false) => "int8".to_string(),
        ("smallint", _, true) => "uint16".to_string(),
        ("smallint", _, false) => "int16".to_string(),
        ("mediumint", _, true) => "uint32".to_string(),
        ("mediumint", _, false) => "int32".to_string(),
        ("int" | "integer", _, true) => "uint32".to_string(),
        ("int" | "integer", _, false) => "int32".to_string(),
        ("bigint", _, true) => "uint64".to_string(),
        ("bigint", _, false) => "int64".to_string(),
        ("decimal" | "numeric", _, _) => "string".to_string(),
        ("float", _, _) => "float32".to_string(),
        ("double" | "real", _, _) => "float64".to_string(),
        ("bool" | "boolean", _, _) => "bool".to_string(),
        (
            "varchar" | "char" | "text" | "tinytext" | "mediumtext" | "longtext" | "enum" | "set",
            _,
            _,
        ) => "string".to_string(),
        ("date" | "time" | "timestamp" | "datetime" | "year", _, _) => "time.Time".to_string(),
        ("json", _, _) => "json.RawMessage".to_string(),
        ("blob" | "tinyblob" | "mediumblob" | "longblob" | "binary" | "varbinary", _, _) => "[]byte".to_string(),
        _ => {
            return MappedType {
                go_type: "any".to_string(),
                warning: Some(format!("unsupported column type `{base}`, mapped to `any`")),
            }
        }
    };

    let go_type = if nullable {
        format!("*{go_type}")
    } else {
        go_type
    };

    MappedType {
        go_type,
        warning: None,
    }
}

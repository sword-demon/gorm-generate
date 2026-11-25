//! Column type classification and Go type mapping (spec: 类型映射).

use sqlparser::ast::DataType;

use crate::parser::ident_value;

/// Result of mapping one column type.
pub struct MappedType {
    pub go_type: String,
    pub warning: Option<String>,
    /// The Go type needs the `time` import.
    pub needs_time: bool,
    /// The Go type needs the `encoding/json` import.
    pub needs_json: bool,
}

/// Maps a normalized column type (base name + display width) to a Go type.
/// `nullable` controls pointer-ness. Spec: `decimal`->string, `tinyint(1)`
/// default int8 (bool via option), unsigned by width, unknown -> any + warning.
pub fn map_type(
    base: &str,
    width: Option<u64>,
    unsigned: bool,
    nullable: bool,
    tinyint_as_bool: bool,
) -> MappedType {
    let base = base.to_ascii_lowercase();
    let go_type = match (base.as_str(), width, unsigned) {
        ("tinyint", Some(1), _) if tinyint_as_bool => "bool",
        ("tinyint", _, true) => "uint8",
        ("tinyint", _, false) => "int8",
        ("smallint", _, true) => "uint16",
        ("smallint", _, false) => "int16",
        ("mediumint", _, true) => "uint32",
        ("mediumint", _, false) => "int32",
        ("int" | "integer", _, true) => "uint32",
        ("int" | "integer", _, false) => "int32",
        ("bigint", _, true) => "uint64",
        ("bigint", _, false) => "int64",
        ("decimal" | "numeric", _, _) => "string",
        ("float", _, _) => "float32",
        ("double" | "real", _, _) => "float64",
        ("bool" | "boolean", _, _) => "bool",
        (
            "varchar" | "char" | "text" | "tinytext" | "mediumtext" | "longtext" | "enum" | "set",
            _,
            _,
        ) => "string",
        ("date" | "time" | "timestamp" | "datetime" | "year", _, _) => "time.Time",
        ("json", _, _) => "json.RawMessage",
        ("blob" | "tinyblob" | "mediumblob" | "longblob" | "binary" | "varbinary" | "bytea", _, _) => "[]byte",
        ("uuid", _, _) => "string",
        ("number", _, _) => "string",
        _ => {
            return MappedType {
                go_type: "any".to_string(),
                warning: Some(format!("unsupported column type `{base}`, mapped to `any`")),
                needs_time: false,
                needs_json: false,
            }
        }
    };

    let needs_time = go_type == "time.Time";
    let needs_json = go_type == "json.RawMessage";
    // []byte and json.RawMessage already express nullability via nil; a
    // pointer to them adds noise, so they are never pointered.
    let pointerable = go_type != "[]byte" && go_type != "json.RawMessage";
    let go_type = if nullable && pointerable {
        format!("*{go_type}")
    } else {
        go_type.to_string()
    };

    MappedType {
        go_type,
        warning: None,
        needs_time,
        needs_json,
    }
}

/// Classifies a SQL column type: (base name, unsigned, display width).
/// Handles both MySQL and PostgreSQL `DataType` variants.
pub fn classify_mysql_type(data_type: &DataType) -> (String, bool, Option<u64>) {
    match data_type {
        DataType::Zerofill(inner) => { let (base, _, width) = classify_mysql_type(inner); (base, true, width) }
        DataType::Bit(_) | DataType::VarBit(_) => ("binary".into(), false, None),
        DataType::TinyInt(w) => ("tinyint".into(), false, *w),
        DataType::TinyIntUnsigned(w) => ("tinyint".into(), true, *w),
        DataType::SmallInt(w) | DataType::Int2(w) => ("smallint".into(), false, *w),
        DataType::SmallIntUnsigned(w) => ("smallint".into(), true, *w),
        DataType::MediumInt(w) => ("mediumint".into(), false, *w),
        DataType::MediumIntUnsigned(w) => ("mediumint".into(), true, *w),
        DataType::Int(w) | DataType::Int4(w) => ("int".into(), false, *w),
        DataType::IntUnsigned(w) => ("int".into(), true, *w),
        DataType::Integer(w) => ("int".into(), false, *w),
        DataType::IntegerUnsigned(w) => ("int".into(), true, *w),
        DataType::BigInt(w) | DataType::Int8(w) => ("bigint".into(), false, *w),
        DataType::BigIntUnsigned(w) => ("bigint".into(), true, *w),
        DataType::Decimal(_) | DataType::Dec(_) => ("decimal".into(), false, None),
        DataType::DecimalUnsigned(_) | DataType::DecUnsigned(_) => ("decimal".into(), true, None),
        DataType::Numeric(_) | DataType::BigNumeric(_) => ("numeric".into(), false, None),
        DataType::Float(_) | DataType::FloatUnsigned(_) | DataType::Float4 => ("float".into(), false, None),
        // MySQL `REAL` is a DOUBLE synonym unless REAL_AS_FLOAT is set; PG REAL
        // is 4-byte but widening to float64 is lossless, so map to double.
        DataType::Real | DataType::Float8 | DataType::Double(_) | DataType::DoubleUnsigned(_) | DataType::DoublePrecision => {
            ("double".into(), false, None)
        }
        DataType::Boolean | DataType::Bool => ("bool".into(), false, None),
        DataType::Varchar(_) | DataType::CharacterVarying(_) | DataType::CharVarying(_) | DataType::Nvarchar(_) => {
            ("varchar".into(), false, None)
        }
        DataType::Char(_) | DataType::Character(_) => ("char".into(), false, None),
        DataType::Text | DataType::TinyText | DataType::MediumText | DataType::LongText => {
            ("text".into(), false, None)
        }
        DataType::Varbinary(_) | DataType::Binary(_) | DataType::Bytea => ("binary".into(), false, None),
        DataType::Blob(_) | DataType::TinyBlob | DataType::MediumBlob | DataType::LongBlob => {
            ("blob".into(), false, None)
        }
        DataType::JSON | DataType::JSONB => ("json".into(), false, None),
        DataType::Enum(_, _) => ("enum".into(), false, None),
        DataType::Set(_) => ("set".into(), false, None),
        DataType::Datetime(_) => ("datetime".into(), false, None),
        DataType::Timestamp(_, _) => ("timestamp".into(), false, None),
        DataType::Time(_, _) => ("time".into(), false, None),
        DataType::Date => ("date".into(), false, None),
        DataType::Custom(name, _) => {
            // PG SERIAL family parses as Custom types; normalize to the
            // underlying int with an autoIncrement signal carried separately.
            let base = name
                .0
                .last()
                .map(ident_value)
                .unwrap_or_default()
                .to_lowercase();
            match base.as_str() {
                "serial" => ("int".into(), false, None),
                "smallserial" => ("smallint".into(), false, None),
                "bigserial" => ("bigint".into(), false, None),
                "uuid" => ("uuid".into(), false, None),
                "bytea" => ("bytea".into(), false, None),
                other => (other.to_string(), false, None),
            }
        }
        other => (format!("{other:?}").to_lowercase(), false, None),
    }
}

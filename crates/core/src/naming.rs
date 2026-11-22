//! Naming conversions: table/column names to Go identifiers.

/// `users` -> `User`, `user_orders` -> `UserOrder`. Always singular PascalCase.
pub fn struct_name(table: &str) -> String {
    let singular = singularize(table);
    pascal_case(&singular)
}

/// `user_id` -> `UserId`, `created_at` -> `CreatedAt`.
pub fn field_name(column: &str) -> String {
    pascal_case(column)
}

/// `UserOrder` -> `user_order`: PascalCase back to snake_case for file names.
pub fn snake_case(name: &str) -> String {
    name.chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if c.is_ascii_uppercase() && i > 0 {
                vec!['_', c.to_ascii_lowercase()]
            } else {
                vec![c.to_ascii_lowercase()]
            }
        })
        .collect()
}

/// Strips a trailing plural `s` (covers the common cases; irregular plurals
/// stay as-is, which matches the "strictly follow DDL" spirit).
fn singularize(word: &str) -> &str {
    if word.len() > 3 && word.ends_with('s') && !word.ends_with("ss") && !word.ends_with("us") {
        &word[..word.len() - 1]
    } else {
        word
    }
}

/// snake_case / kebab-case -> PascalCase. Keeps digits, splits on `_` and `-`.
fn pascal_case(name: &str) -> String {
    name.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

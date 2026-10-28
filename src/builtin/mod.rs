#[cfg(any(feature = "cli", feature = "server"))]
pub use items::*;

pub fn stdlib_source() -> &'static str {
    include_str!("stdlib.xml")
}

#[cfg(feature = "cli")]
pub fn pandoc_filter_source() -> &'static str {
    include_str!("pandoc_filter.lua")
}

// #[cfg(any(feature = "cli", feature = "server"))]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn example_document() -> &'static str {
    include_str!("example.xml")
}

#[cfg(any(feature = "cli", feature = "server"))]
mod items {
    use crate::Error;
    use std::fmt::Display;
    use std::str::FromStr;

    #[derive(Debug)]
    pub enum BuiltinItem {
        #[cfg(feature = "cli")]
        PandocFilter,
        Stdlib,
        Example,
    }

    impl FromStr for BuiltinItem {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                #[cfg(feature = "cli")]
                "pandoc-filter" => Ok(BuiltinItem::PandocFilter),
                "stdlib" => Ok(BuiltinItem::Stdlib),
                "example" => Ok(BuiltinItem::Example),
                _ => Err(Error::Parse(format!("invalid BuiltinItem: {s}"))),
            }
        }
    }

    impl Display for BuiltinItem {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let s = match self {
                #[cfg(feature = "cli")]
                BuiltinItem::PandocFilter => "pandoc-filter",
                BuiltinItem::Stdlib => "stdlib",
                BuiltinItem::Example => "example",
            };
            write!(f, "{}", s)
        }
    }

    pub fn builtin_item(item: BuiltinItem) -> &'static str {
        match item {
            BuiltinItem::Stdlib => super::stdlib_source(),
            #[cfg(feature = "cli")]
            BuiltinItem::PandocFilter => super::pandoc_filter_source(),
            BuiltinItem::Example => super::example_document(),
        }
    }

    impl BuiltinItem {
        pub fn variants() -> Vec<(String, String)> {
            vec![
                #[cfg(feature = "cli")]
                (
                    BuiltinItem::PandocFilter.to_string(),
                    "pandoc Lua filter".into(),
                ),
                (BuiltinItem::Stdlib.to_string(), "standard library".into()),
                (
                    BuiltinItem::Example.to_string(),
                    "example svgdx document".into(),
                ),
            ]
        }
    }
}

pub(crate) fn stdlib_source() -> &'static str {
    include_str!("stdlib.xml")
}

#[cfg(feature = "cli")]
pub(crate) fn pandoc_filter_source() -> &'static str {
    include_str!("pandoc_filter.lua")
}

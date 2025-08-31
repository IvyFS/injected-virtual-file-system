use win_types::BOOL;

pub(crate) mod handles;
pub(crate) mod object_attributes;
pub(crate) mod paths;
pub(crate) mod unicode_string;

pub(crate) const WIN_FALSE: BOOL = BOOL(0);

use std::fmt;

// ID PREFIX FETCH ERROR
// ================================================================================================

/// Error when Looking for a specific ID from a partial ID
#[derive(Debug, Eq, PartialEq)]
pub enum IdPrefixFetchError {
    NoMatch(String),
    MultipleMatches(String),
}

impl fmt::Display for IdPrefixFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdPrefixFetchError::NoMatch(id) => {
                write!(f, "No matches were found with the {id}.")
            },
            IdPrefixFetchError::MultipleMatches(id) => {
                write!(
                    f,
                    "Found more than one element for the provided {id} and only one match is expected."
                )
            },
        }
    }
}

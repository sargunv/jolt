/// Heuristics for classifying qualified names as Java type prefixes.
///
/// Mirrors google-java-format's `TypeNameClassifier`.
#[derive(Clone, Copy, Eq, PartialEq)]
enum JavaCaseFormat {
    Uppercase,
    Lowercase,
    UpperCamel,
    LowerCamel,
}

impl JavaCaseFormat {
    fn from_identifier(name: &str) -> Option<Self> {
        let mut chars = name.chars().filter(|ch| ch.is_alphabetic());
        let first = chars.next()?;
        let first_uppercase = first.is_uppercase();
        let mut has_uppercase = first.is_uppercase();
        let mut has_lowercase = first.is_lowercase();
        for ch in chars {
            has_uppercase |= ch.is_uppercase();
            has_lowercase |= ch.is_lowercase();
        }

        Some(if first_uppercase {
            if has_lowercase || name.chars().filter(|ch| ch.is_alphabetic()).count() == 1 {
                Self::UpperCamel
            } else {
                Self::Uppercase
            }
        } else if has_uppercase {
            Self::LowerCamel
        } else {
            Self::Lowercase
        })
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum TyParseState {
    Start,
    Type,
    FirstStaticMember,
    Reject,
    Ambiguous,
}

impl TyParseState {
    const fn is_single_unit(self) -> bool {
        matches!(self, Self::Type | Self::FirstStaticMember)
    }

    const fn next(self, case: JavaCaseFormat) -> Self {
        match (self, case) {
            (Self::Start, JavaCaseFormat::Uppercase) => Self::Ambiguous,
            (Self::Start, JavaCaseFormat::LowerCamel) => Self::Reject,
            (Self::Start, JavaCaseFormat::Lowercase) => Self::Start,
            (Self::Start, JavaCaseFormat::UpperCamel) => Self::Type,

            (
                Self::Type,
                JavaCaseFormat::Uppercase | JavaCaseFormat::LowerCamel | JavaCaseFormat::Lowercase,
            ) => Self::FirstStaticMember,
            (Self::Type, JavaCaseFormat::UpperCamel) => Self::Type,

            (Self::FirstStaticMember, _) => Self::Reject,

            (Self::Reject, _) => Self::Reject,

            (Self::Ambiguous, JavaCaseFormat::Uppercase) => Self::Ambiguous,
            (Self::Ambiguous, JavaCaseFormat::LowerCamel | JavaCaseFormat::Lowercase) => {
                Self::Reject
            }
            (Self::Ambiguous, JavaCaseFormat::UpperCamel) => Self::Type,
        }
    }
}

/// Returns the inclusive index of the longest type-shaped prefix in `name_parts`.
pub(crate) fn type_prefix_length(name_parts: &[&str]) -> Option<usize> {
    let mut state = TyParseState::Start;
    let mut type_length = None;

    for (index, name) in name_parts.iter().enumerate() {
        let Some(case) = JavaCaseFormat::from_identifier(name) else {
            break;
        };
        state = state.next(case);
        if state == TyParseState::Reject {
            break;
        }
        if state.is_single_unit() {
            type_length = Some(index);
        }
    }

    type_length
}

/// Collects simple names for type-prefix classification, stopping at the first call.
pub(crate) fn type_prefix_simple_names<'a>(
    base_simple_name: Option<&'a str>,
    members: impl IntoIterator<Item = &'a crate::analyzers::chains::ChainMember>,
) -> Vec<&'a str> {
    let mut names = Vec::new();
    if let Some(base) = base_simple_name {
        names.push(base);
    }

    for member in members {
        let Some(simple_name) = member.simple_name.as_deref() else {
            break;
        };
        names.push(simple_name);
        if member.is_call() {
            break;
        }
    }

    names
}

/// Maps a type-prefix name index to an inclusive chain member index.
pub(crate) fn type_name_prefix_member_end_index(
    base_simple_name: Option<&str>,
    members: &[crate::analyzers::chains::ChainMember],
) -> Option<usize> {
    let names = type_prefix_simple_names(base_simple_name, members.iter());
    let end = type_prefix_length(&names)?;

    if base_simple_name.is_some() {
        end.checked_sub(1)
    } else {
        Some(end)
    }
}

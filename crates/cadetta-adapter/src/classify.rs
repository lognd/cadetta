//! Three-way sync classifier (adapters.md 2): base (lock), ours (design),
//! theirs (CAD), per locator.
//!
//! Values are compared as emitted literals (native unit, declared precision),
//! so equality is exact. Pure: no I/O, the caller acts on the [`Action`].

/// Diagnostic code for a sync disagreement ("cad moved" / "both moved").
pub const R001: &str = "R001";

/// Class of one locator in the three-way compare (adapters.md 2 table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Ours and theirs both equal base: nothing to do.
    NoOp,
    /// Only the design moved: push ours.
    Push,
    /// Only the CAD moved (drift): R001 "cad moved".
    CadMoved,
    /// Both moved to the same value: record only.
    Converged,
    /// Both moved to different values (conflict): R001 "both moved".
    BothMoved,
    /// No base and theirs equals ours: record only.
    Adopt,
    /// No base and theirs differs from ours: push ours.
    FirstPush,
}

/// What the sync does for a classified locator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Nothing to do.
    Nothing,
    /// Push ours into the CAD tool.
    Push,
    /// Record the value in the lock without pushing.
    RecordOnly,
    /// No push; report R001 and exit 1.
    Refuse,
}

impl Class {
    /// Reports whether the class is an R001 disagreement.
    pub fn is_r001(self) -> bool {
        matches!(self, Self::CadMoved | Self::BothMoved)
    }

    /// Returns the action for this class; `force_design` pushes over any R001.
    pub fn action(self, force_design: bool) -> Action {
        match self {
            Self::NoOp => Action::Nothing,
            Self::Push | Self::FirstPush => Action::Push,
            Self::Converged | Self::Adopt => Action::RecordOnly,
            Self::CadMoved | Self::BothMoved if force_design => Action::Push,
            Self::CadMoved | Self::BothMoved => Action::Refuse,
        }
    }

    /// Returns the R001 message for the class, if it is an R001 class.
    pub fn r001_message(self) -> Option<&'static str> {
        match self {
            Self::CadMoved => Some("cad moved"),
            Self::BothMoved => Some("both moved"),
            _ => None,
        }
    }
}

/// Classifies one locator from its base, ours and theirs values.
pub fn classify(base: Option<&str>, ours: &str, theirs: &str) -> Class {
    let class = match base {
        None if theirs == ours => Class::Adopt,
        None => Class::FirstPush,
        Some(b) => match (ours == b, theirs == b) {
            (true, true) => Class::NoOp,
            (false, true) => Class::Push,
            (true, false) => Class::CadMoved,
            (false, false) if theirs == ours => Class::Converged,
            (false, false) => Class::BothMoved,
        },
    };
    tracing::debug!(?class, "classified locator");
    if class.is_r001() {
        tracing::warn!(code = R001, ?class, "sync disagreement");
    }
    class
}

/// Reports whether emitted text is unchanged, ignoring header lines.
///
/// A header line (it carries the IR digest and changes on any unrelated edit)
/// is one starting with `header_prefix`; it is excluded from the no-op test.
pub fn same_ignoring_header(a: &str, b: &str, header_prefix: &str) -> bool {
    let body = |s: &'_ str| -> Vec<String> {
        s.lines()
            .filter(|l| !l.starts_with(header_prefix))
            .map(str::to_owned)
            .collect()
    };
    body(a) == body(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_rows() {
        assert_eq!(classify(Some("1"), "1", "1"), Class::NoOp);
        assert_eq!(classify(Some("1"), "2", "1"), Class::Push);
        assert_eq!(classify(Some("1"), "1", "3"), Class::CadMoved);
        assert_eq!(classify(Some("1"), "2", "2"), Class::Converged);
        assert_eq!(classify(Some("1"), "2", "3"), Class::BothMoved);
        assert_eq!(classify(None, "2", "2"), Class::Adopt);
        assert_eq!(classify(None, "2", "3"), Class::FirstPush);
    }

    #[test]
    fn actions_and_r001() {
        assert_eq!(Class::NoOp.action(false), Action::Nothing);
        assert_eq!(Class::Push.action(false), Action::Push);
        assert_eq!(Class::FirstPush.action(false), Action::Push);
        assert_eq!(Class::Converged.action(false), Action::RecordOnly);
        assert_eq!(Class::Adopt.action(false), Action::RecordOnly);
        assert_eq!(Class::CadMoved.action(false), Action::Refuse);
        assert_eq!(Class::BothMoved.action(false), Action::Refuse);
        assert_eq!(Class::CadMoved.r001_message(), Some("cad moved"));
        assert_eq!(Class::BothMoved.r001_message(), Some("both moved"));
        assert_eq!(Class::Push.r001_message(), None);
    }

    #[test]
    fn force_design_pushes_over_r001() {
        assert_eq!(Class::CadMoved.action(true), Action::Push);
        assert_eq!(Class::BothMoved.action(true), Action::Push);
        assert_eq!(Class::Converged.action(true), Action::RecordOnly);
    }

    #[test]
    fn header_excluded_from_noop() {
        let a = "# digest: aaa\nx=1\n";
        let b = "# digest: bbb\nx=1\n";
        assert!(same_ignoring_header(a, b, "# digest:"));
        assert!(!same_ignoring_header(
            a,
            "# digest: bbb\nx=2\n",
            "# digest:"
        ));
    }
}

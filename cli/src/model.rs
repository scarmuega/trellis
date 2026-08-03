//! The spec's closed vocabularies (v-pinned enums) and the escalation-record
//! shape. Open registries (plan types, tags) are NOT here — they are read
//! from the instance's `conventions.md` at runtime.

use crate::markdown;
use crate::tree::Artifact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStatus {
    Draft,
    Ready,
    Active,
    Blocked,
    Retired,
}

impl PlanStatus {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "draft" => Self::Draft,
            "ready" => Self::Ready,
            "active" => Self::Active,
            "blocked" => Self::Blocked,
            "retired" => Self::Retired,
            _ => return None,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Ready => "ready",
            Self::Active => "active",
            Self::Blocked => "blocked",
            Self::Retired => "retired",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyStatus {
    Raw,
    Defined,
    Validated,
    Implemented,
    Established,
    Discarded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    Aspirational,
    Committed,
    Retired,
}

impl StrategyStatus {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "raw" => Self::Raw,
            "defined" => Self::Defined,
            "validated" => Self::Validated,
            "implemented" => Self::Implemented,
            "established" => Self::Established,
            "discarded" => Self::Discarded,
            _ => return None,
        })
    }

    pub fn band(&self) -> Band {
        match self {
            Self::Raw | Self::Defined => Band::Aspirational,
            Self::Validated | Self::Implemented | Self::Established => Band::Committed,
            Self::Discarded => Band::Retired,
        }
    }
}

impl Band {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aspirational => "aspirational",
            Self::Committed => "committed",
            Self::Retired => "retired",
        }
    }
}

/// Automation class on an `induced-by:` edge. Ordering is strictness:
/// `Core > Supporting > Generic`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Generic,
    Supporting,
    Core,
}

impl Class {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "core" => Self::Core,
            "supporting" => Self::Supporting,
            "generic" => Self::Generic,
            _ => return None,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Supporting => "supporting",
            Self::Generic => "generic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Complexity {
    Mechanical,
    Standard,
    Deep,
}

impl Complexity {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "mechanical" => Self::Mechanical,
            "standard" => Self::Standard,
            "deep" => Self::Deep,
            _ => return None,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mechanical => "mechanical",
            Self::Standard => "standard",
            Self::Deep => "deep",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    Authored,
    Generated,
    StateRef,
}

impl Provenance {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "authored" => Self::Authored,
            "generated" => Self::Generated,
            "state-ref" => Self::StateRef,
            _ => return None,
        })
    }
}

/// One fenced-yaml record under an `## Escalations` heading. Fields are read
/// leniently (line scan) — well-formedness is lint item 24's call, not a
/// parse precondition.
#[derive(Debug, Clone)]
pub struct EscalationRecord {
    pub raised: Option<String>,
    pub by: Option<String>,
    pub to: Option<String>,
    pub status: Option<String>,
    pub asks: Option<String>,
    pub attempted: Option<String>,
    pub blocked: Option<String>,
    /// 1-based line of the record's opening fence.
    pub line: u32,
}

impl EscalationRecord {
    fn from_fence(content: &str, line: u32) -> Self {
        let field = |key: &str| -> Option<String> {
            for l in content.lines() {
                if let Some(rest) = l.strip_prefix(key) {
                    if let Some(v) = rest.strip_prefix(':') {
                        let v = v.split(" #").next().unwrap_or("").trim();
                        if !v.is_empty() {
                            return Some(v.to_string());
                        }
                    }
                }
            }
            None
        };
        EscalationRecord {
            raised: field("raised"),
            by: field("by"),
            to: field("to"),
            status: field("status"),
            asks: field("asks"),
            attempted: field("attempted"),
            blocked: field("blocked"),
            line,
        }
    }
}

/// All escalation records in an artifact: fenced `yaml` blocks whose
/// enclosing section is headed `Escalations` (any level).
pub fn escalation_records(artifact: &Artifact) -> Vec<EscalationRecord> {
    let mut out = Vec::new();
    for fence in &artifact.fences {
        let Some(section) = fence.section else {
            continue;
        };
        let Some(heading) = artifact.headings.get(section) else {
            continue;
        };
        if heading.slug != markdown::slugify("Escalations") {
            continue;
        }
        if !fence.lang.eq_ignore_ascii_case("yaml") {
            continue;
        }
        out.push(EscalationRecord::from_fence(
            &fence.content,
            fence.open_line,
        ));
    }
    out
}

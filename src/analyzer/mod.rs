pub mod global_index;
pub mod framework_patterns;
pub mod workspace_analyzer;

pub use global_index::{
    GlobalSymbolIndex,
    CrateFunctionInfo,
    CrateTypeInfo,
    CrateTraitInfo,
    CrateExports,
    Visibility,
    TypeKind,
    TraitMethodInfo,
    IndexStats,
};

pub use framework_patterns::{
    FrameworkPatterns,
    EntryPointPattern,
    RuntimeCallPattern,
    TraitDispatchPattern,
    ActorPattern,
    EntryPointType,
    RuntimeCallType,
    PatternStats,
};

pub use workspace_analyzer::{
    WorkspaceAnalyzer,
    WorkspaceSnapshot,
    HybridWorkspaceAnalyzer,
    RustFunction,
    RustType,
};
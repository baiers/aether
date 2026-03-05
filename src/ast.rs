use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level program: one or more ROOT blocks
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AetherProgram {
    pub roots: Vec<RootBlock>,
}

/// A ROOT block containing an ordered list of blocks
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RootBlock {
    pub id: String,
    pub blocks: Vec<Block>,
}

/// All block types in Aether
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Block {
    Context(ContextBlock),
    Action(ActionNode),
    Request(RequestBlock),
    Failure(FailureBlock),
    Parallel(ParallelBlock),
}

/// Global or scoped context — key-value store for shared data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContextBlock {
    pub id: Option<String>,
    pub data: HashMap<String, AetherValue>,
}

/// The core execution unit
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActionNode {
    pub id: String,
    pub meta: Option<MetaBlock>,
    pub inputs: Option<Vec<InputBinding>>,
    pub condition: Option<Expr>,
    pub language: GuestLang,
    pub code: String,
    pub outputs: Option<Vec<OutputBinding>>,
    pub validation: Option<Vec<Assertion>>,
    /// Computed at parse time: which node IDs this node depends on
    pub depends_on: Vec<String>,
}

/// Parallel execution group
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParallelBlock {
    pub id: Option<String>,
    pub nodes: Vec<ActionNode>,
}

/// Agent-to-agent request
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestBlock {
    pub id: String,
    pub sender: Option<String>,
    pub target: Option<String>,
    pub context: Option<HashMap<String, AetherValue>>,
    pub instructions: Option<String>,
    pub data: HashMap<String, AetherValue>,
}

/// Failure handler
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FailureBlock {
    pub id: String,
    pub data: HashMap<String, AetherValue>,
}

// --- Sub-structures ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetaBlock {
    pub intent: Option<String>,
    pub safety: Option<SafetyLevel>,
    pub extra: HashMap<String, AetherValue>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub enum SafetyLevel {
    L0Pure,
    L1ReadOnly,
    L2StateMod,
    L3NetEgress,
    L4SystemRoot,
}

impl SafetyLevel {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pure" | "l0" => Some(Self::L0Pure),
            "read_only" | "l1" => Some(Self::L1ReadOnly),
            "state_mod" | "l2" => Some(Self::L2StateMod),
            "net_egress" | "full_egress" | "l3" => Some(Self::L3NetEgress),
            "system_root" | "l4" => Some(Self::L4SystemRoot),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::L0Pure => "L0 (Pure)",
            Self::L1ReadOnly => "L1 (Read-Only)",
            Self::L2StateMod => "L2 (State-Mod)",
            Self::L3NetEgress => "L3 (Net-Egress)",
            Self::L4SystemRoot => "L4 (System-Root)",
        }
    }

    pub fn requires_approval(&self) -> bool {
        matches!(self, Self::L3NetEgress | Self::L4SystemRoot)
    }
}

/// Input binding: maps an address to a reference or literal value
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InputBinding {
    pub address: String,
    pub alias: Option<String>,
    pub source: InputSource,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InputSource {
    Ref(String),     // Ref($0xOther)
    Literal(AetherValue),
}

/// Output binding: maps an address to a declared type
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputBinding {
    pub address: String,
    pub declared_type: AetherType,
}

/// All Aether types
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum AetherType {
    Bool,
    Int,
    Float,
    String,
    JSON,
    JSONString,
    JSONObject,
    Blob,
    Tensor,
    Ref,
    Map,
    List,
    Table,
}

impl AetherType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Bool" => Some(Self::Bool),
            "Int" => Some(Self::Int),
            "Float" => Some(Self::Float),
            "String" => Some(Self::String),
            "JSON" => Some(Self::JSON),
            "JSON_String" => Some(Self::JSONString),
            "JSON_Object" => Some(Self::JSONObject),
            "Blob" => Some(Self::Blob),
            "Tensor" => Some(Self::Tensor),
            "Ref" => Some(Self::Ref),
            "Map" => Some(Self::Map),
            "List" => Some(Self::List),
            "Table" => Some(Self::Table),
            _ => None,
        }
    }

    /// Validate that a serde_json::Value matches this type
    pub fn validate(&self, val: &serde_json::Value) -> bool {
        match self {
            Self::Bool => val.is_boolean(),
            Self::Int => val.is_i64() || val.is_u64(),
            Self::Float => val.is_f64(),
            Self::String | Self::JSONString => val.is_string(),
            Self::JSON => val.is_object() || val.is_array() || val.is_string() || val.is_number() || val.is_boolean(),
            Self::JSONObject | Self::Map => val.is_object(),
            Self::List => val.is_array(),
            Self::Blob => val.is_string(), // Base64 encoded
            Self::Tensor => val.is_array(), // N-dimensional array
            Self::Ref => val.is_string(), // Memory address
            Self::Table => val.is_array() || val.is_object(),
        }
    }
}

/// Runtime values in the Aether system
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AetherValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Json(serde_json::Value),
}

impl AetherValue {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(b) => serde_json::Value::Bool(*b),
            Self::Int(i) => serde_json::json!(*i),
            Self::Float(f) => serde_json::json!(*f),
            Self::String(s) => serde_json::Value::String(s.clone()),
            Self::Json(v) => v.clone(),
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Guest language for EXEC blocks
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum GuestLang {
    Python,
    JS,
    Rust,
    SQL,
    Shell,
    Text,
    TextGen,
    Wasm,
}

impl GuestLang {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "PYTHON" => Some(Self::Python),
            "JS" => Some(Self::JS),
            "RUST" => Some(Self::Rust),
            "SQL" => Some(Self::SQL),
            "SHELL" => Some(Self::Shell),
            "TEXT" => Some(Self::Text),
            "TEXT_GEN" => Some(Self::TextGen),
            "WASM" => Some(Self::Wasm),
            _ => None,
        }
    }
}

/// Assertion in a VALIDATE block
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Assertion {
    pub condition: Expr,
    pub on_fail: HaltAction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum HaltAction {
    Halt,
    Retry(Option<u32>), // Optional max retries
    Warn,
}

/// Expression AST — used in VALIDATE and CONDITION blocks
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Expr {
    // Literals
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),

    // References
    Address(String),    // $0x1, $0xRAW
    Identifier(String), // variable names

    // Operations
    BinOp {
        left: Box<Expr>,
        op: BinOperator,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expr>,
    },

    // Access
    Index {
        object: Box<Expr>,
        key: Box<Expr>,
    },
    DotAccess {
        object: Box<Expr>,
        field: String,
    },

    // Function call
    FuncCall {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BinOperator {
    Eq, Ne, Gt, Lt, Ge, Le,
    Add, Sub, Mul, Div, Mod,
    And, Or,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UnaryOperator {
    Not,
    Neg,
}

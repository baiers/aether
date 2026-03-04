/// Aether Standard Library (ASL) Registry
/// Machine-readable registry of canonical intents, their required safety levels,
/// and recommended guest languages. Embedded at compile time.

use serde::{Deserialize, Serialize};

// Embedded registry — always available, no runtime file dependency
static REGISTRY_JSON: &str = include_str!("../asl/registry.json");

// =============================================================================
// Types
// =============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AslEntry {
    pub id: String,
    pub description: String,
    /// Canonical safety level: "l0", "l1", "l2", "l3", "l4"
    pub safety: String,
    pub recommended_lang: Option<String>,
    pub namespace: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AslRegistry {
    pub version: String,
    pub entries: Vec<AslEntry>,
}

/// Result of checking an intent against the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryCheck {
    /// The matched ASL entry ID, if found
    pub matched_id: Option<String>,
    /// Any warning (safety mismatch, unknown intent)
    pub warning: Option<String>,
}

// =============================================================================
// Registry
// =============================================================================

impl AslRegistry {
    /// Load the embedded registry (panics only if the embedded JSON is malformed,
    /// which would be caught at compile-time CI)
    pub fn load() -> Self {
        serde_json::from_str(REGISTRY_JSON)
            .expect("Embedded ASL registry is invalid — this is a build error")
    }

    /// Look up an intent by its full dotted ID (e.g. "std.io.net_get")
    pub fn lookup(&self, intent: &str) -> Option<&AslEntry> {
        self.entries.iter().find(|e| e.id == intent)
    }

    /// List all entries in a namespace prefix (e.g. "std.io")
    pub fn namespace(&self, prefix: &str) -> Vec<&AslEntry> {
        self.entries.iter()
            .filter(|e| e.namespace == prefix)
            .collect()
    }

    /// Validate an intent against the registry.
    /// Returns a RegistryCheck with:
    ///   - matched_id: Some(id) if found, None if unknown
    ///   - warning: Some(msg) if safety mismatch or intent unknown
    ///
    /// Unknown intents produce a warning but are NOT blocked — custom intents
    /// are valid and encouraged. Only safety mismatches against known entries warn.
    pub fn check(&self, intent: &str, declared_safety: &str) -> RegistryCheck {
        match self.lookup(intent) {
            Some(entry) => {
                let safety_matches = entry.safety.to_lowercase()
                    == declared_safety.to_lowercase();

                let warning = if !safety_matches {
                    Some(format!(
                        "ASL safety mismatch: intent '{}' recommends '{}', declared '{}'",
                        intent, entry.safety.to_uppercase(), declared_safety.to_uppercase()
                    ))
                } else {
                    None
                };

                RegistryCheck {
                    matched_id: Some(entry.id.clone()),
                    warning,
                }
            }
            None => {
                // Not a registered std.* intent — fine for custom intents,
                // but flag std.* unknowns as potential typos
                let warning = if intent.starts_with("std.") {
                    Some(format!(
                        "ASL: intent '{}' looks like a std namespace entry but is not registered",
                        intent
                    ))
                } else {
                    None // Custom intents are silently accepted
                };

                RegistryCheck {
                    matched_id: None,
                    warning,
                }
            }
        }
    }
}

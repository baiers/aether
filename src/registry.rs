//! Aether Standard Library (ASL) Registry
//! Machine-readable registry of canonical intents, their required safety levels,
//! and recommended guest languages. Embedded at compile time.

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
    /// Any warning (safety mismatch)
    pub warning: Option<String>,
    /// Hard error — unknown `std.*` intent or strict mode violation
    pub is_error: bool,
    /// Error message when is_error is true
    pub error_message: Option<String>,
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
        self.entries
            .iter()
            .filter(|e| e.namespace == prefix)
            .collect()
    }

    /// Validate an intent against the registry.
    ///
    /// - `strict`: when true, unknown intents in ANY namespace are hard errors.
    ///   When false (default), only unknown `std.*` intents are errors; custom
    ///   namespaces are silently accepted.
    ///
    /// Returns a RegistryCheck with:
    ///   - matched_id: Some(id) if found
    ///   - warning: Some(msg) on safety mismatch
    ///   - is_error / error_message: set for unknown std.* or strict violations
    pub fn check(&self, intent: &str, declared_safety: &str, strict: bool) -> RegistryCheck {
        match self.lookup(intent) {
            Some(entry) => {
                let safety_matches = entry.safety.to_lowercase() == declared_safety.to_lowercase();

                let warning = if !safety_matches {
                    Some(format!(
                        "ASL safety mismatch: intent '{}' recommends '{}', declared '{}'",
                        intent,
                        entry.safety.to_uppercase(),
                        declared_safety.to_uppercase()
                    ))
                } else {
                    None
                };

                RegistryCheck {
                    matched_id: Some(entry.id.clone()),
                    warning,
                    is_error: false,
                    error_message: None,
                }
            }
            None => {
                let is_std = intent.starts_with("std.");

                if is_std {
                    // Unknown std.* intents are always hard errors — the std
                    // namespace is Aether-owned; an unknown entry is a typo or
                    // LLM hallucination that must not execute.
                    RegistryCheck {
                        matched_id: None,
                        warning: None,
                        is_error: true,
                        error_message: Some(format!(
                            "Unknown std.* intent '{}'. Check the ASL registry or use a custom namespace.",
                            intent
                        )),
                    }
                } else if strict {
                    // Strict mode: reject any unregistered intent
                    RegistryCheck {
                        matched_id: None,
                        warning: None,
                        is_error: true,
                        error_message: Some(format!(
                            "Unregistered intent '{}' rejected (--strict-registry mode).",
                            intent
                        )),
                    }
                } else {
                    // Custom namespaces silently accepted
                    RegistryCheck {
                        matched_id: None,
                        warning: None,
                        is_error: false,
                        error_message: None,
                    }
                }
            }
        }
    }
}

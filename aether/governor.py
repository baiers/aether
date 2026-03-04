"""
Aether Kernel - Governor Module
Safety Level enforcement and resource auditing
"""
from dataclasses import dataclass
from typing import Dict, List, Optional, Set
from enum import IntEnum

from .parser import AetherProgram, RootBlock, ActionNode


class SafetyLevel(IntEnum):
    """Safety levels for Aether nodes (L0-L4)."""
    L0_PURE = 0       # No side effects, no I/O
    L1_READ_ONLY = 1  # File system/DB read access only
    L2_STATE_MOD = 2  # Local state modifications allowed
    L3_NET_EGRESS = 3 # Network calls allowed
    L4_SYSTEM_ROOT = 4 # Full system access


# Map safety tags to levels
SAFETY_TAG_MAP = {
    'pure': SafetyLevel.L0_PURE,
    'pure_compute': SafetyLevel.L0_PURE,
    'read_only': SafetyLevel.L1_READ_ONLY,
    'read': SafetyLevel.L1_READ_ONLY,
    'state_mod': SafetyLevel.L2_STATE_MOD,
    'filesystem_write': SafetyLevel.L2_STATE_MOD,
    'filesystem_write_create': SafetyLevel.L2_STATE_MOD,
    'net_egress': SafetyLevel.L3_NET_EGRESS,
    'network_egress': SafetyLevel.L3_NET_EGRESS,
    'full_egress': SafetyLevel.L3_NET_EGRESS,
    'system_root': SafetyLevel.L4_SYSTEM_ROOT,
    'system_audit': SafetyLevel.L4_SYSTEM_ROOT,
}


@dataclass
class GovernancePolicy:
    """Policy for governing Aether execution."""
    max_safety_level: SafetyLevel = SafetyLevel.L3_NET_EGRESS
    allowed_languages: Set[str] = None
    require_intent: bool = True
    max_budget_usd: float = 1.0
    
    def __post_init__(self):
        if self.allowed_languages is None:
            self.allowed_languages = {'PYTHON', 'JS', 'SHELL', 'SQL', 'RUST', 'WASM'}


@dataclass
class GovernanceViolation:
    """Represents a governance policy violation."""
    node_id: str
    violation_type: str
    message: str
    severity: str  # 'error', 'warning'


class Governor:
    """
    Enforces safety policies on Aether programs.
    Pre-flight checks before execution.
    """
    
    def __init__(self, policy: GovernancePolicy = None):
        self.policy = policy or GovernancePolicy()
        self.violations: List[GovernanceViolation] = []
    
    def audit_program(self, program: AetherProgram) -> List[GovernanceViolation]:
        """
        Audit an Aether program for governance violations.
        Returns list of violations found.
        """
        self.violations = []
        
        for root in program.roots:
            self._audit_root(root)
        
        return self.violations
    
    def _audit_root(self, root: RootBlock) -> None:
        """Audit a root block."""
        for block in root.blocks:
            if isinstance(block, ActionNode):
                self._audit_action(block)
    
    def _audit_action(self, node: ActionNode) -> None:
        """Audit an action node for policy violations."""
        # Check for required intent
        if self.policy.require_intent:
            intent = node.meta.get('_intent')
            if not intent or intent == 'Unknown intent':
                self.violations.append(GovernanceViolation(
                    node_id=node.id,
                    violation_type='missing_intent',
                    message='Action node is missing required _intent metadata',
                    severity='error'
                ))
        
        # Check safety level
        safety_tag = node.meta.get('_safety', 'unknown').lower()
        node_safety = SAFETY_TAG_MAP.get(safety_tag, SafetyLevel.L4_SYSTEM_ROOT)
        
        if node_safety > self.policy.max_safety_level:
            self.violations.append(GovernanceViolation(
                node_id=node.id,
                violation_type='safety_level_exceeded',
                message=f'Node requires L{node_safety} but policy allows max L{self.policy.max_safety_level}',
                severity='error'
            ))
        
        # Check language allowlist
        if node.language and node.language.upper() not in self.policy.allowed_languages:
            self.violations.append(GovernanceViolation(
                node_id=node.id,
                violation_type='language_not_allowed',
                message=f'Language "{node.language}" is not in allowed list',
                severity='error'
            ))
        
        # Warn about high-risk operations
        if node_safety >= SafetyLevel.L3_NET_EGRESS:
            self.violations.append(GovernanceViolation(
                node_id=node.id,
                violation_type='high_risk_operation',
                message=f'Node has L{node_safety} safety level - network/system access',
                severity='warning'
            ))
    
    def is_allowed(self) -> bool:
        """Check if program is allowed to execute (no error-level violations)."""
        return not any(v.severity == 'error' for v in self.violations)
    
    def get_safety_summary(self, program: AetherProgram) -> Dict:
        """Get a summary of safety levels in the program."""
        summary = {
            'total_nodes': 0,
            'by_level': {f'L{i}': 0 for i in range(5)},
            'high_risk_nodes': [],
            'missing_safety': []
        }
        
        for root in program.roots:
            for block in root.blocks:
                if isinstance(block, ActionNode):
                    summary['total_nodes'] += 1
                    safety_tag = block.meta.get('_safety', '').lower()
                    
                    if not safety_tag:
                        summary['missing_safety'].append(block.id)
                        summary['by_level']['L4'] += 1  # Assume worst case
                    else:
                        level = SAFETY_TAG_MAP.get(safety_tag, SafetyLevel.L4_SYSTEM_ROOT)
                        summary['by_level'][f'L{level}'] += 1
                        
                        if level >= SafetyLevel.L3_NET_EGRESS:
                            summary['high_risk_nodes'].append(block.id)
        
        return summary


def preflight_check(program: AetherProgram, policy: GovernancePolicy = None) -> tuple[bool, List[GovernanceViolation]]:
    """
    Perform pre-flight governance check on a program.
    Returns (is_allowed, violations).
    """
    governor = Governor(policy)
    violations = governor.audit_program(program)
    return governor.is_allowed(), violations

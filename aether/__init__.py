"""
Aether Kernel - Python Prototype
Package initialization
"""
from .parser import parse_aether, AetherProgram, ActionNode, RootBlock, ContextBlock
from .executor import execute_program, ExecutionLog, log_to_json
from .dag import DependencyGraph, validate_dag
from .governor import Governor, GovernancePolicy, SafetyLevel, preflight_check

__version__ = "0.1.0"
__all__ = [
    "parse_aether",
    "execute_program", 
    "log_to_json",
    "AetherProgram",
    "ActionNode",
    "RootBlock",
    "ContextBlock",
    "ExecutionLog",
    "DependencyGraph",
    "validate_dag",
    "Governor",
    "GovernancePolicy",
    "SafetyLevel",
    "preflight_check",
]

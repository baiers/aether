#!/usr/bin/env python3
"""
Aether Kernel CLI - Main entry point
Usage: python -m aether <file.ae>
"""
import sys
import json
from pathlib import Path

from aether import (
    parse_aether, execute_program, log_to_json,
    validate_dag, preflight_check, GovernancePolicy, SafetyLevel
)


def main():
    if len(sys.argv) < 2:
        print("Aether Kernel v0.1.0")
        print("Usage: python -m aether <file.ae>")
        print("       python main.py <file.ae>")
        sys.exit(1)
    
    file_path = Path(sys.argv[1])
    if not file_path.exists():
        print(f"Error: File not found: {file_path}")
        sys.exit(1)
    
    print(f"Aether Kernel v0.1.0")
    print(f"=" * 40)
    print(f"Parsing: {file_path}")
    
    try:
        source = file_path.read_text(encoding='utf-8')
        program = parse_aether(source)
        print(f"  Found {len(program.roots)} root block(s)")
        
        # Phase 1: DAG Validation
        print(f"\n[Phase 1] DAG Validation...")
        is_dag_valid, dag_error = validate_dag(program)
        if not is_dag_valid:
            print(f"  ✗ DAG Error: {dag_error}")
            sys.exit(1)
        print(f"  ✓ No cycles detected")
        
        # Phase 2: Governance Pre-Flight
        print(f"\n[Phase 2] Governance Pre-Flight...")
        policy = GovernancePolicy(
            max_safety_level=SafetyLevel.L3_NET_EGRESS,
            require_intent=True
        )
        is_allowed, violations = preflight_check(program, policy)
        
        errors = [v for v in violations if v.severity == 'error']
        warnings = [v for v in violations if v.severity == 'warning']
        
        if warnings:
            for w in warnings:
                print(f"  ⚠ {w.node_id}: {w.message}")
        
        if errors:
            for e in errors:
                print(f"  ✗ {e.node_id}: {e.message}")
            print(f"\n  Governance check failed with {len(errors)} error(s)")
            sys.exit(1)
        
        print(f"  ✓ Governance check passed ({len(warnings)} warning(s))")
        
        # Phase 3: Execution
        print(f"\n[Phase 3] Executing...")
        log = execute_program(program)
        
        print(f"\n{'=' * 40}")
        print(f"Execution Log:")
        print(log_to_json(log))
        
        # Save log to file
        output_path = file_path.with_suffix('.ae.json')
        output_path.write_text(log_to_json(log))
        print(f"\nLog saved to: {output_path}")
        
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()

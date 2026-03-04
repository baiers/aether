"""
Aether Kernel - Python Prototype
Executor module - runs parsed Aether programs
"""
import subprocess
import json
import uuid
from datetime import datetime
from dataclasses import dataclass, field, asdict
from typing import List, Dict, Any, Optional

from .parser import AetherProgram, ActionNode, ContextBlock, RootBlock


@dataclass
class NodeTrace:
    node: str
    intent: str
    status: str
    duration_ms: int
    output: Dict[str, Any] = field(default_factory=dict)


@dataclass
class ExecutionLog:
    execution_id: str
    host_agent: str
    timestamp_start: str
    global_status: str
    memory_context: Dict[str, str] = field(default_factory=dict)
    trace: List[NodeTrace] = field(default_factory=list)
    total_duration_ms: int = 0


class AetherExecutor:
    """Execute Aether programs."""
    
    def __init__(self):
        self.memory: Dict[str, Any] = {}
        self.trace: List[NodeTrace] = []
    
    def execute(self, program: AetherProgram) -> ExecutionLog:
        """Execute an Aether program and return the execution log."""
        start_time = datetime.now()
        execution_id = f"exe_{uuid.uuid4().hex[:12]}"
        
        for root in program.roots:
            self._execute_root(root)
        
        end_time = datetime.now()
        duration = int((end_time - start_time).total_seconds() * 1000)
        
        return ExecutionLog(
            execution_id=execution_id,
            host_agent="Aether_Kernel_Python_v0.1",
            timestamp_start=start_time.isoformat(),
            global_status="SUCCESS" if all(t.status == "COMPLETED" for t in self.trace) else "PARTIAL",
            memory_context=dict(self.memory),
            trace=self.trace,
            total_duration_ms=duration
        )
    
    def _execute_root(self, root: RootBlock):
        """Execute a root block."""
        print(f"  [ROOT] {root.id}")
        for block in root.blocks:
            if isinstance(block, ContextBlock):
                self._execute_context(block)
            elif isinstance(block, ActionNode):
                self._execute_action(block)
    
    def _execute_context(self, ctx: ContextBlock):
        """Load context variables into memory."""
        for key, value in ctx.data.items():
            # Strip the $ prefix if present
            clean_key = key.lstrip('$')
            self.memory[clean_key] = value.strip('"')
        print(f"    [CTX] Loaded {len(ctx.data)} variables")
    
    def _execute_action(self, node: ActionNode):
        """Execute an action node."""
        intent = node.meta.get('_intent', 'Unknown intent')
        safety = node.meta.get('_safety', 'unknown')
        
        print(f"    [ACT] {node.id} - {intent} (safety: {safety})")
        
        start = datetime.now()
        
        # Execute based on language
        result = self._dispatch_execution(node)
        
        end = datetime.now()
        duration = int((end - start).total_seconds() * 1000)
        
        trace = NodeTrace(
            node=node.id,
            intent=intent,
            status="COMPLETED" if result.get('success', True) else "FAILED",
            duration_ms=duration,
            output=result
        )
        self.trace.append(trace)
        
        # Store output in memory
        for out_key in node.outputs.keys():
            self.memory[out_key.lstrip('$')] = result
    
    def _dispatch_execution(self, node: ActionNode) -> Dict[str, Any]:
        """Dispatch execution to the appropriate guest language runner."""
        lang = node.language.upper()
        code = node.code
        
        # Substitute memory references in code
        for key, value in self.memory.items():
            code = code.replace(f"${key}", json.dumps(value) if isinstance(value, (dict, list)) else str(value))
        
        if lang == "PYTHON":
            return self._run_python(code)
        elif lang == "SHELL":
            return self._run_shell(code)
        elif lang in ("JS", "JAVASCRIPT"):
            return self._run_js(code)
        else:
            # Mock execution for unsupported languages
            return {"success": True, "mocked": True, "language": lang}
    
    def _run_python(self, code: str) -> Dict[str, Any]:
        """Execute Python code."""
        try:
            # Create a simple execution environment
            local_vars = {'result': None, 'memory': self.memory}
            exec(code, {'__builtins__': __builtins__}, local_vars)
            return {"success": True, "result": local_vars.get('result')}
        except Exception as e:
            return {"success": False, "error": str(e)}
    
    def _run_shell(self, code: str) -> Dict[str, Any]:
        """Execute shell command."""
        try:
            result = subprocess.run(code, shell=True, capture_output=True, text=True, timeout=30)
            return {
                "success": result.returncode == 0,
                "stdout": result.stdout,
                "stderr": result.stderr,
                "returncode": result.returncode
            }
        except subprocess.TimeoutExpired:
            return {"success": False, "error": "Command timed out"}
        except Exception as e:
            return {"success": False, "error": str(e)}
    
    def _run_js(self, code: str) -> Dict[str, Any]:
        """Execute JavaScript using Bun or Node."""
        try:
            # Try Bun first, fall back to Node
            for runtime in ["bun", "node"]:
                try:
                    result = subprocess.run(
                        [runtime, "-e", code],
                        capture_output=True, text=True, timeout=30
                    )
                    return {
                        "success": result.returncode == 0,
                        "runtime": runtime,
                        "stdout": result.stdout,
                        "stderr": result.stderr
                    }
                except FileNotFoundError:
                    continue
            return {"success": False, "error": "No JS runtime found (tried bun, node)"}
        except Exception as e:
            return {"success": False, "error": str(e)}


def execute_program(program: AetherProgram) -> ExecutionLog:
    """Convenience function to execute an Aether program."""
    executor = AetherExecutor()
    return executor.execute(program)


def log_to_json(log: ExecutionLog) -> str:
    """Convert execution log to JSON string."""
    def serialize(obj):
        if hasattr(obj, '__dataclass_fields__'):
            return asdict(obj)
        return str(obj)
    
    return json.dumps(asdict(log), indent=2, default=serialize)

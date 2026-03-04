"""
Aether Kernel - DAG Module
Dependency graph management and cycle detection
"""
from dataclasses import dataclass, field
from typing import Dict, List, Set, Optional, Any
from collections import defaultdict
import re

from .parser import AetherProgram, RootBlock, ActionNode, ContextBlock


@dataclass
class DependencyNode:
    """Represents a node in the dependency graph."""
    id: str
    dependencies: Set[str] = field(default_factory=set)
    outputs: Set[str] = field(default_factory=set)


class DependencyGraph:
    """
    Manages the dependency graph for an Aether program.
    Detects cycles and provides topological ordering.
    """
    
    def __init__(self):
        self.nodes: Dict[str, DependencyNode] = {}
        self.memory_providers: Dict[str, str] = {}  # memory_address -> node_id
    
    def build_from_program(self, program: AetherProgram) -> None:
        """Build the dependency graph from an Aether program."""
        for root in program.roots:
            self._process_root(root)
    
    def _process_root(self, root: RootBlock) -> None:
        """Process a root block and extract dependencies."""
        for block in root.blocks:
            if isinstance(block, ContextBlock):
                # Context blocks define global memory
                for key in block.data.keys():
                    clean_key = key.lstrip('$')
                    self.memory_providers[clean_key] = f"CTX_{root.id}"
            elif isinstance(block, ActionNode):
                self._process_action(block)
    
    def _process_action(self, node: ActionNode) -> None:
        """Process an action node and extract its dependencies."""
        dep_node = DependencyNode(id=node.id)
        
        # Extract input dependencies
        for key, value in node.inputs.items():
            # Check for Ref() dependencies
            ref_match = re.match(r'Ref\((\$[0-9a-zA-Z_]+)\)', value)
            if ref_match:
                ref_addr = ref_match.group(1).lstrip('$')
                dep_node.dependencies.add(ref_addr)
        
        # Extract code dependencies (addresses used in code)
        code = node.code
        addr_pattern = r'\$([0-9a-zA-Z_]+)'
        for match in re.finditer(addr_pattern, code):
            dep_node.dependencies.add(match.group(1))
        
        # Register outputs
        for key in node.outputs.keys():
            clean_key = key.lstrip('$')
            dep_node.outputs.add(clean_key)
            self.memory_providers[clean_key] = node.id
        
        self.nodes[node.id] = dep_node
    
    def detect_cycles(self) -> Optional[List[str]]:
        """
        Detect cycles in the dependency graph using DFS.
        Returns the cycle path if found, None otherwise.
        """
        visited = set()
        rec_stack = set()
        path = []
        
        def dfs(node_id: str) -> Optional[List[str]]:
            visited.add(node_id)
            rec_stack.add(node_id)
            path.append(node_id)
            
            node = self.nodes.get(node_id)
            if not node:
                path.pop()
                rec_stack.remove(node_id)
                return None
            
            for dep in node.dependencies:
                # Find which node provides this dependency
                provider = self.memory_providers.get(dep)
                if provider and provider in self.nodes:
                    if provider not in visited:
                        cycle = dfs(provider)
                        if cycle:
                            return cycle
                    elif provider in rec_stack:
                        # Found a cycle!
                        cycle_start = path.index(provider)
                        return path[cycle_start:] + [provider]
            
            path.pop()
            rec_stack.remove(node_id)
            return None
        
        for node_id in self.nodes:
            if node_id not in visited:
                cycle = dfs(node_id)
                if cycle:
                    return cycle
        
        return None
    
    def topological_sort(self) -> List[str]:
        """
        Return nodes in topological order (dependencies first).
        Raises ValueError if a cycle is detected.
        """
        cycle = self.detect_cycles()
        if cycle:
            raise ValueError(f"Cycle detected: {' -> '.join(cycle)}")
        
        visited = set()
        order = []
        
        def dfs(node_id: str):
            if node_id in visited:
                return
            visited.add(node_id)
            
            node = self.nodes.get(node_id)
            if node:
                for dep in node.dependencies:
                    provider = self.memory_providers.get(dep)
                    if provider and provider in self.nodes:
                        dfs(provider)
            
            order.append(node_id)
        
        for node_id in self.nodes:
            dfs(node_id)
        
        return order
    
    def get_parallel_groups(self) -> List[List[str]]:
        """
        Group nodes that can be executed in parallel.
        Returns list of groups where each group can run concurrently.
        """
        order = self.topological_sort()
        groups = []
        executed = set()
        
        while len(executed) < len(self.nodes):
            # Find all nodes whose dependencies are satisfied
            ready = []
            for node_id in order:
                if node_id in executed:
                    continue
                node = self.nodes[node_id]
                deps_satisfied = all(
                    self.memory_providers.get(dep) in executed or 
                    self.memory_providers.get(dep, '').startswith('CTX_')
                    for dep in node.dependencies
                )
                if deps_satisfied:
                    ready.append(node_id)
            
            if not ready:
                break
            
            groups.append(ready)
            executed.update(ready)
        
        return groups


def validate_dag(program: AetherProgram) -> tuple[bool, Optional[str]]:
    """
    Validate that a program has no dependency cycles.
    Returns (is_valid, error_message).
    """
    dag = DependencyGraph()
    dag.build_from_program(program)
    
    cycle = dag.detect_cycles()
    if cycle:
        return False, f"Dependency cycle detected: {' -> '.join(cycle)}"
    
    return True, None

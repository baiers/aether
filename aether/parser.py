"""
Aether Kernel - Python Prototype
Parser module using Lark grammar
"""
from lark import Lark, Transformer, v_args
from pathlib import Path
from dataclasses import dataclass, field
from typing import Optional, Dict, List, Any

# Load grammar
GRAMMAR_PATH = Path(__file__).parent / "aether.lark"

@dataclass
class ActionNode:
    id: str
    meta: Dict[str, str] = field(default_factory=dict)
    inputs: Dict[str, str] = field(default_factory=dict)
    condition: Optional[str] = None
    language: str = ""
    code: str = ""
    outputs: Dict[str, str] = field(default_factory=dict)
    validation: List[str] = field(default_factory=list)

@dataclass
class ContextBlock:
    id: Optional[str] = None
    data: Dict[str, str] = field(default_factory=dict)

@dataclass
class RequestBlock:
    id: str
    data: Dict[str, str] = field(default_factory=dict)

@dataclass
class RootBlock:
    id: str
    blocks: List[Any] = field(default_factory=list)

@dataclass
class AetherProgram:
    roots: List[RootBlock] = field(default_factory=list)


class AetherTransformer(Transformer):
    """Transform the parse tree into Aether AST objects."""
    
    def start(self, items):
        return AetherProgram(roots=list(items))
    
    def root_block(self, items):
        hash_id = str(items[0])
        blocks = [b for b in items[1:] if b is not None]
        return RootBlock(id=hash_id, blocks=blocks)
    
    def block(self, items):
        return items[0]
    
    def context_block(self, items):
        ctx = ContextBlock()
        for item in items:
            if isinstance(item, str) and item.startswith("0x"):
                ctx.id = item
            elif isinstance(item, tuple):
                ctx.data[item[0]] = item[1]
        return ctx
    
    def action_node(self, items):
        node = ActionNode(id=str(items[0]))
        if len(items) > 1 and items[1]:
            content = items[1]
            if isinstance(content, dict):
                node.meta = content.get('meta', {})
                node.inputs = content.get('inputs', {})
                node.condition = content.get('condition')
                node.language = content.get('language', '')
                node.code = content.get('code', '')
                node.outputs = content.get('outputs', {})
                node.validation = content.get('validation', [])
        return node
    
    def act_content(self, items):
        content = {'meta': {}, 'inputs': {}, 'outputs': {}, 'validation': []}
        for item in items:
            if isinstance(item, dict):
                if 'meta' in item:
                    content['meta'] = item['meta']
                elif 'inputs' in item:
                    content['inputs'] = item['inputs']
                elif 'outputs' in item:
                    content['outputs'] = item['outputs']
                elif 'exec' in item:
                    content['language'] = item['exec']['lang']
                    content['code'] = item['exec']['code']
                elif 'condition' in item:
                    content['condition'] = item['condition']
            elif isinstance(item, list):
                content['validation'] = item
        return content
    
    def meta_block(self, items):
        return {'meta': dict(items)}
    
    def in_block(self, items):
        return {'inputs': dict(items)}
    
    def out_block(self, items):
        return {'outputs': dict(items)}
    
    def exec_block(self, items):
        return {'exec': {'lang': str(items[0]), 'code': str(items[1]).strip()}}
    
    def condition_block(self, items):
        return {'condition': str(items[0])}
    
    def val_block(self, items):
        return list(items)
    
    def pair(self, items):
        return (str(items[0]), str(items[1]).strip('"'))
    
    def ref_pair(self, items):
        return (str(items[0]), f"Ref({items[1]})")
    
    def type_pair(self, items):
        return (str(items[0]), f"Type<{items[1]}>")
    
    def assertion(self, items):
        return f"ASSERT {items[0]} OR {items[-1]}"
    
    def expression(self, items):
        return str(items[0])
    
    def comparison(self, items):
        return f"{items[0]} {items[1]} {items[2]}"
    
    def value(self, items):
        return items[0]
    
    def code_payload(self, items):
        return str(items[0]) if items else ""
    
    def request_block(self, items):
        return RequestBlock(id=str(items[0]), data=dict(items[1:]))
    
    def failure_block(self, items):
        return RequestBlock(id=str(items[0]), data=dict(items[1:]))
    
    # Terminal transformations
    def HASH(self, token):
        return str(token)
    
    def ADDRESS(self, token):
        return str(token)
    
    def STRING(self, token):
        return str(token)
    
    def NUMBER(self, token):
        return str(token)
    
    def BOOLEAN(self, token):
        return str(token)
    
    def KEY(self, token):
        return str(token)
    
    def LANG(self, token):
        return str(token)
    
    def TYPE_NAME(self, token):
        return str(token)


def create_parser() -> Lark:
    """Create and return the Aether parser."""
    grammar = GRAMMAR_PATH.read_text(encoding='utf-8')
    return Lark(grammar, parser='earley')


def parse_aether(source: str) -> AetherProgram:
    """Parse Aether source code and return an AetherProgram."""
    parser = create_parser()
    tree = parser.parse(source)
    transformer = AetherTransformer()
    return transformer.transform(tree)

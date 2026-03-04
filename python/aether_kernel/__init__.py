"""
Aether Kernel — AI-native pipeline language and runtime.

This package bundles the pre-built Aether binaries and provides a Python API.

Usage:
    # CLI (installed as entry point)
    aether examples/demo.ae

    # Python API
    from aether_kernel import execute, validate
    result = execute("examples/demo.ae")
    errors = validate("examples/demo.ae")
"""

__version__ = "0.3.1"

import json
import subprocess
import sys
from pathlib import Path


def _find_binary(name: str) -> Path:
    """Locate the bundled native binary for the current platform."""
    bin_dir = Path(__file__).parent / "bin"

    if sys.platform == "win32":
        binary = bin_dir / f"{name}.exe"
    else:
        binary = bin_dir / name

    if not binary.exists():
        raise FileNotFoundError(
            f"Aether binary '{name}' not found at {binary}. "
            f"This platform may not be supported. "
            f"Try building from source: cargo build --release"
        )

    return binary


def execute(
    path: str,
    *,
    safety: int | None = None,
    no_registry: bool = False,
    timeout: int = 60,
) -> dict:
    """Execute an Aether pipeline and return the result as a dict.

    Args:
        path: Path to the .ae or .as file
        safety: Safety auto-approve level (0-4)
        no_registry: Disable ASL registry checks
        timeout: Execution timeout in seconds

    Returns:
        Dict with 'ledger', 'traces', and 'status' keys

    Raises:
        subprocess.CalledProcessError: If execution fails
        FileNotFoundError: If the Aether binary is not found
    """
    binary = _find_binary("aether")
    cmd = [str(binary), path, "--json"]

    if safety is not None:
        cmd.extend(["--safety", str(safety)])
    if no_registry:
        cmd.append("--no-registry")

    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=timeout,
    )

    if result.returncode != 0:
        raise subprocess.CalledProcessError(
            result.returncode, cmd, result.stdout, result.stderr
        )

    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {"raw_output": result.stdout, "stderr": result.stderr}


def validate(path: str) -> list[str]:
    """Validate an Aether file and return any errors.

    Args:
        path: Path to the .ae or .as file

    Returns:
        List of error strings (empty if valid)
    """
    binary = _find_binary("aether")
    result = subprocess.run(
        [str(binary), path, "--validate-only"],
        capture_output=True,
        text=True,
    )

    if result.returncode == 0:
        return []

    return [line for line in result.stderr.splitlines() if line.strip()]


def expand(path: str) -> str:
    """Expand an Aether-Short (.as) file to full .ae syntax.

    Args:
        path: Path to the .as file

    Returns:
        Expanded .ae source string
    """
    binary = _find_binary("aether")
    result = subprocess.run(
        [str(binary), path, "--expand-only"],
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        raise subprocess.CalledProcessError(
            result.returncode,
            [str(binary), path, "--expand-only"],
            result.stdout,
            result.stderr,
        )

    return result.stdout

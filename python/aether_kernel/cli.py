"""CLI entry points — thin wrappers that exec the bundled native binaries."""

import os
import sys

from . import _find_binary


def _exec_binary(name: str) -> None:
    """Replace the current process with the named Aether binary."""
    binary = _find_binary(name)

    # Make the binary executable on Unix (wheels may strip permissions)
    if sys.platform != "win32":
        os.chmod(binary, 0o755)

    # Pass through all CLI arguments
    args = [str(binary)] + sys.argv[1:]

    if sys.platform == "win32":
        # Windows: subprocess (no execvp)
        import subprocess

        result = subprocess.run(args)
        sys.exit(result.returncode)
    else:
        # Unix: replace process
        os.execvp(str(binary), args)


def main() -> None:
    """Entry point for 'aether' command."""
    _exec_binary("aether")


def main_mcp() -> None:
    """Entry point for 'aether-mcp' command."""
    _exec_binary("aether-mcp")


def main_api() -> None:
    """Entry point for 'aether-api' command."""
    _exec_binary("aether-api")

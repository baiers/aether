"""
Build platform-specific wheels from pre-built native binaries.

Usage (from CI):
    python python/scripts/build_wheels.py artifacts/

This script:
1. Takes the path to downloaded CI artifacts (one dir per target)
2. For each target, copies the binaries into aether_kernel/bin/
3. Builds a platform-specific wheel with the correct platform tag

The resulting wheels are placed in python/dist/.
"""

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

# Map Rust targets to Python wheel platform tags
TARGET_MAP = {
    "x86_64-pc-windows-msvc": {
        "plat": "win_amd64",
        "ext": ".exe",
        "archive": "zip",
    },
    "x86_64-unknown-linux-gnu": {
        "plat": "manylinux_2_17_x86_64",
        "ext": "",
        "archive": "tar.gz",
    },
    "x86_64-apple-darwin": {
        "plat": "macosx_13_0_x86_64",
        "ext": "",
        "archive": "tar.gz",
    },
    "aarch64-apple-darwin": {
        "plat": "macosx_14_0_arm64",
        "ext": "",
        "archive": "tar.gz",
    },
}

BINARIES = ["aether", "aether-mcp", "aether-api"]


def extract_binaries(artifact_dir: Path, target: str, dest: Path) -> bool:
    """Extract binaries from a CI artifact archive into dest."""
    info = TARGET_MAP[target]
    ext = info["ext"]
    archive_ext = info["archive"]

    artifact_name = f"aether-{target}"
    archive_path = artifact_dir / artifact_name

    # Look for the archive file
    if archive_ext == "zip":
        archive_file = archive_path / f"{artifact_name}.zip"
        if not archive_file.exists():
            archive_file = artifact_dir / f"{artifact_name}.zip"
        if archive_file.exists():
            import zipfile
            with zipfile.ZipFile(archive_file) as zf:
                zf.extractall(dest)
            return True
    else:
        archive_file = archive_path / f"{artifact_name}.tar.gz"
        if not archive_file.exists():
            archive_file = artifact_dir / f"{artifact_name}.tar.gz"
        if archive_file.exists():
            import tarfile
            with tarfile.open(archive_file) as tf:
                tf.extractall(dest)
            return True

    # Try direct binary files (no archive)
    found = 0
    for binary in BINARIES:
        src = archive_path / f"{binary}{ext}"
        if src.exists():
            shutil.copy2(src, dest / f"{binary}{ext}")
            found += 1
    return found > 0


def build_wheel(target: str, artifact_dir: Path, python_dir: Path) -> Path | None:
    """Build a platform-specific wheel for one target."""
    info = TARGET_MAP[target]
    plat = info["plat"]
    ext = info["ext"]
    bin_dir = python_dir / "aether_kernel" / "bin"

    # Clean bin directory
    for f in bin_dir.iterdir():
        if f.name != ".gitkeep":
            f.unlink()

    # Extract binaries
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        if not extract_binaries(artifact_dir, target, tmp_path):
            print(f"  SKIP {target}: no artifacts found")
            return None

        for binary in BINARIES:
            src = tmp_path / f"{binary}{ext}"
            if src.exists():
                shutil.copy2(src, bin_dir / f"{binary}{ext}")
                if ext == "":
                    os.chmod(bin_dir / binary, 0o755)
            else:
                print(f"  WARN: {binary}{ext} not found for {target}")

    # Build the wheel
    dist_dir = python_dir / "dist"
    dist_dir.mkdir(exist_ok=True)

    subprocess.run(
        [
            sys.executable, "-m", "build",
            "--wheel",
            "--outdir", str(dist_dir),
            str(python_dir),
        ],
        check=True,
    )

    # Rename wheel with correct platform tag
    for whl in dist_dir.glob("*.whl"):
        if "none-any" in whl.name:
            new_name = whl.name.replace("none-any", f"none-{plat}")
            new_path = whl.parent / new_name
            whl.rename(new_path)
            print(f"  OK: {new_name}")
            return new_path

    return None


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <artifacts-dir>")
        sys.exit(1)

    artifact_dir = Path(sys.argv[1]).resolve()
    python_dir = Path(__file__).resolve().parent.parent

    print(f"Artifact dir: {artifact_dir}")
    print(f"Python dir:   {python_dir}")
    print()

    for target in TARGET_MAP:
        print(f"Building wheel for {target}...")
        result = build_wheel(target, artifact_dir, python_dir)
        if result:
            print(f"  -> {result.name}")
        print()

    print("Done. Wheels in python/dist/:")
    dist_dir = python_dir / "dist"
    if dist_dir.exists():
        for whl in sorted(dist_dir.glob("*.whl")):
            print(f"  {whl.name}")


if __name__ == "__main__":
    main()

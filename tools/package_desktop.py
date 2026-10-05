#!/usr/bin/env python3
"""Build an auditable desktop directory from an existing binary; never build it."""

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def linux_linkage(binary, profile, target):
    if "linux" not in target:
        return {
            "checked": False,
            "reason": "Non-Linux target; ELF checks do not apply.",
        }
    command = shutil.which("readelf")
    if not command:
        if profile == "release":
            raise ValueError("Linux release packaging requires readelf.")
        return {"checked": False, "reason": "readelf unavailable; debug package only."}
    result = subprocess.run(
        [command, "-l", "-d", str(binary)], capture_output=True, text=True
    )
    if result.returncode:
        if profile == "release":
            raise ValueError(
                "Linux release binary failed readelf validation: "
                + result.stderr.strip()
            )
        return {
            "checked": False,
            "reason": "readelf rejected input; debug package only.",
        }
    report = result.stdout
    nix_lines = [line.strip() for line in report.splitlines() if "/nix/" in line]
    if nix_lines and profile == "release":
        raise ValueError(
            "Linux release has absolute Nix dependencies: " + "; ".join(nix_lines)
        )
    return {
        "checked": True,
        "nix_dependencies": nix_lines,
        "interpreter": re.findall(r"Requesting program interpreter: ([^\]]+)", report),
        "needed": re.findall(r"\(NEEDED\).*?\[([^\]]+)\]", report),
        "search_paths": re.findall(r"\((?:RUNPATH|RPATH)\).*?\[([^\]]+)\]", report),
    }


def reachable_packages(metadata):
    packages = {package["id"]: package for package in metadata["packages"]}
    resolve = metadata.get("resolve")
    root_id = resolve.get("root") if resolve else None
    if not root_id:
        root_id = next(
            (
                key
                for key, package in packages.items()
                if Path(package["manifest_path"]).resolve() == ROOT / "Cargo.toml"
            ),
            None,
        )
    if root_id not in packages:
        raise ValueError("Cargo metadata must identify the root project package.")
    if resolve:
        graph = {
            node["id"]: [
                edge["pkg"]
                for edge in node.get("deps", [])
                if any(kind.get("kind") != "dev" for kind in edge.get("dep_kinds", []))
            ]
            for node in resolve["nodes"]
        }
        pending, visited = [root_id], set()
        while pending:
            package_id = pending.pop()
            if package_id not in visited:
                visited.add(package_id)
                pending.extend(graph.get(package_id, []))
        selected = visited - {root_id}
        scope = "Normal/build dependencies reachable in the supplied Cargo resolve graph; dev-only edges excluded. Target filtering follows supplied metadata."
    else:
        selected = set(packages) - {root_id}
        scope = "All supplied packages; Cargo metadata has no resolve graph, so dependency completeness is unverified."
    return (
        packages[root_id],
        [packages[key] for key in sorted(selected) if key in packages],
        scope,
    )


def collect_licenses(packages, destination):
    retained, missing = [], []
    overrides_path = ROOT / "licenses/rust-overrides.json"
    overrides = (
        json.loads(overrides_path.read_text()).get("packages", [])
        if overrides_path.exists()
        else []
    )
    by_package = {(row["name"], row["version"]): row for row in overrides}
    for package in packages:
        directory = Path(package["manifest_path"]).parent
        candidates = {
            path
            for path in directory.iterdir()
            if path.is_file()
            and re.match(
                r"^(licen[cs]e|copying|notice|copyright)([._-]|$)", path.name, re.I
            )
        }
        explicit = package.get("license_file")
        if explicit:
            path = Path(explicit)
            path = path if path.is_absolute() else directory / path
            if path.is_file():
                candidates.add(path)
        files = []
        folder = destination / (package["name"] + "-" + package["version"])
        for source in sorted(candidates):
            folder.mkdir(parents=True, exist_ok=True)
            target = folder / source.name
            if target.exists() and sha256(target) != sha256(source):
                raise ValueError("Conflicting license filenames for " + package["id"])
            shutil.copy2(source, target)
            files.append(
                {
                    "file": target.relative_to(destination.parent).as_posix(),
                    "sha256": sha256(target),
                }
            )
        row = {
            key: package.get(key)
            for key in ["name", "version", "license", "repository"]
        }
        override = by_package.get((package["name"], package["version"]))
        blocked = override is not None and override["status"] != "downloaded"
        if blocked:
            row["retained_source"] = {
                key: override.get(key)
                for key in ["commit", "status", "blocked_reason", "tree_url"]
            }
            missing.append(
                {
                    "name": package["name"],
                    "version": package["version"],
                    "license": package.get("license"),
                    "status": "blocked",
                    "reason": override.get("blocked_reason"),
                }
            )
        if override and not blocked:
            if package.get("license") and override["spdx"] != package["license"]:
                raise ValueError(
                    "License override SPDX differs from Cargo metadata: "
                    + package["id"]
                )
            for evidence in override["files"]:
                source = (ROOT / evidence["path"]).resolve()
                if (
                    not source.is_relative_to((ROOT / "licenses/rust").resolve())
                    or not source.is_file()
                ):
                    raise ValueError(
                        "Invalid retained license path: " + evidence["path"]
                    )
                if sha256(source) != evidence["sha256"]:
                    raise ValueError(
                        "Retained license SHA256 mismatch: " + evidence["path"]
                    )
                folder.mkdir(parents=True, exist_ok=True)
                target = folder / source.name
                if target.exists() and sha256(target) != evidence["sha256"]:
                    raise ValueError("Conflicting retained license: " + package["id"])
                shutil.copy2(source, target)
                relative = target.relative_to(destination.parent).as_posix()
                if not any(entry["file"] == relative for entry in files):
                    files.append(
                        {
                            "file": relative,
                            "sha256": sha256(target),
                            "source_url": evidence["source_url"],
                            "role": evidence.get("role", "upstream_license_file"),
                        }
                    )
            row["retained_source"] = {
                key: override.get(key)
                for key in ["commit", "repository", "path_in_vcs", "spdx", "status", "selected_spdx", "license_text_origin", "upstream_text_note"]
            }
        row.update(package_id=package["id"], retained_files=files)
        retained.append(row)
        if not files and not blocked:
            missing.append(
                {
                    "name": package["name"],
                    "version": package["version"],
                    "license": package.get("license"),
                }
            )
    return retained, missing


def package(binary, metadata_path, output, profile, target, allow_missing=False):
    if allow_missing and profile != "debug":
        raise ValueError(
            "Missing-license exception is permitted only for explicit debug packages."
        )
    if output.exists() or output.is_symlink():
        raise ValueError("Output already exists; choose a new output directory.")
    if not binary.is_file():
        raise ValueError("Binary is not a file.")
    assets = ROOT / "assets"
    readme = ROOT / "docs/DESKTOP_PLAYER_README.md"
    if not assets.is_dir() or not readme.is_file():
        raise ValueError(
            "Complete assets/ and docs/DESKTOP_PLAYER_README.md are required."
        )
    if not target.strip():
        raise ValueError("Explicit target must not be empty.")
    metadata = json.loads(metadata_path.read_text())
    if profile == "release" and not metadata.get("resolve"):
        raise ValueError("Release packaging requires the complete Cargo resolve graph.")
    root, dependencies, scope = reachable_packages(metadata)
    linkage = linux_linkage(binary, profile, target)
    commit = subprocess.run(
        ["git", "-C", str(ROOT), "rev-parse", "HEAD"], capture_output=True, text=True
    )
    version = {
        "version": root["version"],
        "git_commit": commit.stdout.strip() if commit.returncode == 0 else None,
        "profile": profile,
        "target": target,
        "binary_sha256": sha256(binary),
        "release_package": profile == "release",
        "linux_linkage": linkage,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix="." + output.name + "-", dir=output.parent))
    try:
        executable = "rogue_commander.exe" if "windows" in target else "rogue_commander"
        shutil.copy2(binary, staging / executable)
        shutil.copytree(assets, staging / "assets")
        shutil.copy2(readme, staging / "README.txt")
        if profile == "debug":
            content = (staging / "README.txt").read_text()
            (staging / "README.txt").write_text(
                "DEBUG TEST PACKAGE — NOT A STANDALONE RELEASE.\n"
                "Development runtime dependencies may still be required.\n\n" + content
            )
        licenses, missing = collect_licenses(dependencies, staging / "third-party")
        if missing and not allow_missing:
            raise ValueError("Missing license files: " + json.dumps(missing))
        notice = {
            "dependency_scope": scope,
            "packages": licenses,
            "missing_license_files": missing,
            "legal_review": "Not performed. Retained files and missing entries are evidence, not legal approval.",
        }
        (staging / "third-party-index.json").write_text(
            json.dumps(notice, indent=2, sort_keys=True) + "\n"
        )
        (staging / "version.json").write_text(
            json.dumps(version, indent=2, sort_keys=True) + "\n"
        )
        manifest = {"excluded_self": "package-manifest.json", "files": []}
        for path in sorted(staging.rglob("*")):
            if path.is_file():
                manifest["files"].append(
                    {
                        "file": path.relative_to(staging).as_posix(),
                        "bytes": path.stat().st_size,
                        "sha256": sha256(path),
                    }
                )
        (staging / "package-manifest.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n"
        )
        if output.exists() or output.is_symlink():
            raise ValueError(
                "Output appeared during packaging; refusing to overwrite it."
            )
        staging.rename(output)
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    print(
        json.dumps(
            {
                "output": str(output),
                "profile": profile,
                "files": len(manifest["files"]),
                "missing_license_files": missing,
                "legal_review": "Not performed.",
            }
        )
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--metadata", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--profile", choices=["release", "debug"], required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument(
        "--allow-missing-licenses",
        action="store_true",
        help="Debug only; preserve explicit missing-license evidence.",
    )
    args = parser.parse_args()
    try:
        package(
            args.binary.resolve(),
            args.metadata.resolve(),
            args.output.absolute(),
            args.profile,
            args.target,
            args.allow_missing_licenses,
        )
    except (ValueError, OSError, KeyError, json.JSONDecodeError) as error:
        parser.exit(2, f"Packaging failed: {error}\n")

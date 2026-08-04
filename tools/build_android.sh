#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
android_project="$project_root/android"
jni_output="$android_project/app/src/main/jniLibs"

: "${ANDROID_SDK_ROOT:?Set ANDROID_SDK_ROOT after accepting the Android SDK license}"
: "${ANDROID_NDK_ROOT:?Set ANDROID_NDK_ROOT to an installed Android NDK}"

for required_tool in cargo cargo-ndk gradle; do
    if ! command -v "$required_tool" >/dev/null 2>&1; then
        echo "Missing required tool: $required_tool" >&2
        exit 1
    fi
done

android_target_libdir="$(rustc --print target-libdir --target aarch64-linux-android 2>/dev/null || true)"
if [[ -z "$android_target_libdir" ]] \
    || [[ ! -d "$android_target_libdir" ]] \
    || [[ -z "$(find "$android_target_libdir" -maxdepth 1 -name 'libstd-*' -print -quit)" ]]; then
    echo "Rust standard library for aarch64-linux-android is not installed" >&2
    exit 1
fi

cd "$project_root"
cargo ndk -t arm64-v8a -P 26 -o "$jni_output" build
gradle -p "$android_project" assembleDebug

apk_path="$android_project/app/build/outputs/apk/debug/app-debug.apk"
if [[ ! -f "$apk_path" ]]; then
    echo "Gradle completed but no APK was found at $apk_path" >&2
    exit 1
fi

echo "APK: $apk_path"

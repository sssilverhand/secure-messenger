#!/bin/bash
# Build script for PrivMsg Client

set -e

echo "==================================="
echo "Building PrivMsg Client"
echo "==================================="

cd "$(dirname "$0")/../client"

# Check for Flutter
if ! command -v flutter &> /dev/null; then
    echo "Error: Flutter is not installed"
    echo "Install from: https://flutter.dev/docs/get-started/install"
    exit 1
fi

# Get dependencies
echo "Getting dependencies..."
flutter pub get

# Generate code
echo "Generating code..."
flutter pub run build_runner build --delete-conflicting-outputs || true

# Build target
BUILD_TARGET="${1:-all}"

mkdir -p ../dist/client

case "$BUILD_TARGET" in
    "android")
        echo "Building Android APK..."
        flutter build apk --release
        cp build/app/outputs/flutter-apk/app-release.apk ../dist/client/privmsg-android.apk
        echo "Android APK: dist/client/privmsg-android.apk"
        ;;

    "android-bundle")
        echo "Building Android App Bundle..."
        flutter build appbundle --release
        cp build/app/outputs/bundle/release/app-release.aab ../dist/client/privmsg-android.aab
        echo "Android AAB: dist/client/privmsg-android.aab"
        ;;

    "windows")
        echo "Building Windows..."
        flutter build windows --release
        cp -r build/windows/x64/runner/Release ../dist/client/privmsg-windows
        echo "Windows: dist/client/privmsg-windows/"
        ;;

    "linux")
        echo "Building Linux..."
        flutter build linux --release
        cp -r build/linux/x64/release/bundle ../dist/client/privmsg-linux
        echo "Linux: dist/client/privmsg-linux/"
        ;;

    "all")
        echo "Building all platforms..."

        # Android
        echo ""
        echo "--- Building Android ---"
        flutter build apk --release || echo "Android build failed (may need Android SDK)"
        if [ -f build/app/outputs/flutter-apk/app-release.apk ]; then
            cp build/app/outputs/flutter-apk/app-release.apk ../dist/client/privmsg-android.apk
        fi

        # Windows (only on Windows)
        if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
            echo ""
            echo "--- Building Windows ---"
            flutter build windows --release || echo "Windows build failed"
            if [ -d build/windows/x64/runner/Release ]; then
                cp -r build/windows/x64/runner/Release ../dist/client/privmsg-windows
            fi
        fi

        # Linux (only on Linux)
        if [[ "$OSTYPE" == "linux-gnu"* ]]; then
            echo ""
            echo "--- Building Linux ---"
            flutter build linux --release || echo "Linux build failed"
            if [ -d build/linux/x64/release/bundle ]; then
                cp -r build/linux/x64/release/bundle ../dist/client/privmsg-linux
            fi
        fi
        ;;

    *)
        echo "Unknown target: $BUILD_TARGET"
        echo "Usage: $0 [android|android-bundle|windows|linux|all]"
        exit 1
        ;;
esac

echo ""
echo "Build complete!"
echo "Output directory: dist/client/"

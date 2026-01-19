#!/bin/bash
#
# Build PrivMsg Android APK without Android Studio
# Uses command-line tools only
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ANDROID_DIR="$PROJECT_DIR/android"
OUTPUT_DIR="$PROJECT_DIR/build/android"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}=== PrivMsg Android Build Script ===${NC}"
echo "Building APK without Android Studio"

# Check Java
if ! command -v java &> /dev/null; then
    echo -e "${RED}Error: Java is not installed${NC}"
    echo "Install OpenJDK 17: "
    echo "  Ubuntu/Debian: sudo apt install openjdk-17-jdk"
    echo "  Arch: sudo pacman -S jdk17-openjdk"
    echo "  macOS: brew install openjdk@17"
    exit 1
fi

JAVA_VERSION=$(java -version 2>&1 | head -1 | cut -d'"' -f2 | cut -d'.' -f1)
echo -e "${YELLOW}Java version: $JAVA_VERSION${NC}"

if [ "$JAVA_VERSION" -lt 11 ]; then
    echo -e "${RED}Error: Java 11 or higher required${NC}"
    exit 1
fi

# Setup Android SDK if not present
ANDROID_SDK="$HOME/android-sdk"
if [ -n "$ANDROID_HOME" ]; then
    ANDROID_SDK="$ANDROID_HOME"
elif [ -n "$ANDROID_SDK_ROOT" ]; then
    ANDROID_SDK="$ANDROID_SDK_ROOT"
fi

if [ ! -d "$ANDROID_SDK/cmdline-tools" ] && [ ! -d "$ANDROID_SDK/tools" ]; then
    echo -e "${YELLOW}Android SDK not found. Installing...${NC}"

    mkdir -p "$ANDROID_SDK"
    cd "$ANDROID_SDK"

    # Download command-line tools
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        CMDLINE_URL="https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        CMDLINE_URL="https://dl.google.com/android/repository/commandlinetools-mac-11076708_latest.zip"
    fi

    echo "Downloading Android command-line tools..."
    curl -L -o cmdline-tools.zip "$CMDLINE_URL"
    unzip -q cmdline-tools.zip
    mkdir -p cmdline-tools/latest
    mv cmdline-tools/bin cmdline-tools/lib cmdline-tools/latest/ 2>/dev/null || true
    rm cmdline-tools.zip

    export ANDROID_HOME="$ANDROID_SDK"
    export ANDROID_SDK_ROOT="$ANDROID_SDK"
    export PATH="$ANDROID_SDK/cmdline-tools/latest/bin:$ANDROID_SDK/platform-tools:$PATH"

    # Accept licenses
    yes | sdkmanager --licenses 2>/dev/null || true

    # Install required components
    echo "Installing Android SDK components..."
    sdkmanager "platform-tools" "platforms;android-34" "build-tools;34.0.0"
fi

export ANDROID_HOME="$ANDROID_SDK"
export ANDROID_SDK_ROOT="$ANDROID_SDK"
export PATH="$ANDROID_SDK/cmdline-tools/latest/bin:$ANDROID_SDK/platform-tools:$ANDROID_SDK/build-tools/34.0.0:$PATH"

echo -e "${YELLOW}ANDROID_HOME: $ANDROID_HOME${NC}"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Go to Android project
cd "$ANDROID_DIR"

# Create local.properties if it doesn't exist
if [ ! -f "local.properties" ]; then
    echo "sdk.dir=$ANDROID_HOME" > local.properties
    echo -e "${YELLOW}Created local.properties${NC}"
fi

# Check if gradle-wrapper.jar exists, if not - download it
WRAPPER_JAR="$ANDROID_DIR/gradle/wrapper/gradle-wrapper.jar"
if [ ! -f "$WRAPPER_JAR" ]; then
    echo -e "${YELLOW}Downloading gradle-wrapper.jar...${NC}"

    # Create wrapper directory
    mkdir -p "$ANDROID_DIR/gradle/wrapper"

    # Download gradle-wrapper.jar from official source
    GRADLE_VERSION="8.4"
    WRAPPER_URL="https://github.com/gradle/gradle/raw/v${GRADLE_VERSION}/gradle/wrapper/gradle-wrapper.jar"

    curl -L -o "$WRAPPER_JAR" "$WRAPPER_URL" || {
        # Alternative: use gradle to generate wrapper
        echo -e "${YELLOW}Trying alternative method...${NC}"
        if command -v gradle &> /dev/null; then
            cd "$ANDROID_DIR"
            gradle wrapper --gradle-version $GRADLE_VERSION
        else
            # Download and use gradle distribution
            echo "Downloading Gradle..."
            GRADLE_DIST_URL="https://services.gradle.org/distributions/gradle-${GRADLE_VERSION}-bin.zip"
            GRADLE_TMP="/tmp/gradle-${GRADLE_VERSION}"

            curl -L -o /tmp/gradle.zip "$GRADLE_DIST_URL"
            unzip -q /tmp/gradle.zip -d /tmp

            cd "$ANDROID_DIR"
            /tmp/gradle-${GRADLE_VERSION}/bin/gradle wrapper --gradle-version $GRADLE_VERSION

            rm -rf /tmp/gradle.zip /tmp/gradle-${GRADLE_VERSION}
        fi
    }
fi

# Make gradlew executable
chmod +x ./gradlew

# Build APK
echo -e "${YELLOW}Building Android APK...${NC}"
./gradlew assembleRelease --no-daemon

# Find and copy APK
APK_PATH=$(find "$ANDROID_DIR" -name "*.apk" -path "*release*" | head -1)
if [ -n "$APK_PATH" ]; then
    cp "$APK_PATH" "$OUTPUT_DIR/privmsg-android.apk"
    echo -e "${GREEN}Build complete!${NC}"
    echo "APK: $OUTPUT_DIR/privmsg-android.apk"

    # Show APK size
    APK_SIZE=$(du -h "$OUTPUT_DIR/privmsg-android.apk" | cut -f1)
    echo "Size: $APK_SIZE"
else
    echo -e "${RED}APK not found!${NC}"
    exit 1
fi

echo -e "${GREEN}=== Build Complete ===${NC}"

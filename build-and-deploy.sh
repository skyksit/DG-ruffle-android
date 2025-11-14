#!/bin/bash

# Flash Player (Ruffle) Rust 라이브러리 빌드 및 Android 앱 배포 스크립트
# 작성일: 2025-01-XX
# 목적: 멀티터치 지원 Rust 네이티브 라이브러리 빌드 및 Android 앱 자동 배포

set -e  # 에러 발생 시 즉시 종료

# 색상 정의
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 경로 설정
RUST_PROJECT_DIR="/Users/skyksit/studyspace/ruffle-android"
ANDROID_PROJECT_DIR="/Users/skyksit/studyspace/dgplayer-app/dsam3"
JNI_OUTPUT_DIR="${ANDROID_PROJECT_DIR}/app/src/main/jniLibs"

# NDK 경로 확인
if [ -z "$ANDROID_NDK_HOME" ]; then
    ANDROID_NDK_HOME="/Users/skyksit/Library/Android/sdk/ndk/27.2.12479018"
    echo -e "${YELLOW}⚠️  ANDROID_NDK_HOME not set, using default: $ANDROID_NDK_HOME${NC}"
fi

if [ ! -d "$ANDROID_NDK_HOME" ]; then
    echo -e "${RED}❌ NDK not found at: $ANDROID_NDK_HOME${NC}"
    echo -e "${YELLOW}Please install Android NDK or set ANDROID_NDK_HOME${NC}"
    exit 1
fi

export ANDROID_NDK_HOME

# 함수: 단계별 로그
print_step() {
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}\n"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Rust 도구 확인
print_step "Step 1: Checking Rust toolchain"

if ! command -v cargo &> /dev/null; then
    print_error "Cargo not found. Please install Rust: https://rustup.rs/"
    exit 1
fi

if ! command -v cargo-ndk &> /dev/null; then
    print_warning "cargo-ndk not found. Installing..."
    cargo install cargo-ndk
fi

print_success "Rust toolchain OK"

# Android 타겟 확인
print_step "Step 2: Checking Android targets"

TARGETS=("aarch64-linux-android" "armv7-linux-androideabi" "i686-linux-android" "x86_64-linux-android")

for target in "${TARGETS[@]}"; do
    if ! rustup target list | grep -q "$target (installed)"; then
        print_warning "Target $target not installed. Installing..."
        rustup target add "$target"
    fi
done

print_success "All Android targets installed"

# Rust 프로젝트 빌드
print_step "Step 3: Building Rust library"

cd "$RUST_PROJECT_DIR"

# 이전 빌드 정리 (선택)
# print_warning "Cleaning previous build..."
# cargo clean

print_warning "Building for all Android architectures (this may take 5-10 minutes)..."

cargo ndk \
    -t arm64-v8a \
    -t armeabi-v7a \
    -t x86 \
    -t x86_64 \
    -o "$JNI_OUTPUT_DIR" \
    build --release

if [ $? -ne 0 ]; then
    print_error "Rust build failed!"
    exit 1
fi

print_success "Rust library built successfully"

# SO 파일 확인
print_step "Step 4: Verifying SO files"

SO_FILE="libruffle_android.so"
ARCH_DIRS=("arm64-v8a" "armeabi-v7a" "x86" "x86_64")

for arch in "${ARCH_DIRS[@]}"; do
    SO_PATH="${JNI_OUTPUT_DIR}/${arch}/${SO_FILE}"
    if [ -f "$SO_PATH" ]; then
        SIZE=$(du -h "$SO_PATH" | cut -f1)
        print_success "✓ $arch: $SIZE"
    else
        print_error "Missing: $arch/$SO_FILE"
        exit 1
    fi
done

# Android 프로젝트 빌드
print_step "Step 5: Building Android app"

cd "$ANDROID_PROJECT_DIR"

print_warning "Running Gradle clean..."
./gradlew clean

print_warning "Building debug APK..."
./gradlew :app:assembleDebug

if [ $? -ne 0 ]; then
    print_error "Android build failed!"
    exit 1
fi

print_success "Android app built successfully"

# APK 위치 확인
APK_PATH="${ANDROID_PROJECT_DIR}/app/build/outputs/apk/debug/app-debug.apk"
if [ -f "$APK_PATH" ]; then
    APK_SIZE=$(du -h "$APK_PATH" | cut -f1)
    print_success "APK created: $APK_SIZE"
    echo -e "${GREEN}   Location: $APK_PATH${NC}"
else
    print_error "APK not found at: $APK_PATH"
    exit 1
fi

# 기기 연결 확인 및 설치
print_step "Step 6: Deploying to device"

if command -v adb &> /dev/null; then
    DEVICES=$(adb devices | grep -w "device" | wc -l)
    
    if [ "$DEVICES" -gt 0 ]; then
        print_warning "Installing APK to device..."
        ./gradlew :app:installDebug
        
        if [ $? -eq 0 ]; then
            print_success "App installed successfully!"
        else
            print_warning "Installation failed. Install manually:"
            echo -e "${YELLOW}   adb install -r $APK_PATH${NC}"
        fi
    else
        print_warning "No Android device connected. Skipping installation."
        echo -e "${YELLOW}To install manually:${NC}"
        echo -e "${YELLOW}   adb install -r $APK_PATH${NC}"
    fi
else
    print_warning "adb not found. Skipping device installation."
fi

# 완료 요약
print_step "🎉 Build Complete!"

echo -e "${GREEN}✅ Rust library: $JNI_OUTPUT_DIR${NC}"
echo -e "${GREEN}✅ Android APK: $APK_PATH${NC}"
echo ""
echo -e "${BLUE}📝 Next steps:${NC}"
echo -e "   1. Install APK on device: ${YELLOW}adb install -r $APK_PATH${NC}"
echo -e "   2. Launch Flash player app"
echo -e "   3. Test multi-touch:"
echo -e "      - VPad click + game screen swipe"
echo -e "      - Game screen swipe + VPad click"
echo -e "      - 3+ simultaneous touches"
echo ""
echo -e "${BLUE}🐛 Debugging:${NC}"
echo -e "   View logs: ${YELLOW}adb logcat | grep -E 'PlayerActivity|Ruffle'${NC}"
echo ""

exit 0


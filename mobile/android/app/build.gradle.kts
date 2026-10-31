import org.gradle.internal.os.OperatingSystem

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

// The repository root, where the Cargo workspace and the sample games live.
val repositoryRoot: File = rootDir.resolve("../..").canonicalFile

// The NDK the Rust library links against. Pinned so a build on another machine
// links the same way; `sdkmanager "ndk;<version>"` installs it.
val ndkVersionPinned = "29.0.14206865"

// The lowest Android the app runs on (Android 10). The Rust library is linked
// against the same API level, so the two cannot disagree.
val minimumApi = 29

android {
    namespace = "dev.sindri.mobile"
    compileSdk = 36
    ndkVersion = ndkVersionPinned

    defaultConfig {
        applicationId = "dev.sindri.mobile"
        minSdk = minimumApi
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        // The reference device is a Pixel 8, which is arm64 only; the Rust
        // library is built for that ABI alone.
        ndk { abiFilters += "arm64-v8a" }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // Signed with the debug key until a release key is decided; this
            // build is for the slice's own phone, not for distribution.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures { compose = true }

    sourceSets {
        getByName("main") {
            jniLibs.srcDir(layout.buildDirectory.dir("rustJniLibs"))
            assets.srcDir(layout.buildDirectory.dir("bundledProjects"))
        }
    }

    packaging {
        // Scenes, scripts and sheets are read whole on first launch; storing
        // them uncompressed costs little and keeps the copy simple.
        resources.excludes += "/META-INF/{AL2.0,LGPL2.1}"
    }
}

// Java 17 bytecode, compiled by whatever JDK 17 or newer runs Gradle.
kotlin {
    compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2025.12.01")
    implementation(composeBom)
    implementation("androidx.core:core-ktx:1.17.0")
    implementation("androidx.activity:activity-compose:1.12.4")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.9.4")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
}

// The sample project the slice opens, copied into the APK's assets under
// `projects/lamplighter`: its manifest and `assets/`, which is everything
// opening a project reads.
val bundleProjects by tasks.registering(Sync::class) {
    from(repositoryRoot.resolve("games/lamplighter")) {
        include("sindri.toml", "assets/**")
        into("projects/lamplighter")
    }
    into(layout.buildDirectory.dir("bundledProjects"))
}

// `-PsindriRustProfile=dev` builds the library unoptimized, which compiles
// faster and runs much slower. The default is the optimized build a phone needs.
val rustProfile = (findProperty("sindriRustProfile") as String?) ?: "release"

// Builds `libsindri_mobile.so` for arm64 with the pinned NDK's clang as linker.
val buildRustLibrary by tasks.registering(Exec::class) {
    val sdkDirectory = android.sdkDirectory
    val ndkDirectory = sdkDirectory.resolve("ndk/$ndkVersionPinned")
    val host = when {
        OperatingSystem.current().isMacOsX -> "darwin-x86_64"
        OperatingSystem.current().isLinux -> "linux-x86_64"
        else -> "windows-x86_64"
    }
    val tools = ndkDirectory.resolve("toolchains/llvm/prebuilt/$host/bin")
    val clang = tools.resolve("aarch64-linux-android$minimumApi-clang")
    val target = "aarch64-linux-android"
    val targetEnv = target.replace('-', '_')

    workingDir = repositoryRoot
    environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", clang.path)
    environment("CC_$targetEnv", clang.path)
    environment("AR_$targetEnv", tools.resolve("llvm-ar").path)
    commandLine(
        "cargo", "build",
        "--package", "sindri-mobile",
        "--lib",
        "--target", target,
        "--profile", rustProfile,
    )

    val outputDirectory = if (rustProfile == "dev") "debug" else rustProfile
    val built = repositoryRoot.resolve("target/$target/$outputDirectory/libsindri_mobile.so")
    val copied = layout.buildDirectory.file("rustJniLibs/arm64-v8a/libsindri_mobile.so")
    outputs.file(copied)
    // Cargo decides what is stale; Gradle always asks it.
    outputs.upToDateWhen { false }
    doFirst {
        check(ndkDirectory.isDirectory) {
            "NDK $ndkVersionPinned is not installed in $sdkDirectory; " +
                "run: sdkmanager \"ndk;$ndkVersionPinned\""
        }
    }
    doLast {
        val destination = copied.get().asFile
        destination.parentFile.mkdirs()
        built.copyTo(destination, overwrite = true)
    }
}

tasks.named("preBuild") {
    dependsOn(bundleProjects, buildRustLibrary)
}

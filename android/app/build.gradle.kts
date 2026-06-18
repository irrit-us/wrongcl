plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

val appRootDir = rootProject.projectDir.parentFile.parentFile
val rustBuildScript = appRootDir.resolve("scripts/build-android-rust.sh")

android {
    namespace = "us.irrit.wrongcl"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "us.irrit.wrongcl"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    buildTypes {
        release {
            // TODO: Add your own signing config for the release build.
            // Signing with the debug keys for now, so `flutter run --release` works.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDir("../../build/android/jniLibs")
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}

fun registerRustBuildTask(name: String, profile: String) =
    tasks.register<Exec>(name) {
        workingDir = appRootDir
        commandLine("bash", rustBuildScript.absolutePath, profile)
    }

val buildWrongclNativeDebug = registerRustBuildTask(
    "buildWrongclNativeDebug",
    "debug",
)
val buildWrongclNativeProfile = registerRustBuildTask(
    "buildWrongclNativeProfile",
    "release",
)
val buildWrongclNativeRelease = registerRustBuildTask(
    "buildWrongclNativeRelease",
    "release",
)

tasks.matching { it.name == "preDebugBuild" }.configureEach {
    dependsOn(buildWrongclNativeDebug)
}
tasks.matching { it.name == "preProfileBuild" }.configureEach {
    dependsOn(buildWrongclNativeProfile)
}
tasks.matching { it.name == "preReleaseBuild" }.configureEach {
    dependsOn(buildWrongclNativeRelease)
}

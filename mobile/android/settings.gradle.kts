// Sindri Mobile's Android app. The native half is the `sindri-mobile` crate in
// `mobile/native`, built by the app's `buildRustLibrary` task.
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "sindri-mobile"
include(":app")

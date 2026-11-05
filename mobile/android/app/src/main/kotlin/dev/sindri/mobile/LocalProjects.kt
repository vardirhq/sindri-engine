package dev.sindri.mobile

import java.io.File
import java.util.Properties
import java.util.UUID

/** Projects created on the device. Creation never overwrites an existing project. */
object LocalProjects {
    data class Project(val id: String, val name: String, val directory: File)

    fun list(base: File): List<Project> = base.listFiles().orEmpty().mapNotNull { directory ->
        val metadata = File(directory, ".mobile-project")
        if (!directory.isDirectory || !metadata.isFile || !File(directory, "sindri.toml").isFile) return@mapNotNull null
        runCatching {
            val properties = Properties().also { values -> metadata.inputStream().use { values.load(it) } }
            Project(directory.name, properties.getProperty("name") ?: return@mapNotNull null, directory)
        }.getOrNull()
    }.sortedBy { it.name.lowercase() }

    fun create(base: File, title: String): Project {
        val name = title.trim()
        require(name.isNotEmpty() && name.length <= 100 && name.none { it.isISOControl() }) { "Enter a game name of up to 100 characters." }
        require(base.isDirectory || base.mkdirs()) { "Could not create the projects directory." }
        val id = UUID.randomUUID().toString()
        val temporary = File(base, ".creating-$id")
        val destination = File(base, id)
        require(temporary.mkdir()) { "Could not create a project directory." }
        try {
            check(File(temporary, "assets").mkdir()) { "Could not create the assets directory." }
            val escaped = name.replace("\\", "\\\\").replace("\"", "\\\"")
            File(temporary, "sindri.toml").writeText("format_version = 1\n\n[project]\nname = \"$escaped\"\nmain_scene = \"assets/world.scene\"\n")
            File(temporary, "assets/world.scene").writeText(SCENE)
            Properties().apply { setProperty("name", name) }.let { metadata -> File(temporary, ".mobile-project").outputStream().use { metadata.store(it, null) } }
            check(!destination.exists() && temporary.renameTo(destination)) { "Could not finish creating the project." }
            return Project(id, name, destination)
        } finally {
            if (temporary.exists()) temporary.deleteRecursively()
        }
    }

    private const val SCENE = """{
  "format_version": 10,
  "metadata": { "name": "World" },
  "entities": [{
    "id": "camera", "name": "Camera",
    "transform_3d": { "position": [0, 0, 10] },
    "components": { "sindri.camera": { "projection": "orthographic", "vertical_size": 16, "near": 0.1, "far": 100, "fit": "height" } }
  }]
}
"""
}

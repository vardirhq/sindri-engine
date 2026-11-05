package dev.sindri.mobile

import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class LocalProjectsTest {
    @Test fun projects_are_created_separately_and_listed_again() {
        val base = Files.createTempDirectory("sindri-projects").toFile()
        try {
            val first = LocalProjects.create(base, "My \"Game\"")
            val second = LocalProjects.create(base, "My \"Game\"")
            assertNotEquals(first.id, second.id)
            assertEquals(2, LocalProjects.list(base).size)
            assertTrue(first.directory.resolve("sindri.toml").readText().contains("My \\\"Game\\\""))
            assertTrue(first.directory.resolve("assets/world.scene").isFile)
        } finally { base.deleteRecursively() }
    }

    @Test fun invalid_names_do_not_create_a_project() {
        val base = Files.createTempDirectory("sindri-projects").toFile()
        try {
            assertTrue(runCatching { LocalProjects.create(base, "  ") }.isFailure)
            assertTrue(runCatching { LocalProjects.create(base, "line\nbreak") }.isFailure)
            assertTrue(LocalProjects.list(base).isEmpty())
        } finally { base.deleteRecursively() }
    }
}

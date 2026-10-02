package org.nativecod.app

import android.content.Context
import android.net.Uri
import android.provider.DocumentsContract
import android.system.Os
import android.system.OsConstants
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream
import java.security.MessageDigest

class SafStore(private val context: Context, tree: String) {
    private val root = Uri.parse(tree)
    private val resolver = context.contentResolver
    data class Entry(val path: String, val id: String, val directory: Boolean, val size: Long, val modified: Long)
    private var entries: List<Entry>? = null
    private fun document(id: String) = DocumentsContract.buildDocumentUriUsingTree(root, id)
    fun rootName(): String {
        resolver.query(document(DocumentsContract.getTreeDocumentId(root)), arrayOf(DocumentsContract.Document.COLUMN_DISPLAY_NAME), null, null, null).use {
            check(it != null && it.moveToFirst()) { "The selected folder is no longer accessible." }
            return it.getString(0)
        }
    }
    @Synchronized fun index(): List<Entry> {
        entries?.let { return it }
        val result = mutableListOf<Entry>()
        val queue = ArrayDeque<Pair<String, String>>()
        queue.add("" to DocumentsContract.getTreeDocumentId(root))
        val seen = mutableSetOf<String>()
        while (queue.isNotEmpty()) {
            val (parent, id) = queue.removeFirst()
            if (!seen.add(id)) continue
            val children = DocumentsContract.buildChildDocumentsUriUsingTree(root, id)
            resolver.query(children, arrayOf(DocumentsContract.Document.COLUMN_DOCUMENT_ID, DocumentsContract.Document.COLUMN_DISPLAY_NAME,
                DocumentsContract.Document.COLUMN_MIME_TYPE, DocumentsContract.Document.COLUMN_SIZE, DocumentsContract.Document.COLUMN_LAST_MODIFIED), null, null, null).use { cursor ->
                check(cursor != null) { "Cannot list the selected folder." }
                while (cursor.moveToNext()) {
                    val name = cursor.getString(1)
                    check(name != "." && name != ".." && !name.contains('/') && !name.contains('\\')) { "Invalid document name." }
                    val path = if (parent.isEmpty()) name else "$parent/$name"
                    val entry = Entry(path, cursor.getString(0), cursor.getString(2) == DocumentsContract.Document.MIME_TYPE_DIR,
                        if (cursor.isNull(3)) -1 else cursor.getLong(3), if (cursor.isNull(4)) 0 else cursor.getLong(4))
                    result.add(entry)
                    check(result.size <= 50000) { "Selected folder contains too many documents. Select the MW2 installation folder." }
                    if (entry.directory) { check(path.count { it == '/' } < 16) { "Selected folder is too deeply nested." }; queue.add(path to entry.id) }
                }
            }
        }
        entries = result
        return result
    }
    fun listJson(): String = JSONArray().apply {
        for (entry in index()) put(JSONObject().put("path", entry.path).put("directory", entry.directory).put("size", entry.size).put("modified", entry.modified))
    }.toString()
    private fun entry(path: String): Entry = index().firstOrNull { it.path == path } ?: error("Game data file was not found: $path")
    fun supportedHeader(path: String): Boolean {
        return try {
            resolver.openInputStream(document(entry(path).id)).use { input ->
                val bytes = ByteArray(21)
                var offset = 0
                while (offset < bytes.size) { val count = input?.read(bytes, offset, bytes.size - offset) ?: -1; if (count <= 0) return false; offset += count }
                val magic = String(bytes, 0, 8, Charsets.US_ASCII)
                (magic == "IWff0100" || magic == "IWffu100") && bytes[8] == 0x14.toByte() && bytes[9] == 1.toByte() && bytes[10] == 0.toByte() && bytes[11] == 0.toByte()
            }
        } catch (_: Exception) { false }
    }
    fun openFd(path: String): Int {
        val source = entry(path)
        check(!source.directory) { "Cannot open a directory as game data." }
        val descriptor = resolver.openFileDescriptor(document(source.id), "r") ?: error("Cannot open $path")
        try { Os.lseek(descriptor.fileDescriptor, 0, OsConstants.SEEK_SET); return descriptor.detachFd() }
        catch (_: android.system.ErrnoException) { descriptor.close() }
        val folder = File(context.cacheDir, "imports").apply { mkdirs() }
        val key = MessageDigest.getInstance("SHA-256").digest("$root|${source.id}|${source.size}|${source.modified}".toByteArray()).joinToString("") { "%02x".format(it) }
        val destination = File(folder, key)
        synchronized(this) {
            if (!destination.isFile || source.modified == 0L) {
                check(source.size <= 0 || folder.usableSpace > source.size + 64L * 1024 * 1024) { "Not enough space to cache $path." }
                val temporary = File.createTempFile(key, ".partial", folder)
                try {
                    resolver.openInputStream(document(source.id)).use { input ->
                        check(input != null) { "Cannot read $path." }
                        FileOutputStream(temporary).use { output -> input.copyTo(output); output.fd.sync() }
                    }
                    check(source.size < 0 || temporary.length() == source.size) { "Incomplete cache copy of $path." }
                    check(temporary.renameTo(destination)) { "Cannot publish cache for $path." }
                } finally { temporary.delete() }
            }
        }
        return android.os.ParcelFileDescriptor.open(destination, android.os.ParcelFileDescriptor.MODE_READ_ONLY).detachFd()
    }
}

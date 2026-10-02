package org.nativecod.app

import android.content.Context
import android.net.Uri
import org.json.JSONObject

class InstallationStore(private val context: Context) {
    private val prefs = context.getSharedPreferences("installation", Context.MODE_PRIVATE)
    fun tree(): Uri? = prefs.getString("tree", null)?.let(Uri::parse)
    fun select(uri: Uri) {
        prefs.edit().putString("tree", uri.toString()).remove("validation").commit()
    }
    fun status(): JSONObject {
        val uri = tree() ?: return JSONObject().put("configured", false).put("valid", false)
        val result = JSONObject(prefs.getString("validation", null) ?: "{}").put("configured", true)
        if (!context.contentResolver.persistedUriPermissions.any { it.uri == uri && it.isReadPermission }) {
            result.put("valid", false).put("error", "Folder permission was revoked. Select the installation again.")
        }
        if (!result.has("valid")) result.put("valid", false)
        val report = java.io.File(context.filesDir, "runtime-status.txt")
        if (report.isFile) result.put("runtimeStatus", report.readText().take(4000))
        return result
    }
    fun validate(): JSONObject {
        val uri = tree() ?: return status()
        val result = JSONObject().put("configured", true).put("valid", false)
        try {
            val source = SafStore(context, uri.toString())
            result.put("name", source.rootName())
            val files = source.index()
            val common = files.firstOrNull { it.path.substringAfterLast('/').equals("common_mp.ff", true) && source.supportedHeader(it.path) }
            if (common == null) {
                if (files.any { it.path.substringAfterLast('/').equals("common_mp.ff", true) }) error("common_mp.ff is truncated or uses an unsupported IW4 format.")
                error("common_mp.ff was not found. This folder does not contain the required MW2 Multiplayer data.")
            }
            val map = files.firstOrNull { it.path.substringAfterLast('/').equals("mp_boneyard.ff", true) }
            result.put("common", common.path)
            if (map != null) result.put("map", map.path)
            result.put("valid", true)
            result.put("error", if (map == null) "mp_boneyard.ff is missing; the first map proof requires it." else "")
        } catch (e: Exception) { result.put("error", e.message ?: e.javaClass.simpleName) }
        prefs.edit().putString("validation", result.toString()).commit()
        return result
    }
}

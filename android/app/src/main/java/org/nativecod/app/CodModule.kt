package org.nativecod.app

import android.app.Activity
import android.content.Intent
import android.net.Uri
import com.facebook.react.bridge.ActivityEventListener
import com.facebook.react.bridge.Promise
import com.facebook.react.bridge.ReactApplicationContext
import java.util.concurrent.Executors

class CodModule(private val context: ReactApplicationContext) : NativeCodSpec(context), ActivityEventListener {
    private val worker = Executors.newSingleThreadExecutor()
    private var selection: Promise? = null
    init { context.addActivityEventListener(this) }
    override fun getName() = "NativeCod"
    fun controllerAction(action: String) = emitOnControllerAction(action)
    private fun execute(promise: Promise, block: () -> Any?) {
        worker.execute {
            try { promise.resolve(block()) }
            catch (e: Exception) { promise.reject("NATIVE_COD", e.message ?: e.javaClass.simpleName, e) }
        }
    }
    override fun getInstallation(promise: Promise) = execute(promise) { InstallationStore(context).status().toString() }
    override fun validateInstallation(promise: Promise) = execute(promise) { InstallationStore(context).validate().toString() }
    override fun selectInstallation(promise: Promise) {
        context.runOnUiQueueThread {
            val activity = context.currentActivity
            if (activity == null) { promise.reject("NO_ACTIVITY", "Launcher is not active."); return@runOnUiQueueThread }
            if (selection != null) { promise.reject("PICKER_BUSY", "Folder selection is already open."); return@runOnUiQueueThread }
            selection = promise
            try {
                activity.startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION or Intent.FLAG_GRANT_PREFIX_URI_PERMISSION)
                }, 4201)
            } catch (e: Exception) { selection = null; promise.reject("PICKER_FAILED", e) }
        }
    }
    override fun onActivityResult(activity: Activity, requestCode: Int, resultCode: Int, data: Intent?) {
        if (requestCode != 4201) return
        val promise = selection ?: return
        selection = null
        execute(promise) {
            val store = InstallationStore(context)
            val uri = data?.data
            if (resultCode == Activity.RESULT_OK && uri != null) {
                context.contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
                store.select(uri)
                store.validate().toString()
            } else store.status().toString()
        }
    }
    override fun onNewIntent(intent: Intent) = Unit
    override fun startRuntime(diagnostic: Boolean, promise: Promise) {
        worker.execute {
            try {
                val store = InstallationStore(context)
                if (!diagnostic && !store.validate().getBoolean("valid")) error(store.status().optString("error", "Game files are not configured."))
                val tree = if (diagnostic) "" else store.tree()?.toString() ?: ""
                context.runOnUiQueueThread {
                    try {
                        val activity = context.currentActivity ?: error("Launcher is not active.")
                        activity.startActivity(Intent(activity, RuntimeActivity::class.java).apply {
                            putExtra("diagnostic", diagnostic)
                            putExtra("tree", tree)
                        })
                        promise.resolve(null)
                    } catch (e: Exception) { promise.reject("RUNTIME_LAUNCH", e) }
                }
            } catch (e: Exception) { promise.reject("RUNTIME_LAUNCH", e) }
        }
    }
    override fun invalidate() {
        context.removeActivityEventListener(this)
        selection?.reject("DESTROYED", "Launcher closed during folder selection.")
        selection = null
        worker.shutdown()
        super.invalidate()
    }
}

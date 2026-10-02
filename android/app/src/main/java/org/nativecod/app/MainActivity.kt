package org.nativecod.app

import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.view.Gravity
import android.view.WindowInsets
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {
    private val pick = registerForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
        if (uri == null) return@registerForActivityResult
        contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
        val store = InstallationStore(this)
        store.select(uri)
        showStatus("Checking game files…")
        Thread {
            val status = store.validate()
            runOnUiThread {
                if (status.optBoolean("valid")) startGame(store)
                else showPicker(status.optString("error", "That folder is not an MW2 installation."))
            }
        }.start()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        showStatus("Starting…")
        window.insetsController?.hide(WindowInsets.Type.systemBars())
        val store = InstallationStore(this)
        Thread {
            val ready = store.tree() != null && store.validate().optBoolean("valid")
            runOnUiThread {
                if (isFinishing) return@runOnUiThread
                if (ready) startGame(store) else showPicker(null)
            }
        }.start()
    }

    private fun startGame(store: InstallationStore) {
        startActivity(Intent(this, RuntimeActivity::class.java).apply {
            putExtra("diagnostic", false)
            putExtra("tree", store.tree()?.toString() ?: "")
        })
        finish()
    }

    private fun showStatus(text: String) {
        setContentView(TextView(this).apply {
            this.text = text
            setTextColor(Color.parseColor("#f2f5ef"))
            textSize = 22f
            gravity = Gravity.CENTER
            setBackgroundColor(Color.parseColor("#101513"))
        })
    }

    private fun showPicker(error: String?) {
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            setBackgroundColor(Color.parseColor("#101513"))
            setPadding(64, 64, 64, 64)
        }
        root.addView(TextView(this).apply {
            text = "IW4L"
            setTextColor(Color.parseColor("#f2f5ef"))
            textSize = 32f
        })
        root.addView(TextView(this).apply {
            text = error ?: "Select the Modern Warfare 2 folder."
            setTextColor(Color.parseColor(if (error == null) "#a7b1a9" else "#e07a7a"))
            textSize = 16f
            setPadding(0, 24, 0, 32)
        })
        val button = Button(this).apply {
            text = "Select game folder"
            setOnClickListener { pick.launch(null) }
        }
        root.addView(button)
        setContentView(root)
        button.requestFocus()
    }
}
